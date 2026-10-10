//! File classification shared by import, cover matching and external opening.
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub const VIDEO_EXTS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "webm", "m4v", "mpg", "mpeg", "ts", "m2ts",
];
pub fn is_video(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        VIDEO_EXTS.contains(&ext.to_string_lossy().to_ascii_lowercase().as_str())
    })
}

pub fn is_archive(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        ["zip", "cbz"].contains(&ext.to_string_lossy().to_ascii_lowercase().as_str())
    })
}

/// Same-stem images always match; generic cover names only match a single video.
pub fn matching_cover(video: &Path, files: &[PathBuf], video_count: usize) -> Option<PathBuf> {
    let stem = video.file_stem()?.to_string_lossy().to_lowercase();
    let mut images: Vec<_> = files
        .iter()
        .filter(|p| p.is_file() && crate::book::is_image(&p.to_string_lossy()))
        .collect();
    images.sort_by_key(|p| p.to_string_lossy().to_lowercase());
    images
        .iter()
        .find(|p| {
            p.file_stem()
                .is_some_and(|s| s.to_string_lossy().to_lowercase() == stem)
        })
        .or_else(|| {
            if video_count == 1 {
                images.iter().find(|p| {
                    p.file_stem().is_some_and(|s| {
                        ["cover", "poster", "folder"]
                            .contains(&s.to_string_lossy().to_lowercase().as_str())
                    })
                })
            } else {
                None
            }
        })
        .map(|p| (*p).clone())
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportCandidate {
    pub path: String,
    pub title: String,
    pub format: String,
    pub selected: bool,
    pub existing: bool,
    pub warning: String,
}
#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub items: Vec<ImportCandidate>,
    pub issues: Vec<String>,
    pub canceled: bool,
    pub truncated: bool,
}
#[derive(Default)]
pub struct ScanState(pub Mutex<Option<Arc<AtomicBool>>>);

fn issue(result: &mut ScanResult, message: String) {
    if result.issues.len() < 200 {
        result.issues.push(message);
    } else {
        result.truncated = true;
    }
}
fn add(
    result: &mut ScanResult,
    path: &Path,
    format: &str,
    existing: &HashSet<String>,
    warning: &str,
) {
    if result.items.len() >= 10_000 {
        result.truncated = true;
        return;
    }
    let path = path.to_string_lossy().into_owned();
    let is_existing = existing.contains(&path);
    result.items.push(ImportCandidate {
        title: if format == "folder" {
            Path::new(&path).file_name()
        } else {
            Path::new(&path).file_stem()
        }
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned(),
        path,
        format: format.into(),
        existing: is_existing,
        selected: !is_existing && warning.is_empty(),
        warning: warning.into(),
    });
}
fn redirected(entry: &std::fs::DirEntry) -> bool {
    let Ok(kind) = entry.file_type() else {
        return true;
    };
    if kind.is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Reparse points include junctions, which file_type alone can miss.
        if entry
            .metadata()
            .map(|m| m.file_attributes() & 0x400 != 0)
            .unwrap_or(true)
        {
            return true;
        }
    }
    false
}

pub fn scan(
    paths: Vec<String>,
    recursive: bool,
    existing: HashSet<String>,
    cancel: &AtomicBool,
) -> ScanResult {
    let mut result = ScanResult::default();
    let mut stack: Vec<_> = paths
        .into_iter()
        .rev()
        .map(|p| (PathBuf::from(p), 0usize))
        .collect();
    let mut visited = HashSet::new();
    let mut seen_entries = 0usize;
    while let Some((raw, depth)) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            result.canceled = true;
            break;
        }
        if seen_entries >= 100_000 || result.items.len() >= 10_000 {
            result.truncated = true;
            break;
        }
        let path = match raw.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                issue(&mut result, format!("{}：{e}", raw.display()));
                continue;
            }
        };
        if !visited.insert(path.clone()) {
            continue;
        }
        if path.is_file() {
            if is_video(&path) {
                add(&mut result, &path, "video", &existing, "");
            } else if is_archive(&path) {
                add(&mut result, &path, "cbz", &existing, "");
            } else {
                issue(&mut result, format!("{}：不支援的媒體檔案", path.display()));
            }
            continue;
        }
        let entries = match std::fs::read_dir(&path) {
            Ok(e) => e,
            Err(e) => {
                issue(&mut result, format!("{}：{e}", path.display()));
                continue;
            }
        };
        let mut files = vec![];
        let mut children = vec![];
        for entry in entries {
            if cancel.load(Ordering::Relaxed) {
                result.canceled = true;
                break;
            }
            seen_entries += 1;
            if seen_entries > 100_000 {
                result.truncated = true;
                break;
            }
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    issue(&mut result, format!("{}：{e}", path.display()));
                    continue;
                }
            };
            if redirected(&entry) {
                issue(
                    &mut result,
                    format!("{}：略過連結／重新導向來源", entry.path().display()),
                );
                continue;
            }
            let p = entry.path();
            if p.is_dir() {
                children.push(p);
            } else if p.is_file() {
                files.push(p);
            }
        }
        if result.canceled {
            break;
        }
        files.sort();
        children.sort();
        let videos: Vec<_> = files.iter().filter(|p| is_video(p)).collect();
        let covers: HashSet<_> = videos
            .iter()
            .filter_map(|p| matching_cover(p, &files, videos.len()))
            .collect();
        let has_pages = files
            .iter()
            .any(|p| crate::book::is_image(&p.to_string_lossy()) && !covers.contains(p));
        let archives = files.iter().any(|p| is_archive(p));
        if has_pages {
            let warning = if !videos.is_empty() || archives || !children.is_empty() {
                "此目錄同時含圖片及其他媒體／子目錄；未預選。勾選時只匯入直接圖片（包含封面），不合併子目錄。"
            } else {
                ""
            };
            add(&mut result, &path, "folder", &existing, warning);
        }
        for p in files {
            if is_video(&p) || is_archive(&p) {
                if visited.insert(p.clone()) {
                    add(
                        &mut result,
                        &p,
                        if is_video(&p) { "video" } else { "cbz" },
                        &existing,
                        "",
                    );
                }
            }
        }
        if recursive {
            if depth >= 64 && !children.is_empty() {
                issue(&mut result, format!("{}：已達64層深度限制", path.display()));
                result.truncated = true;
            } else {
                stack.extend(children.into_iter().rev().map(|p| (p, depth + 1)));
            }
        }
    }
    result
}

#[tauri::command]
pub async fn scan_library_sources(
    paths: Vec<String>,
    recursive: bool,
    app: tauri::AppHandle,
) -> Result<ScanResult, String> {
    use tauri::Manager;
    if paths.is_empty() || paths.len() > 100 {
        return Err("請選擇1–100個匯入來源。".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let state = app.state::<ScanState>();
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("已有掃描進行中。".into());
        }
        *guard = Some(cancel.clone());
    }
    let app_copy = app.clone();
    let worker_cancel = cancel.clone();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        let existing = app_copy
            .state::<crate::library::Library>()
            .list()?
            .into_iter()
            .map(|b| {
                Path::new(&b.path)
                    .canonicalize()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or(b.path)
            })
            .collect();
        Ok(scan(paths, recursive, existing, &worker_cancel))
    })
    .await
    .map_err(|e| e.to_string());
    let state = app.state::<ScanState>();
    *state.0.lock().map_err(|e| e.to_string())? = None;
    outcome?
}
#[tauri::command]
pub fn cancel_library_scan(state: tauri::State<ScanState>) -> Result<(), String> {
    if let Some(cancel) = state.0.lock().map_err(|e| e.to_string())?.as_ref() {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scan_preserves_book_boundaries_pairs_covers_and_deduplicates_roots() {
        let root = std::env::temp_dir().join(format!(
            "mangafolio-scan-{}-{}",
            std::process::id(),
            crate::library::now()
        ));
        let pages = root.join("1001/comic");
        std::fs::create_dir_all(&pages).unwrap();
        std::fs::write(pages.join("001.jpg"), b"image").unwrap();
        let date = root.join("1002");
        std::fs::create_dir_all(&date).unwrap();
        std::fs::write(date.join("movie.mp4"), b"video").unwrap();
        std::fs::write(date.join("movie.jpg"), b"cover").unwrap();
        std::fs::write(date.join("book.cbz"), b"archive").unwrap();
        let existing =
            HashSet::from([pages.canonicalize().unwrap().to_string_lossy().into_owned()]);
        let r = scan(
            vec![
                root.to_string_lossy().into(),
                pages.to_string_lossy().into(),
            ],
            true,
            existing,
            &AtomicBool::new(false),
        );
        assert_eq!(r.items.len(), 3);
        assert_eq!(r.items.iter().filter(|p| p.format == "folder").count(), 1);
        assert!(
            r.items
                .iter()
                .find(|p| p.format == "folder")
                .unwrap()
                .existing
        );
        assert!(
            !r.items
                .iter()
                .find(|p| p.format == "folder")
                .unwrap()
                .selected
        );
        let r = scan(
            vec![root.to_string_lossy().into()],
            false,
            HashSet::new(),
            &AtomicBool::new(false),
        );
        assert!(r.items.is_empty());
        let r = scan(
            vec![root.to_string_lossy().into()],
            true,
            HashSet::new(),
            &AtomicBool::new(true),
        );
        assert!(r.canceled);
        assert!(r.items.is_empty());
        std::fs::write(date.join("unrelated.png"), b"image").unwrap();
        let r = scan(
            vec![date.to_string_lossy().into()],
            true,
            HashSet::new(),
            &AtomicBool::new(false),
        );
        let mixed = r.items.iter().find(|p| p.format == "folder").unwrap();
        assert!(!mixed.selected);
        assert!(!mixed.warning.is_empty());
        let invalid = scan(
            vec![root.join("missing").to_string_lossy().into()],
            true,
            HashSet::new(),
            &AtomicBool::new(false),
        );
        assert_eq!(invalid.issues.len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn generic_cover_is_not_shared_between_multiple_videos() {
        let dir = std::env::temp_dir().join(format!(
            "mangafolio-pair-{}-{}",
            std::process::id(),
            crate::library::now()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let image = dir.join("cover.jpg");
        std::fs::write(&image, b"image").unwrap();
        assert!(matching_cover(&dir.join("a.mp4"), &[image.clone()], 2).is_none());
        assert_eq!(
            matching_cover(&dir.join("a.mp4"), &[image], 1),
            Some(dir.join("cover.jpg"))
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
