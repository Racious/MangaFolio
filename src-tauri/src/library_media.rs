use super::*;
use crate::media_source::is_video;

pub(super) fn migrate(conn: &mut Connection) -> Result<(), String> {
    let tx = conn.transaction().map_err(db_error)?;
    tx.execute_batch("CREATE TABLE custom_covers(book_id INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE, data BLOB NOT NULL);
        CREATE TABLE library_roots(path TEXT PRIMARY KEY);
        PRAGMA user_version=5;").map_err(db_error)?;
    tx.commit().map_err(db_error)
}

fn video_path(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path)
        .canonicalize()
        .map_err(|e| format!("無法讀取影片來源：{e}"))?;
    if !p.is_file() || !is_video(&p) {
        return Err("請選擇支援的影片檔案。".into());
    }
    // Check actual readability before adding; opening never executes arbitrary files.
    std::fs::File::open(&p).map_err(|e| format!("影片無法存取：{e}"))?;
    Ok(p)
}

impl Library {
    fn source_location(&self, id: i64) -> Result<PathBuf, String> {
        let saved = self.get(id)?;
        let path = Path::new(&saved.path)
            .canonicalize()
            .map_err(|e| format!("來源無法存取；請確認檔案仍在原位或重新連結：{e}"))?;
        if !path.is_dir() && !path.is_file() {
            return Err("來源不是一般檔案或資料夾。".into());
        }
        Ok(path)
    }

    pub fn import_source(&self, path: &str) -> Result<(LibraryBook, bool), String> {
        if is_video(Path::new(path)) {
            self.register_video(path, None)
        } else {
            self.register_outcome(&book::open(path)?.book, None)
        }
    }

    fn register_video(
        &self,
        path: &str,
        relink: Option<i64>,
    ) -> Result<(LibraryBook, bool), String> {
        let p = video_path(path)?;
        let path = p.to_string_lossy().into_owned();
        let title = p
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let mut conn = self.connection.lock().map_err(db_error)?;
        let tx = conn.transaction().map_err(db_error)?;
        let matched = source_id(&tx, &path, relink)?;
        let id = if let Some(id) = relink {
            let saved = tx
                .query_row(
                    &format!("SELECT {COLUMNS} FROM books WHERE id=?1"),
                    [id],
                    row_book,
                )
                .map_err(db_error)?;
            if saved.format != "video" {
                return Err("漫畫來源不能改成影片。".into());
            }
            if matched.is_some() {
                return Err("這個來源已在書庫中，請選擇其他來源。".into());
            }
            tx.execute(
                "UPDATE books SET path=?1,title=?2 WHERE id=?3",
                params![path, title, id],
            )
            .map_err(db_error)?;
            id
        } else if let Some(id) = matched {
            let format: String = tx
                .query_row("SELECT format FROM books WHERE id=?1", [id], |r| r.get(0))
                .map_err(db_error)?;
            if format != "video" {
                return Err("來源衝突：此來源已登記為漫畫。".into());
            }
            tx.execute("UPDATE books SET title=?1 WHERE id=?2", params![title, id])
                .map_err(db_error)?;
            id
        } else {
            // The legacy positive page_count constraint is retained; video UI never uses it.
            tx.execute("INSERT INTO books(path,title,format,page_count,preferences,created_at) VALUES(?1,?2,'video',1,?3,?4)",
                params![path,title,serde_json::to_string(&ReaderPreferences::default()).map_err(db_error)?,now()]).map_err(db_error)?;
            tx.last_insert_rowid()
        };
        let saved = tx
            .query_row(
                &format!("SELECT {COLUMNS} FROM books WHERE id=?1"),
                [id],
                row_book,
            )
            .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok((saved, relink.is_none() && matched.is_none()))
    }

    pub fn relink_source(&self, id: i64, path: &str) -> Result<LibraryBook, String> {
        let saved = self.get(id)?;
        if saved.format == "video" {
            self.register_video(path, Some(id)).map(|r| r.0)
        } else {
            if is_video(Path::new(path)) {
                return Err("漫畫來源不能改成影片。".into());
            }
            self.relink(id, &book::open(path)?.book)
        }
    }

    pub fn external_video_path(&self, id: i64) -> Result<PathBuf, String> {
        let saved = self.get(id)?;
        if saved.format != "video" {
            return Err("此作品不是影片。".into());
        }
        video_path(&saved.path)
    }
}

#[tauri::command]
pub async fn show_library_source_location(id: i64, app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use tauri_plugin_opener::OpenerExt;
    tauri::async_runtime::spawn_blocking(move || {
        let path = app.state::<Library>().source_location(id)?;
        // Only directories are opened. Files are selected without launching their application.
        if path.is_dir() {
            app.opener().open_path(path.to_string_lossy(), None::<&str>)
        } else {
            app.opener().reveal_item_in_dir(path)
        }
        .map_err(|e| format!("無法在檔案總管中顯示來源：{e}"))
    })
    .await
    .map_err(db_error)?
}

#[tauri::command]
pub async fn open_library_video(id: i64, app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use tauri_plugin_opener::OpenerExt;
    tauri::async_runtime::spawn_blocking(move || {
        let library = app.state::<Library>();
        let path = library.external_video_path(id)?;
        app.opener()
            .open_path(path.to_string_lossy(), None::<&str>)
            .map_err(|e| format!("無法開啟影片，請確認 Windows 預設播放器與檔案關聯：{e}"))?;
        library.mark_opened(id)
    })
    .await
    .map_err(db_error)?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_location_resolves_stored_sources_and_rejects_missing_items() {
        let dir = std::env::temp_dir().join(format!("mangafolio-location-{}-{}", std::process::id(), now()));
        std::fs::create_dir_all(&dir).unwrap();
        let folder = dir.join("圖片書籍");
        std::fs::create_dir_all(&folder).unwrap();
        image::DynamicImage::new_rgb8(2, 2).save(folder.join("001.png")).unwrap();
        let movie = dir.join("測試影片.mp4");
        std::fs::write(&movie, b"fixture").unwrap();
        let library = Library::open(&dir.join("data")).unwrap();
        let comic = library.import_source(folder.to_str().unwrap()).unwrap().0;
        let video = library.import_source(movie.to_str().unwrap()).unwrap().0;
        assert!(library.source_location(comic.id).unwrap().is_dir());
        assert_eq!(library.source_location(video.id).unwrap(), movie.canonicalize().unwrap());
        assert!(library.source_location(i64::MAX).is_err());
        std::fs::remove_file(&movie).unwrap();
        assert!(library.source_location(video.id).unwrap_err().contains("重新連結"));
        assert_eq!(library.get(video.id).unwrap().last_read_at, None);
        drop(library);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn v4_upgrade_takes_snapshot_and_preserves_existing_books() {
        let dir =
            std::env::temp_dir().join(format!("mangafolio-v5-{}-{}", std::process::id(), now()));
        let l = Library::open(&dir).unwrap();
        l.connection
            .lock()
            .unwrap()
            .execute_batch(
                "DROP TABLE custom_covers; DROP TABLE library_roots; PRAGMA user_version=4;",
            )
            .unwrap();
        drop(l);
        let l = Library::open(&dir).unwrap();
        let files: Vec<_> = std::fs::read_dir(dir.join("backups"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(files.len(), 1);
        let snapshot = Connection::open(&files[0]).unwrap();
        assert_eq!(
            snapshot
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            4
        );
        assert_eq!(
            l.connection
                .lock()
                .unwrap()
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
        drop(snapshot);
        drop(l);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn videos_keep_metadata_identity_and_never_enter_comic_relink() {
        let dir =
            std::env::temp_dir().join(format!("mangafolio-video-{}-{}", std::process::id(), now()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("movie.MP4");
        std::fs::write(&p, b"fixture").unwrap();
        let l = Library::open(&dir.join("data")).unwrap();
        let (v, added) = l.import_source(p.to_str().unwrap()).unwrap();
        assert!(added);
        assert_eq!(v.format, "video");
        l.favorite(v.id, true).unwrap();
        l.edit_details(
            v.id,
            &BookDetails {
                custom_title: "自訂".into(),
                ..Default::default()
            },
        )
        .unwrap();
        l.set_status(&[v.id], "read").unwrap();
        let (again, added) = l.import_source(p.to_str().unwrap()).unwrap();
        assert!(!added);
        assert_eq!(again.id, v.id);
        assert!(again.favorite);
        assert_eq!(again.title, "自訂");
        assert_eq!(again.reading_status, "read");
        assert!(l.set_status(&[v.id], "auto").is_err());
        let q = dir.join("moved.mkv");
        std::fs::rename(&p, &q).unwrap();
        assert!(l.external_video_path(v.id).is_err());
        assert_eq!(l.relink_source(v.id, q.to_str().unwrap()).unwrap().id, v.id);
        assert_eq!(
            l.external_video_path(v.id).unwrap(),
            q.canonicalize().unwrap()
        );
        let image = dir.join("001.png");
        image::DynamicImage::new_rgb8(2, 2).save(&image).unwrap();
        let comic = l.import_source(dir.to_str().unwrap()).unwrap().0;
        assert!(l.external_video_path(comic.id).is_err());
        assert!(l.relink_source(comic.id, q.to_str().unwrap()).is_err());
        assert!(l.relink_source(v.id, dir.to_str().unwrap()).is_err());
        assert_eq!(
            l.connection
                .lock()
                .unwrap()
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
        drop(l);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[tauri::command]
pub fn list_library_roots(library: State<Library>) -> Result<Vec<String>, String> {
    let conn = library.connection.lock().map_err(db_error)?;
    let mut query = conn
        .prepare("SELECT path FROM library_roots ORDER BY path")
        .map_err(db_error)?;
    let rows = query
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db_error)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
}
#[tauri::command]
pub fn remember_library_roots(paths: Vec<String>, library: State<Library>) -> Result<(), String> {
    if paths.len() > 100 {
        return Err("一次最多記住100個主目錄。".into());
    }
    let canonical: Vec<_> = paths
        .iter()
        .map(|p| {
            let p = Path::new(p).canonicalize().map_err(db_error)?;
            if !p.is_dir() {
                return Err("主目錄必須是存在的資料夾。".into());
            }
            Ok(p.to_string_lossy().into_owned())
        })
        .collect::<Result<_, String>>()?;
    let mut conn = library.connection.lock().map_err(db_error)?;
    let tx = conn.transaction().map_err(db_error)?;
    for path in canonical {
        tx.execute(
            "INSERT OR IGNORE INTO library_roots(path) VALUES(?1)",
            [path],
        )
        .map_err(db_error)?;
    }
    let count: usize = tx
        .query_row("SELECT COUNT(*) FROM library_roots", [], |r| r.get(0))
        .map_err(db_error)?;
    if count > 100 {
        return Err("最多記住100個主目錄；請先移除不再使用的路徑。".into());
    }
    tx.commit().map_err(db_error)
}
#[tauri::command]
pub fn forget_library_root(path: String, library: State<Library>) -> Result<(), String> {
    library
        .connection
        .lock()
        .map_err(db_error)?
        .execute("DELETE FROM library_roots WHERE path=?1", [path])
        .map_err(db_error)?;
    Ok(())
}
