//! ZIP / CBZ 讀取。
//!
//! 為求 P1 簡潔，每次讀取頁面時重新開啟壓縮檔；待 P3 引入 LRU 快取與
//! 預載後，再評估是否常駐 `ZipArchive` 控制代碼。

use std::io::Read;
use std::path::Path;

use crate::book::is_image;

/// 列出壓縮檔內所有圖片條目名稱（未排序）。
pub fn list_images(path: &Path) -> Result<Vec<String>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("開啟壓縮檔失敗：{e}"))?;
    let mut archive = ::zip::ZipArchive::new(file).map_err(|e| format!("解析壓縮檔失敗：{e}"))?;

    let mut names = Vec::new();
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("讀取條目失敗：{e}"))?;
        if entry.is_file() && is_image(entry.name()) {
            names.push(entry.name().to_string());
        }
    }
    Ok(names)
}

/// 依條目名稱讀出單一頁面的原始位元組。
pub fn read_entry(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("開啟壓縮檔失敗：{e}"))?;
    let mut archive = ::zip::ZipArchive::new(file).map_err(|e| format!("解析壓縮檔失敗：{e}"))?;
    let mut entry = archive
        .by_name(name)
        .map_err(|e| format!("找不到條目 {name}：{e}"))?;

    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry
        .read_to_end(&mut buf)
        .map_err(|e| format!("讀取條目內容失敗：{e}"))?;
    Ok(buf)
}
