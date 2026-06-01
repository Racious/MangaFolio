//! 書籍抽象：統一「資料夾」與「壓縮檔」兩種來源的頁面存取介面。

use std::path::{Path, PathBuf};

use crate::sorting::natural_cmp;

/// 支援的圖片副檔名（小寫）。
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "jfif", "png", "gif", "webp", "bmp"];

/// 依檔名／條目名判斷是否為支援的圖片。
pub fn is_image(name: &str) -> bool {
    match name.rsplit('.').next() {
        Some(ext) => IMAGE_EXTS.contains(&ext.to_ascii_lowercase().as_str()),
        None => false,
    }
}

/// 書籍來源。
pub enum Source {
    /// 資料夾：`entries` 已存各頁的絕對路徑字串，無需另存資料夾路徑。
    Folder,
    /// 壓縮檔：`entries` 為壓縮檔內各頁的條目名稱，讀取時據此重新開檔。
    Zip(PathBuf),
}

/// 一本已開啟的書。
pub struct Book {
    source: Source,
    entries: Vec<String>,
    pub title: String,
}

/// 開啟結果：書本身與建議的起始頁碼。
pub struct OpenResult {
    pub book: Book,
    pub start_index: usize,
}

impl Book {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 供前端顯示用的頁面名稱清單。
    pub fn page_names(&self) -> Vec<String> {
        match &self.source {
            Source::Folder => self.entries.iter().map(|p| file_name_of(p)).collect(),
            Source::Zip(_) => self.entries.clone(),
        }
    }

    /// 讀出第 `index` 頁的原始位元組（尚未經縮放／重新編碼）。
    pub fn read_page(&self, index: usize) -> Result<Vec<u8>, String> {
        let entry = self
            .entries
            .get(index)
            .ok_or_else(|| format!("頁碼超出範圍：{index}"))?;
        match &self.source {
            Source::Folder => std::fs::read(entry).map_err(|e| format!("讀取頁面失敗：{e}")),
            Source::Zip(path) => crate::archive::zip::read_entry(path, entry),
        }
    }
}

/// 開啟一個路徑：自動判別資料夾／壓縮檔／單張圖片。
pub fn open(path: &str) -> Result<OpenResult, String> {
    let p = Path::new(path);

    if p.is_dir() {
        let entries = scan_folder(p)?;
        if entries.is_empty() {
            return Err("此資料夾沒有可顯示的圖片。".into());
        }
        return Ok(OpenResult {
            book: Book {
                title: dir_title(p, path),
                source: Source::Folder,
                entries,
            },
            start_index: 0,
        });
    }

    if p.is_file() {
        let ext = p
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            "zip" | "cbz" => {
                let mut entries = crate::archive::zip::list_images(p)?;
                if entries.is_empty() {
                    return Err("此壓縮檔沒有可顯示的圖片。".into());
                }
                entries.sort_by(|a, b| natural_cmp(a, b));
                return Ok(OpenResult {
                    book: Book {
                        title: dir_title(p, path),
                        source: Source::Zip(p.to_path_buf()),
                        entries,
                    },
                    start_index: 0,
                });
            }
            _ if is_image(&p.to_string_lossy()) => {
                // 開啟單張圖片 → 改開其所在資料夾，並定位到該圖。
                let parent = p.parent().ok_or("無法取得圖片所在資料夾。")?;
                let entries = scan_folder(parent)?;
                let start_index = locate(&entries, p);
                return Ok(OpenResult {
                    book: Book {
                        title: dir_title(parent, path),
                        source: Source::Folder,
                        entries,
                    },
                    start_index,
                });
            }
            _ => return Err(format!("不支援的檔案格式：.{ext}")),
        }
    }

    Err(format!("路徑不存在：{path}"))
}

/// 掃描資料夾內的圖片並自然排序，回傳絕對路徑清單。
fn scan_folder(dir: &Path) -> Result<Vec<String>, String> {
    let mut entries: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| format!("讀取資料夾失敗：{e}"))?
        .filter_map(|r| r.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_image(&p.to_string_lossy()))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    entries.sort_by(|a, b| natural_cmp(&file_name_of(a), &file_name_of(b)));
    Ok(entries)
}

/// 在已排序清單中找出目標圖片的索引。
fn locate(entries: &[String], target: &Path) -> usize {
    let canon = target.canonicalize().ok();
    entries
        .iter()
        .position(|e| match (&canon, Path::new(e).canonicalize().ok()) {
            (Some(a), Some(b)) => *a == b,
            _ => false,
        })
        .unwrap_or(0)
}

fn file_name_of(p: &str) -> String {
    Path::new(p)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string())
}

fn dir_title(p: &Path, fallback: &str) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| fallback.to_string())
}
