use super::*;
use crate::media_source::{is_video, matching_cover};
use std::io::{Cursor, Read};
const MAX_COVER_SOURCE: u64 = 16 * 1024 * 1024;
const MAX_STORED_COVER: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomCover {
    pub book_id: i64,
    pub data: Vec<u8>,
}

fn decode(bytes: &[u8], stored: bool) -> Result<image::DynamicImage, String> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(db_error)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(if stored { 240 } else { 10_000 });
    limits.max_image_height = Some(if stored { 340 } else { 10_000 });
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    if stored && reader.format() != Some(image::ImageFormat::Png) {
        return Err("備份人工封面必須是PNG。".into());
    }
    reader
        .decode()
        .map_err(|e| format!("封面圖片無法解碼或超過尺寸限制：{e}"))
}
fn render_cover(path: &Path) -> Result<Vec<u8>, String> {
    if !path.is_file() || !book::is_image(&path.to_string_lossy()) {
        return Err("請選擇支援的圖片檔案。".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(db_error)?
        .take(MAX_COVER_SOURCE + 1)
        .read_to_end(&mut bytes)
        .map_err(db_error)?;
    if bytes.len() as u64 > MAX_COVER_SOURCE {
        return Err("封面來源圖片最多16 MiB。".into());
    }
    let image = decode(&bytes, false)?;
    image_pipeline::render(
        &image,
        &image_pipeline::ScaleSpec {
            mode: image_pipeline::FitMode::Window,
            viewport_w: 240,
            viewport_h: 340,
            fixed_scale: 1.0,
        },
    )
}

pub(super) fn custom_covers_from(conn: &Connection) -> Result<Vec<CustomCover>, String> {
    let mut query = conn
        .prepare("SELECT book_id,data FROM custom_covers ORDER BY book_id")
        .map_err(db_error)?;
    let rows = query
        .query_map([], |r| {
            Ok(CustomCover {
                book_id: r.get(0)?,
                data: r.get(1)?,
            })
        })
        .map_err(db_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
}
pub(super) fn validate_covers(backup: &LibraryBackup) -> Result<(), String> {
    if backup.version < 4
        && (!backup.custom_covers.is_empty() || backup.books.iter().any(|b| b.format == "video"))
    {
        return Err("舊版備份不支援影片或人工封面。".into());
    }
    let mut ids = std::collections::HashSet::new();
    let mut book_ids = std::collections::HashMap::<i64, usize>::new();
    for book in &backup.books {
        *book_ids.entry(book.id).or_default() += 1;
    }
    for cover in &backup.custom_covers {
        if !ids.insert(cover.book_id)
            || cover.book_id <= 0
            || book_ids.get(&cover.book_id) != Some(&1)
            || cover.data.len() > MAX_STORED_COVER
        {
            return Err("備份封面資料／關聯無效或重複。".into());
        }
        decode(&cover.data, true)?;
    }
    Ok(())
}

impl Library {
    pub fn custom_cover(&self, id: i64) -> Result<Option<Vec<u8>>, String> {
        self.connection
            .lock()
            .map_err(db_error)?
            .query_row(
                "SELECT data FROM custom_covers WHERE book_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)
    }
    pub fn replace_cover(&self, id: i64, path: Option<&Path>) -> Result<(), String> {
        // Validate/decode before locking or mutating the previous cover.
        let data = path.map(render_cover).transpose()?;
        if data.as_ref().is_some_and(|d| d.len() > MAX_STORED_COVER) {
            return Err("處理後封面超過1 MiB。".into());
        }
        let _cover = self.cover_lock.lock().map_err(db_error)?;
        let mut conn = self.connection.lock().map_err(db_error)?;
        let tx = conn.transaction().map_err(db_error)?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM books WHERE id=?1)",
                [id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if !exists {
            return Err("作品已不存在，未修改封面。".into());
        }
        if let Some(data) = data {
            tx.execute("INSERT INTO custom_covers(book_id,data) VALUES(?1,?2) ON CONFLICT(book_id) DO UPDATE SET data=excluded.data",params![id,data]).map_err(db_error)?;
        } else {
            tx.execute("DELETE FROM custom_covers WHERE book_id=?1", [id])
                .map_err(db_error)?;
        }
        tx.commit().map_err(db_error)
    }
    pub fn video_cover(&self, id: i64, path: &Path) -> Result<Vec<u8>, String> {
        let dir = path.parent().ok_or("無法取得影片資料夾。")?;
        let files = std::fs::read_dir(dir)
            .map_err(db_error)?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        let video_count = files.iter().filter(|p| p.is_file() && is_video(p)).count();
        let cover = matching_cover(path, &files, video_count)
            .ok_or("尚無可辨識的影片封面；可在編輯資訊中自行指定。")?;
        let metadata = std::fs::metadata(&cover).map_err(db_error)?;
        let stamp = format!(
            "video-{}-{}-{}",
            cover.display(),
            metadata.len(),
            metadata
                .modified()
                .map_err(db_error)?
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let cached = self.covers.join(format!("{id}.png"));
        let stamp_file = self.covers.join(format!("{id}.stamp"));
        if std::fs::read_to_string(&stamp_file).ok().as_deref() == Some(&stamp) {
            if let Ok(bytes) = std::fs::read(&cached) {
                return Ok(bytes);
            }
        }
        let bytes = render_cover(&cover)?;
        let temporary = self.covers.join(format!("{id}.tmp"));
        std::fs::write(&temporary, &bytes).map_err(db_error)?;
        std::fs::rename(temporary, cached).map_err(db_error)?;
        std::fs::write(stamp_file, stamp).map_err(db_error)?;
        Ok(bytes)
    }
}

#[tauri::command]
pub async fn replace_library_cover(
    id: i64,
    path: Option<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Library>()
            .replace_cover(id, path.as_deref().map(Path::new))
    })
    .await
    .map_err(db_error)?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_covers_survive_source_loss_reimport_backup_and_reset() {
        let dir = std::env::temp_dir().join(format!(
            "mangafolio-covers-{}-{}",
            std::process::id(),
            now()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let video = dir.join("movie.mp4");
        std::fs::write(&video, b"fixture").unwrap();
        let auto = dir.join("movie.jpg");
        image::DynamicImage::new_rgb8(16, 24).save(&auto).unwrap();
        let replacement = dir.join("replacement.png");
        image::DynamicImage::new_rgb8(30, 40)
            .save(&replacement)
            .unwrap();
        let comic_dir = dir.join("comic");
        std::fs::create_dir(&comic_dir).unwrap();
        image::DynamicImage::new_rgb8(20, 30)
            .save(comic_dir.join("001.png"))
            .unwrap();
        let l = Library::open(&dir.join("data")).unwrap();
        let v = l.import_source(video.to_str().unwrap()).unwrap().0;
        let c = l.import_source(comic_dir.to_str().unwrap()).unwrap().0;
        assert!(!l.cover(v.id).unwrap().is_empty());
        let original = l.cover(c.id).unwrap();
        for id in [v.id, c.id] {
            l.replace_cover(id, Some(&replacement)).unwrap();
        }
        let custom = l.cover(v.id).unwrap();
        assert_eq!(custom, l.cover(c.id).unwrap());
        assert_ne!(custom, original);
        l.import_source(video.to_str().unwrap()).unwrap();
        assert_eq!(custom, l.cover(v.id).unwrap());
        let backup = l.backup_json().unwrap();
        let l2 = Library::open(&dir.join("restored")).unwrap();
        assert_eq!(l2.restore_json(&backup).unwrap().added, 2);
        let restored = l2.list().unwrap();
        assert!(restored.iter().all(|b| l2.cover(b.id).unwrap() == custom));
        std::fs::remove_file(&replacement).unwrap();
        std::fs::remove_file(&video).unwrap();
        std::fs::remove_dir_all(&comic_dir).unwrap();
        assert_eq!(l.cover(v.id).unwrap(), custom);
        assert_eq!(l.cover(c.id).unwrap(), custom);
        let invalid = dir.join("bad.png");
        std::fs::write(&invalid, b"not an image").unwrap();
        assert!(l.replace_cover(v.id, Some(&invalid)).is_err());
        assert_eq!(l.cover(v.id).unwrap(), custom);
        l.replace_cover(v.id, None).unwrap();
        assert!(
            l.cover(v.id).is_ok(),
            "matching sidecar remains usable offline"
        );
        let mut invalid_backup: LibraryBackup = serde_json::from_slice(&backup).unwrap();
        invalid_backup.custom_covers[0].book_id = 99999;
        let before = l2.backup_json().unwrap();
        assert!(l2
            .restore_json(&serde_json::to_vec(&invalid_backup).unwrap())
            .is_err());
        assert_eq!(l2.backup_json().unwrap(), before);
        l.remove(&[c.id]).unwrap();
        assert!(l.custom_cover(c.id).unwrap().is_none());
        drop(l2);
        drop(l);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
