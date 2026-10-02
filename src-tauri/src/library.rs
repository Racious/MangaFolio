//! Persistent local library. Original books are never copied or modified.
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{ipc::Response, State};

use crate::{book, image_pipeline};

pub struct Library {
    connection: Mutex<Connection>,
    covers: PathBuf,
    cover_lock: Mutex<()>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReaderPreferences {
    pub direction: String,
    pub page_mode: String,
    pub zoom: String,
    pub fixed_scale: f32,
    pub double_cover: bool,
    pub transition: String,
}

impl Default for ReaderPreferences {
    fn default() -> Self {
        Self {
            direction: "rtl".into(),
            page_mode: "single".into(),
            zoom: "window".into(),
            fixed_scale: 1.0,
            double_cover: false,
            transition: "book".into(),
        }
    }
}

impl ReaderPreferences {
    fn validate(&self) -> Result<(), String> {
        if !["rtl", "ltr"].contains(&self.direction.as_str())
            || !["single", "double"].contains(&self.page_mode.as_str())
            || !["window", "width", "height", "original", "fixed"].contains(&self.zoom.as_str())
            || !["book", "none", "slide", "fade"].contains(&self.transition.as_str())
            || !self.fixed_scale.is_finite()
            || !(0.1..=8.0).contains(&self.fixed_scale)
        {
            return Err("閱讀設定無效。".into());
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LibraryBook {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub format: String,
    pub page_count: usize,
    pub favorite: bool,
    pub last_index: usize,
    pub last_page_name: Option<String>,
    pub last_read_at: Option<i64>,
    pub preferences: ReaderPreferences,
    pub available: bool,
}

const COLUMNS: &str = "id, path, title, format, page_count, favorite, last_index, last_page_name, last_read_at, preferences";
fn db_error(e: impl std::fmt::Display) -> String {
    format!("書庫資料操作失敗：{e}")
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
fn row_book(row: &rusqlite::Row<'_>) -> rusqlite::Result<LibraryBook> {
    let path: String = row.get(1)?;
    let preferences: String = row.get(9)?;
    let preferences = serde_json::from_str(&preferences).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let page_count: usize = row.get(4)?;
    let last_index: usize = row.get(6)?;
    Ok(LibraryBook {
        id: row.get(0)?,
        available: Path::new(&path).exists(),
        path,
        title: row.get(2)?,
        format: row.get(3)?,
        page_count,
        favorite: row.get(5)?,
        last_index: last_index.min(page_count.saturating_sub(1)),
        last_page_name: row.get(7)?,
        last_read_at: row.get(8)?,
        preferences,
    })
}

impl Library {
    pub fn open(directory: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(directory).map_err(db_error)?;
        let connection = Connection::open(directory.join("library.sqlite3")).map_err(db_error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(db_error)?;
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(db_error)?;
        if version > 1 {
            return Err("書庫來自較新版本，請更新 MangaFolio；原始資料已保留。".into());
        }
        connection.execute_batch("PRAGMA journal_mode=WAL;
            BEGIN IMMEDIATE;
            CREATE TABLE IF NOT EXISTS books (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE, title TEXT NOT NULL, format TEXT NOT NULL,
                page_count INTEGER NOT NULL CHECK(page_count > 0), favorite INTEGER NOT NULL DEFAULT 0,
                last_index INTEGER NOT NULL DEFAULT 0, last_page_name TEXT,
                last_read_at INTEGER, preferences TEXT NOT NULL, created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS books_recent ON books(last_read_at DESC);
            PRAGMA user_version=1;
            COMMIT;").map_err(db_error)?;
        let covers = directory.join("covers");
        std::fs::create_dir_all(&covers).map_err(db_error)?;
        Ok(Self {
            connection: Mutex::new(connection),
            covers,
            cover_lock: Mutex::new(()),
        })
    }

    pub fn list(&self) -> Result<Vec<LibraryBook>, String> {
        let connection = self.connection.lock().map_err(db_error)?;
        let mut query = connection
            .prepare(&format!(
                "SELECT {COLUMNS} FROM books ORDER BY last_read_at DESC, id DESC"
            ))
            .map_err(db_error)?;
        let rows = query.query_map([], row_book).map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
    }

    pub fn get(&self, id: i64) -> Result<LibraryBook, String> {
        let connection = self.connection.lock().map_err(db_error)?;
        connection
            .query_row(
                &format!("SELECT {COLUMNS} FROM books WHERE id=?1"),
                [id],
                row_book,
            )
            .optional()
            .map_err(db_error)?
            .ok_or("找不到這本書。".into())
    }

    pub fn register(&self, book: &book::Book) -> Result<LibraryBook, String> {
        let path = book
            .source_path()
            .canonicalize()
            .map_err(db_error)?
            .to_string_lossy()
            .into_owned();
        let preferences = serde_json::to_string(&ReaderPreferences::default()).map_err(db_error)?;
        let connection = self.connection.lock().map_err(db_error)?;
        connection.execute("INSERT INTO books(path,title,format,page_count,preferences,created_at)
            VALUES(?1,?2,?3,?4,?5,?6)
            ON CONFLICT(path) DO UPDATE SET title=excluded.title,format=excluded.format,page_count=excluded.page_count",
            params![path, book.title, book.format(), book.len(), preferences, now()]).map_err(db_error)?;
        connection
            .query_row(
                &format!("SELECT {COLUMNS} FROM books WHERE path=?1"),
                [path],
                row_book,
            )
            .map_err(db_error)
    }

    pub fn mark_opened(&self, id: i64) -> Result<(), String> {
        self.connection
            .lock()
            .map_err(db_error)?
            .execute(
                "UPDATE books SET last_read_at=?1 WHERE id=?2",
                params![now(), id],
            )
            .map_err(db_error)?;
        Ok(())
    }

    pub fn save_progress(
        &self,
        id: i64,
        index: usize,
        page_name: &str,
        preferences: &ReaderPreferences,
    ) -> Result<(), String> {
        preferences.validate()?;
        let preferences = serde_json::to_string(preferences).map_err(db_error)?;
        let changed = self
            .connection
            .lock()
            .map_err(db_error)?
            .execute(
                "UPDATE books SET last_index=?1,last_page_name=?2,
            last_read_at=?3,preferences=?4 WHERE id=?5 AND page_count>?1",
                params![index, page_name, now(), preferences, id],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err("書籍不存在或頁碼超出範圍。".into());
        }
        Ok(())
    }

    pub fn favorite(&self, id: i64, favorite: bool) -> Result<(), String> {
        let changed = self
            .connection
            .lock()
            .map_err(db_error)?
            .execute(
                "UPDATE books SET favorite=?1 WHERE id=?2",
                params![favorite, id],
            )
            .map_err(db_error)?;
        if changed == 0 {
            return Err("找不到這本書。".into());
        }
        Ok(())
    }

    pub fn cover(&self, id: i64) -> Result<Vec<u8>, String> {
        let entry = self.get(id)?;
        let _guard = self.cover_lock.lock().map_err(db_error)?;
        let opened = book::open(&entry.path)?;
        let source = Path::new(&entry.path);
        let first = if source.is_dir() {
            source.join(&opened.book.page_names()[0])
        } else {
            source.to_path_buf()
        };
        let metadata = std::fs::metadata(first).map_err(db_error)?;
        let stamp = format!(
            "{}-{}",
            metadata.len(),
            metadata
                .modified()
                .map_err(db_error)?
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let cover = self.covers.join(format!("{id}.png"));
        let stamp_file = self.covers.join(format!("{id}.stamp"));
        if std::fs::read_to_string(&stamp_file).ok().as_deref() == Some(&stamp) {
            if let Ok(bytes) = std::fs::read(&cover) {
                return Ok(bytes);
            }
        }
        let image = image::load_from_memory(&opened.book.read_page(0)?)
            .map_err(|e| format!("封面解碼失敗：{e}"))?;
        let bytes = image_pipeline::render(
            &image,
            &image_pipeline::ScaleSpec {
                mode: image_pipeline::FitMode::Window,
                viewport_w: 240,
                viewport_h: 340,
                fixed_scale: 1.0,
            },
        )?;
        let temporary = self.covers.join(format!("{id}.tmp"));
        std::fs::write(&temporary, &bytes).map_err(db_error)?;
        std::fs::rename(temporary, &cover).map_err(db_error)?;
        std::fs::write(stamp_file, stamp).map_err(db_error)?;
        Ok(bytes)
    }
}

pub fn resume_index(saved: &LibraryBook, pages: &[String]) -> usize {
    saved
        .last_page_name
        .as_ref()
        .and_then(|name| pages.iter().position(|p| p == name))
        .unwrap_or(saved.last_index.min(pages.len().saturating_sub(1)))
}

#[tauri::command]
pub fn list_library(library: State<Library>) -> Result<Vec<LibraryBook>, String> {
    library.list()
}

#[tauri::command]
pub async fn import_book(path: String, library: State<'_, Library>) -> Result<LibraryBook, String> {
    let opened = tauri::async_runtime::spawn_blocking(move || book::open(&path))
        .await
        .map_err(db_error)??;
    library.register(&opened.book)
}

#[tauri::command]
pub fn set_favorite(id: i64, favorite: bool, library: State<Library>) -> Result<(), String> {
    library.favorite(id, favorite)
}

#[tauri::command]
pub fn save_reading_progress(
    id: i64,
    index: usize,
    page_name: String,
    preferences: ReaderPreferences,
    library: State<Library>,
) -> Result<(), String> {
    library.save_progress(id, index, &page_name, &preferences)
}

#[tauri::command]
pub async fn library_cover(id: i64, app: tauri::AppHandle) -> Result<Response, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Library>().cover(id).map(Response::new)
    })
    .await
    .map_err(db_error)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "mangafolio-library-{}-{}-{}",
                std::process::id(),
                now(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(path.join("pages")).unwrap();
            for name in ["1.png", "2.png", "10.png"] {
                image::RgbImage::from_pixel(20, 30, image::Rgb([255, 20, 20]))
                    .save(path.join("pages").join(name))
                    .unwrap();
            }
            Self(path)
        }
        fn book(&self) -> book::Book {
            book::open(self.0.join("pages").to_str().unwrap())
                .unwrap()
                .book
        }
        fn library(&self) -> Library {
            Library::open(&self.0.join("data")).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn favorites_progress_and_preferences_survive_restart() {
        let f = Fixture::new();
        let entry;
        let preferences = ReaderPreferences {
            direction: "ltr".into(),
            page_mode: "double".into(),
            ..Default::default()
        };
        {
            let library = f.library();
            entry = library.register(&f.book()).unwrap();
            library.favorite(entry.id, true).unwrap();
            library
                .save_progress(entry.id, 1, "2.png", &preferences)
                .unwrap();
        }
        let library = f.library();
        let restored = library.get(entry.id).unwrap();
        assert!(restored.favorite);
        assert_eq!(restored.last_index, 1);
        assert_eq!(restored.preferences, preferences);
        assert!(restored.last_read_at.is_some());
    }

    #[test]
    fn reimport_keeps_id_and_favorite_without_marking_unread_book_recent() {
        let f = Fixture::new();
        let library = f.library();
        let a = library.register(&f.book()).unwrap();
        library.favorite(a.id, true).unwrap();
        let b = library.register(&f.book()).unwrap();
        assert_eq!(a.id, b.id);
        assert!(b.favorite);
        assert!(b.last_read_at.is_none());
        assert_eq!(library.list().unwrap().len(), 1);
    }

    #[test]
    fn resume_tracks_page_name_when_pages_are_inserted() {
        let f = Fixture::new();
        let library = f.library();
        let a = library.register(&f.book()).unwrap();
        library
            .save_progress(a.id, 1, "2.png", &ReaderPreferences::default())
            .unwrap();
        let saved = library.get(a.id).unwrap();
        assert_eq!(
            resume_index(&saved, &["0.png".into(), "1.png".into(), "2.png".into()]),
            2
        );
        assert_eq!(resume_index(&saved, &["remaining.png".into()]), 0);
    }

    #[test]
    fn invalid_progress_cannot_overwrite_good_data() {
        let f = Fixture::new();
        let library = f.library();
        let a = library.register(&f.book()).unwrap();
        library
            .save_progress(a.id, 1, "2.png", &ReaderPreferences::default())
            .unwrap();
        assert!(library
            .save_progress(a.id, 99, "wrong", &ReaderPreferences::default())
            .is_err());
        assert!(library
            .save_progress(
                a.id,
                0,
                "1.png",
                &ReaderPreferences {
                    zoom: "bad".into(),
                    ..Default::default()
                }
            )
            .is_err());
        assert_eq!(library.get(a.id).unwrap().last_index, 1);
    }

    #[test]
    fn missing_source_keeps_favorite_and_progress() {
        let f = Fixture::new();
        let library = f.library();
        let a = library.register(&f.book()).unwrap();
        library.favorite(a.id, true).unwrap();
        std::fs::remove_dir_all(f.0.join("pages")).unwrap();
        let missing = library.list().unwrap().pop().unwrap();
        assert!(!missing.available);
        assert!(missing.favorite);
    }

    #[test]
    fn cover_is_a_real_cached_png_and_leaves_current_book_independent() {
        let f = Fixture::new();
        let library = f.library();
        let a = library.register(&f.book()).unwrap();
        let first = library.cover(a.id).unwrap();
        let second = library.cover(a.id).unwrap();
        assert_eq!(first, second);
        let decoded = image::load_from_memory(&first).unwrap();
        assert!(decoded.width() <= 240 && decoded.height() <= 340);
        assert_eq!(decoded.to_rgb8().get_pixel(0, 0).0, [255, 20, 20]);
    }

    #[test]
    fn newer_database_is_rejected_without_resetting_it() {
        let f = Fixture::new();
        let library = f.library();
        library
            .connection
            .lock()
            .unwrap()
            .execute_batch("PRAGMA user_version=2")
            .unwrap();
        drop(library);
        assert!(f.library_result().is_err());
    }
    impl Fixture {
        fn library_result(&self) -> Result<Library, String> {
            Library::open(&self.0.join("data"))
        }
    }
}
