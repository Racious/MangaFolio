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

/// 解碼快取容量（已解碼影像張數，每張較占記憶體）。
const DECODE_CAPACITY: usize = 14;
/// 算繪快取容量（最終 PNG 張數，每張較小）。
const RENDER_CAPACITY: usize = 48;

type DecodeCache = Lru<usize, Arc<DynamicImage>>;
type RenderCache = Lru<RenderKey, Arc<Vec<u8>>>;
type BookSlot = Arc<Mutex<Option<Book>>>;

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
            preload_tx: Mutex::new(tx),
        }
    }
}

/// 開啟書籍後回傳給前端的資訊。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookInfo {
    pub title: String,
    pub page_count: usize,
    pub pages: Vec<String>,
    pub start_index: usize,
}

/// 開啟資料夾／壓縮檔／單張圖片，建立頁面清單。
#[tauri::command]
pub fn open_path(path: String, state: State<AppState>) -> Result<BookInfo, String> {
    let result = book::open(&path)?;
    let info = BookInfo {
        title: result.book.title.clone(),
        page_count: result.book.len(),
        pages: result.book.page_names(),
        start_index: result.start_index,
    };
    *state.book.lock().unwrap() = Some(result.book);
    state.decode.lock().unwrap().clear();
    state.render.lock().unwrap().clear();
    Ok(info)
}

/// 前端傳來的縮放請求。
#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RenderRequest {
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
    let key = req.key_for(req.index);

    // 先取出查詢結果再判斷：務必讓鎖在本行結束即釋放。
    // （若寫成 `if let ... = lock().get() {} else { lock() }`，暫時鎖會存活到整段
    //   if/else 結束，於 else 分支再次上鎖將造成自我死鎖。）
    let cached = state.render.lock().unwrap().get(&key);
    let png = match cached {
        Some(bytes) => bytes, // 算繪快取命中，零運算。
        None => {
            let img = get_or_decode(&state.book, &state.decode, req.index)?;
            let bytes = image_pipeline::render(&img, &req.spec())?;
            let rendered = Arc::new(bytes);
            state.render.lock().unwrap().put(key, rendered.clone());
            rendered
        }
    };

    // 派一筆新世代的預載任務（同時令工作執行緒中止舊任務）。
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let _ = state
        .preload_tx
        .lock()
        .unwrap()
        .send(PreloadJob { generation, req });

    Ok(Response::new((*png).clone()))
}

/// 從解碼快取取得影像；未命中則讀檔解碼並寫入快取。
fn get_or_decode(
    book: &BookSlot,
    decode: &Arc<Mutex<DecodeCache>>,
    index: usize,
) -> Result<Arc<DynamicImage>, String> {
    if let Some(img) = decode.lock().unwrap().get(&index) {
        return Ok(img);
    }
    let bytes = {
        let guard = book.lock().unwrap();
        guard.as_ref().ok_or("尚未開啟任何書籍。")?.read_page(index)?
    };
    let img = Arc::new(image::load_from_memory(&bytes).map_err(|e| format!("解碼影像失敗：{e}"))?);
    decode.lock().unwrap().put(index, img.clone());
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
            let count = book.lock().unwrap().as_ref().map(|b| b.len()).unwrap_or(0);
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
                let img = match get_or_decode(&book, &decode, t) {
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
