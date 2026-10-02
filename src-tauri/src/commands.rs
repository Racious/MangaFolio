//! Tauri IPC 指令入口。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use image::DynamicImage;
use serde::{Deserialize, Serialize};
use tauri::ipc::Response;
use tauri::State;

use crate::book::{self, Book};
use crate::cache::{Lru, RenderKey};
use crate::image_pipeline::{self, FitMode, ScaleSpec};
use crate::library::{self, Library, ReaderPreferences};

/// 解碼快取容量（已解碼影像張數，每張較占記憶體）。
const DECODE_CAPACITY: usize = 14;
/// 算繪快取容量（最終 PNG 張數，每張較小）。
const RENDER_CAPACITY: usize = 48;

type DecodeCache = Lru<(u64, usize), Arc<DynamicImage>>;
type RenderCache = Lru<RenderKey, Arc<Vec<u8>>>;
type BookSlot = Arc<Mutex<Option<(u64, Arc<Book>)>>>;

/// 背景預載任務：以世代號標記，過時者由工作執行緒略過或中止。
struct PreloadJob {
    generation: u64,
    req: RenderRequest,
}

/// 應用程式狀態：當前書 + 兩級快取 + 單一預載工作執行緒。
pub struct AppState {
    book: BookSlot,
    decode: Arc<Mutex<DecodeCache>>,
    render: Arc<Mutex<RenderCache>>,
    /// 最新預載世代號；工作執行緒據此中止過時任務。
    generation: Arc<AtomicU64>,
    next_session: AtomicU64,
    /// 送往預載工作執行緒的通道（Sender 非 Sync，故以 Mutex 包裝）。
    preload_tx: Mutex<Sender<PreloadJob>>,
}

impl Default for AppState {
    fn default() -> Self {
        let book: BookSlot = Arc::new(Mutex::new(None));
        let decode = Arc::new(Mutex::new(Lru::new(DECODE_CAPACITY)));
        let render = Arc::new(Mutex::new(Lru::new(RENDER_CAPACITY)));
        let generation = Arc::new(AtomicU64::new(0));
        let (tx, rx) = channel::<PreloadJob>();

        spawn_preload_worker(
            book.clone(),
            decode.clone(),
            render.clone(),
            generation.clone(),
            rx,
        );

        Self {
            book,
            decode,
            render,
            generation,
            next_session: AtomicU64::new(1),
            preload_tx: Mutex::new(tx),
        }
    }
}

/// 開啟書籍後回傳給前端的資訊。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookInfo {
    pub book_id: i64,
    pub session_id: u64,
    pub favorite: bool,
    pub preferences: ReaderPreferences,
    pub title: String,
    pub page_count: usize,
    pub pages: Vec<String>,
    pub start_index: usize,
}

/// Opening a local source also registers it in the persistent library.
#[tauri::command]
pub fn open_path(
    path: String,
    state: State<AppState>,
    library: State<Library>,
) -> Result<BookInfo, String> {
    let explicit_image = std::path::Path::new(&path).is_file() && book::is_image(&path);
    open_source(&path, !explicit_image, &state, &library)
}

#[tauri::command]
pub fn open_library_book(
    id: i64,
    state: State<AppState>,
    library: State<Library>,
) -> Result<BookInfo, String> {
    let entry = library.get(id)?;
    open_source(&entry.path, true, &state, &library)
}

fn open_source(
    path: &str,
    resume: bool,
    state: &AppState,
    library: &Library,
) -> Result<BookInfo, String> {
    let result = book::open(path)?;
    let saved = library.register(&result.book)?;
    let pages = result.book.page_names();
    let start_index = if resume {
        library::resume_index(&saved, &pages)
    } else {
        result.start_index
    };
    library.mark_opened(saved.id)?;
    let session_id = state.next_session.fetch_add(1, Ordering::SeqCst);
    let info = BookInfo {
        book_id: saved.id,
        session_id,
        favorite: saved.favorite,
        preferences: saved.preferences,
        title: result.book.title.clone(),
        page_count: result.book.len(),
        pages,
        start_index,
    };
    // Cache keys include the immutable reading session. Old preload work can
    // finish after a switch, but can never supply another book's pixels.
    state.generation.fetch_add(1, Ordering::SeqCst);
    *state.book.lock().unwrap() = Some((session_id, Arc::new(result.book)));
    state.decode.lock().unwrap().clear();
    state.render.lock().unwrap().clear();
    Ok(info)
}

/// 前端傳來的縮放請求。
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RenderRequest {
    pub session_id: u64,
    pub index: usize,
    /// "window" | "width" | "height" | "original" | "fixed"
    pub mode: String,
    pub viewport_w: u32,
    pub viewport_h: u32,
    pub fixed_scale: f32,
}

impl RenderRequest {
    fn key_for(&self, index: usize) -> RenderKey {
        RenderKey {
            session_id: self.session_id,
            index,
            mode: mode_code(&self.mode),
            viewport_w: self.viewport_w,
            viewport_h: self.viewport_h,
            scale_milli: (self.fixed_scale * 1000.0).round() as u32,
        }
    }

    fn spec(&self) -> ScaleSpec {
        ScaleSpec {
            mode: FitMode::parse(&self.mode),
            viewport_w: self.viewport_w,
            viewport_h: self.viewport_h,
            fixed_scale: self.fixed_scale,
        }
    }
}

/// 縮放模式 → 代碼，供算繪快取鍵使用。
fn mode_code(s: &str) -> u8 {
    match s {
        "width" => 1,
        "height" => 2,
        "original" => 3,
        "fixed" => 4,
        _ => 0,
    }
}

/// 取得指定頁面，經 Rust 後端依縮放模式以 Lanczos3 處理後的 PNG 位元組。
#[tauri::command]
pub fn render_page(req: RenderRequest, state: State<AppState>) -> Result<Response, String> {
    current_book(&state.book, req.session_id)?;
    let key = req.key_for(req.index);

    // 先取出查詢結果再判斷：務必讓鎖在本行結束即釋放。
    // （若寫成 `if let ... = lock().get() {} else { lock() }`，暫時鎖會存活到整段
    //   if/else 結束，於 else 分支再次上鎖將造成自我死鎖。）
    let cached = state.render.lock().unwrap().get(&key);
    let png = match cached {
        Some(bytes) => bytes, // 算繪快取命中，零運算。
        None => {
            let img = get_or_decode(&state.book, &state.decode, req.session_id, req.index)?;
            let bytes = image_pipeline::render(&img, &req.spec())?;
            let rendered = Arc::new(bytes);
            state.render.lock().unwrap().put(key, rendered.clone());
            rendered
        }
    };

    let session_id = req.session_id;
    // 派一筆新世代的預載任務（同時令工作執行緒中止舊任務）。
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = state
        .preload_tx
        .lock()
        .unwrap()
        .send(PreloadJob { generation, req });

    current_book(&state.book, session_id)?;
    Ok(Response::new((*png).clone()))
}

fn current_book(book: &BookSlot, session_id: u64) -> Result<Arc<Book>, String> {
    let guard = book.lock().unwrap();
    let (active_session, active_book) = guard.as_ref().ok_or("尚未開啟任何書籍。")?;
    if *active_session != session_id {
        return Err("閱讀來源已切換。".into());
    }
    Ok(active_book.clone())
}

fn get_or_decode(
    book: &BookSlot,
    decode: &Arc<Mutex<DecodeCache>>,
    session_id: u64,
    index: usize,
) -> Result<Arc<DynamicImage>, String> {
    let active_book = current_book(book, session_id)?;
    if let Some(img) = decode.lock().unwrap().get(&(session_id, index)) {
        return Ok(img);
    }
    let bytes = active_book.read_page(index)?;
    let img = Arc::new(image::load_from_memory(&bytes).map_err(|e| format!("解碼影像失敗：{e}"))?);
    current_book(book, session_id)?;
    decode.lock().unwrap().put((session_id, index), img.clone());
    Ok(img)
}

/// 單一常駐預載工作執行緒：背景把鄰頁的**最終 PNG** 先做好，
/// 但同時只跑一條，且偵測到新世代即中止過時任務，避免搶 CPU。
fn spawn_preload_worker(
    book: BookSlot,
    decode: Arc<Mutex<DecodeCache>>,
    render: Arc<Mutex<RenderCache>>,
    generation: Arc<AtomicU64>,
    rx: Receiver<PreloadJob>,
) {
    std::thread::spawn(move || {
        while let Ok(job) = rx.recv() {
            // 已有更新的世代 → 此任務過時，略過（直接處理最新的）。
            if generation.load(Ordering::SeqCst) != job.generation {
                continue;
            }
            let count = current_book(&book, job.req.session_id)
                .map(|b| b.len())
                .unwrap_or(0);
            if count == 0 {
                continue;
            }

            // 往前多看數頁（雙頁連翻時，下一對、下兩對都先備妥）；近端優先。
            let center = job.req.index;
            let mut targets = vec![center + 1, center + 2, center + 3, center + 4];
            if center >= 1 {
                targets.push(center - 1);
            }
            if center >= 2 {
                targets.push(center - 2);
            }

            for t in targets {
                // 翻頁／切模式產生新世代 → 立即中止剩餘預載。
                if generation.load(Ordering::SeqCst) != job.generation {
                    break;
                }
                if t >= count {
                    continue;
                }
                let key = job.req.key_for(t);
                if render.lock().unwrap().contains(&key) {
                    continue;
                }
                let img = match get_or_decode(&book, &decode, job.req.session_id, t) {
                    Ok(img) => img,
                    Err(_) => continue,
                };
                if let Ok(png) = image_pipeline::render(&img, &job.req.spec()) {
                    render.lock().unwrap().put(key, Arc::new(png));
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_session_cannot_use_new_book_or_its_cached_pixels() {
        let root = std::env::temp_dir().join(format!("mangafolio-session-{}", std::process::id()));
        std::fs::create_dir_all(root.join("first")).unwrap();
        std::fs::create_dir_all(root.join("second")).unwrap();
        image::RgbImage::from_pixel(4, 4, image::Rgb([255, 0, 0]))
            .save(root.join("first/1.png"))
            .unwrap();
        image::RgbImage::from_pixel(4, 4, image::Rgb([0, 255, 0]))
            .save(root.join("second/1.png"))
            .unwrap();
        let first = book::open(root.join("first").to_str().unwrap())
            .unwrap()
            .book;
        let second = book::open(root.join("second").to_str().unwrap())
            .unwrap()
            .book;
        let books: BookSlot = Arc::new(Mutex::new(Some((1, Arc::new(first)))));
        let decode = Arc::new(Mutex::new(Lru::new(14)));
        assert_eq!(
            get_or_decode(&books, &decode, 1, 0)
                .unwrap()
                .to_rgb8()
                .get_pixel(0, 0)
                .0,
            [255, 0, 0]
        );
        *books.lock().unwrap() = Some((2, Arc::new(second)));
        assert!(get_or_decode(&books, &decode, 1, 0).is_err());
        assert_eq!(
            get_or_decode(&books, &decode, 2, 0)
                .unwrap()
                .to_rgb8()
                .get_pixel(0, 0)
                .0,
            [0, 255, 0]
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
