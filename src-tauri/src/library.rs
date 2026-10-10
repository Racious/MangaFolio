//! Persistent local library. Original books are never copied or modified.
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{ipc::Response, State};

use crate::{book, image_pipeline};
#[path = "library_metadata.rs"]
mod metadata;
pub use metadata::*;
#[path = "library_reading.rs"]
mod reading;
pub use reading::*;
#[path = "library_safety.rs"]
mod safety;
pub use safety::*;
#[path = "library_media.rs"]
mod media;
pub use media::*;
#[path = "library_covers.rs"]
mod covers;
pub use covers::*;
fn default_status() -> String {
    "unread".into()
}

pub struct Library {
    connection: Mutex<Connection>,
    covers: PathBuf,
    cover_lock: Mutex<()>,
    directory: PathBuf,
    backup_lock: Mutex<()>,
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

#[derive(Clone, Serialize, Deserialize, Debug)]
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
    #[serde(default)]
    pub source_title: String,
    #[serde(default)]
    pub custom_title: String,
    #[serde(default)]
    pub series: String,
    #[serde(default)]
    pub volume: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "default_status")]
    pub reading_status: String,
    #[serde(default)]
    pub status_manual: bool,
    #[serde(default)]
    pub tags: Vec<Tag>,
}

const COLUMNS: &str = "id, path, title, format, page_count, favorite, last_index, last_page_name, last_read_at, preferences, custom_title, series, volume, notes, reading_status, status_manual, COALESCE((SELECT json_group_array(json_object('id',tags.id,'name',tags.name)) FROM tags JOIN book_tags ON tags.id=book_tags.tag_id WHERE book_tags.book_id=books.id),'[]')";
fn db_error(e: impl std::fmt::Display) -> String {
    format!("書庫資料操作失敗：{e}")
}
pub(crate) fn now() -> i64 {
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
    let source_title: String = row.get(2)?;
    let custom_title: String = row.get(10)?;
    let tags: String = row.get(16)?;
    let tags = serde_json::from_str(&tags).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(16, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(LibraryBook {
        id: row.get(0)?,
        available: Path::new(&path).exists(),
        path,
        title: if custom_title.is_empty() {
            source_title.clone()
        } else {
            custom_title.clone()
        },
        source_title,
        custom_title,
        series: row.get(11)?,
        volume: row.get(12)?,
        notes: row.get(13)?,
        reading_status: row.get(14)?,
        status_manual: row.get(15)?,
        tags,
        format: row.get(3)?,
        page_count,
        favorite: row.get(5)?,
        last_index: last_index.min(page_count.saturating_sub(1)),
        last_page_name: row.get(7)?,
        last_read_at: row.get(8)?,
        preferences,
    })
}

// Identity keys do not change the persisted/displayed path format. Available
// sources resolve aliases; offline Windows paths still match verbatim spellings.
fn source_key(path: &str) -> String {
    // Do not resolve other Windows device namespaces into ordinary disk keys.
    if path.starts_with(r"\\.\") || (path.starts_with(r"\\?\") && windows_source_key(path) == path)
    {
        return path.to_owned();
    }
    let resolved = Path::new(path).canonicalize().ok();
    let value = resolved
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned());
    windows_source_key(&value)
}

fn windows_source_key(value: &str) -> String {
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        let mut parts = unc.split('\\');
        if parts.next().is_some_and(|server| !server.is_empty())
            && parts.next().is_some_and(|share| !share.is_empty())
        {
            return format!(r"\\{}", unc);
        }
    }
    if let Some(disk) = value.strip_prefix(r"\\?\") {
        let bytes = disk.as_bytes();
        if bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'\\'
        {
            return disk.to_owned();
        }
    }
    value.to_owned()
}

fn source_id(
    connection: &Connection,
    path: &str,
    exclude: Option<i64>,
) -> Result<Option<i64>, String> {
    let key = source_key(path);
    let mut statement = connection
        .prepare("SELECT id,path FROM books ORDER BY id")
        .map_err(db_error)?;
    let rows = statement
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(db_error)?;
    let mut matched = None;
    for row in rows {
        let (id, stored) = row.map_err(db_error)?;
        if Some(id) != exclude && source_key(&stored) == key {
            if matched.is_some() {
                return Err(
                    "來源衝突：多筆書籍指向同一來源，未修改任何紀錄；請先確認書庫資料。".into(),
                );
            }
            matched = Some(id);
        }
    }
    Ok(matched)
}

impl Library {
    pub fn open(directory: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(directory).map_err(db_error)?;
        let mut connection =
            Connection::open(directory.join("library.sqlite3")).map_err(db_error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(db_error)?;
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(db_error)?;
        if version > 5 {
            return Err("書庫來自較新版本，請更新 MangaFolio；原始資料已保留。".into());
        }
        if (1..=4).contains(&version) {
            safety::upgrade_snapshot(&connection, directory)?;
        }
        connection
            .execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
            .map_err(db_error)?;
        if version == 0 {
            connection.execute_batch("
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
        }
        if version < 2 {
            metadata::migrate(&mut connection)?;
        }
        if version == 2 {
            reading::migrate(&mut connection)?;
        }
        if version < 4 {
            safety::migrate(&mut connection, directory)?;
        }
        if version < 5 {
            media::migrate(&mut connection)?;
        }
        let covers = directory.join("covers");
        std::fs::create_dir_all(&covers).map_err(db_error)?;
        Ok(Self {
            connection: Mutex::new(connection),
            covers,
            cover_lock: Mutex::new(()),
            directory: directory.to_path_buf(),
            backup_lock: Mutex::new(()),
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
        self.register_selected(book, None)
    }

    pub(crate) fn register_selected(
        &self,
        book: &book::Book,
        selected: Option<i64>,
    ) -> Result<LibraryBook, String> {
        self.register_outcome(book, selected).map(|(book, _)| book)
    }

    pub(crate) fn register_outcome(
        &self,
        book: &book::Book,
        selected: Option<i64>,
    ) -> Result<(LibraryBook, bool), String> {
        let path = book
            .source_path()
            .canonicalize()
            .map_err(db_error)?
            .to_string_lossy()
            .into_owned();
        let preferences = serde_json::to_string(&ReaderPreferences::default()).map_err(db_error)?;
        let mut guard = self.connection.lock().map_err(db_error)?;
        let connection = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_error)?;
        if let Some(id) = selected {
            let stored: String = connection
                .query_row("SELECT path FROM books WHERE id=?1", [id], |r| r.get(0))
                .optional()
                .map_err(db_error)?
                .ok_or_else(|| "選取的書籍已不存在。".to_string())?;
            if source_key(&stored) != source_key(&path) {
                return Err("來源衝突：選取書籍與實際開啟來源不一致，未修改任何紀錄。".into());
            }
        }
        let matched = source_id(&connection, &path, None)?;
        if selected.is_some() && matched != selected {
            return Err("來源衝突：無法保留選取的書籍 ID，未修改任何紀錄。".into());
        }
        let alias_id = matched
            .map(|id| {
                connection
                    .query_row("SELECT path FROM books WHERE id=?1", [id], |r| {
                        r.get::<_, String>(0)
                    })
                    .map(|stored| (stored != path).then_some(id))
                    .map_err(db_error)
            })
            .transpose()?
            .flatten();
        if let Some(id) = alias_id {
            connection
                .execute(
                    "UPDATE books SET title=?1,format=?2,page_count=?3 WHERE id=?4",
                    params![book.title, book.format(), book.len(), id],
                )
                .map_err(db_error)?;
            metadata::reconcile_source(&connection, id, &book.page_names())?;
            let updated = connection
                .query_row(
                    &format!("SELECT {COLUMNS} FROM books WHERE id=?1"),
                    [id],
                    row_book,
                )
                .map_err(db_error)?;
            connection.commit().map_err(db_error)?;
            return Ok((updated, false));
        }
        connection.execute("INSERT INTO books(path,title,format,page_count,preferences,created_at)
            VALUES(?1,?2,?3,?4,?5,?6)
            ON CONFLICT(path) DO UPDATE SET title=excluded.title,format=excluded.format,page_count=excluded.page_count",
            params![path, book.title, book.format(), book.len(), preferences, now()]).map_err(db_error)?;
        let id: i64 = connection
            .query_row("SELECT id FROM books WHERE path=?1", [&path], |r| r.get(0))
            .map_err(db_error)?;
        metadata::reconcile_source(&connection, id, &book.page_names())?;
        let updated = connection
            .query_row(
                &format!("SELECT {COLUMNS} FROM books WHERE path=?1"),
                [path],
                row_book,
            )
            .map_err(db_error)?;
        connection.commit().map_err(db_error)?;
        Ok((updated, matched.is_none()))
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
        let mut connection = self.connection.lock().map_err(db_error)?;
        let tx = connection.transaction().map_err(db_error)?;
        let count: Option<usize> = tx
            .query_row("SELECT page_count FROM books WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()
            .map_err(db_error)?;
        let count = count
            .filter(|count| index < *count)
            .ok_or_else(|| "書籍不存在或頁碼超出範圍。".to_string())?;
        let timestamp = now();
        let status = metadata::automatic_status(index, count, Some(timestamp), preferences);
        let preferences = serde_json::to_string(preferences).map_err(db_error)?;
        tx.execute(
            "UPDATE books SET last_index=?1,last_page_name=?2,last_read_at=?3,preferences=?4,
            reading_status=CASE WHEN status_manual=1 THEN reading_status ELSE ?5 END WHERE id=?6",
            params![index, page_name, timestamp, preferences, status, id],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
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
        let _guard = self.cover_lock.lock().map_err(db_error)?;
        let entry = self.get(id)?;
        if let Some(bytes) = self.custom_cover(id)? { return Ok(bytes); }
        if entry.format == "video" { return self.video_cover(id, Path::new(&entry.path)); }
        let opened = book::open(&entry.path)?;
        let source = Path::new(&entry.path);
        let first = if source.is_dir() {
            source.join(&opened.book.page_names()[0])
        } else {
            source.to_path_buf()
        };
        let metadata = std::fs::metadata(first).map_err(db_error)?;
        let stamp = format!(
            "{}-{}-{}",
            entry.path,
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryBackup {
    pub application: String,
    pub version: u32,
    #[serde(default)]
    pub tags: Vec<Tag>,
    pub books: Vec<LibraryBook>,
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
    #[serde(default)]
    pub custom_covers: Vec<CustomCover>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub added: usize,
    pub skipped: usize,
}

const MAX_BACKUP_BYTES: u64 = 64 * 1024 * 1024;
const MAX_BACKUP_BOOKS: usize = 10_000;

impl Library {
    pub fn remove(&self, ids: &[i64]) -> Result<(), String> {
        validate_ids(ids)?;
        let _cover_guard = self.cover_lock.lock().map_err(db_error)?;
        let mut connection = self.connection.lock().map_err(db_error)?;
        let transaction = connection.transaction().map_err(db_error)?;
        for id in ids {
            if transaction
                .execute("DELETE FROM books WHERE id=?1", [id])
                .map_err(db_error)?
                != 1
            {
                return Err("部分書籍已不存在，未移除任何項目；請重新載入。".into());
            }
        }
        transaction.commit().map_err(db_error)?;
        // Only discard our own cache; never delete a source. IDs are not reused.
        for id in ids {
            for extension in ["png", "stamp", "tmp"] {
                let _ = std::fs::remove_file(self.covers.join(format!("{id}.{extension}")));
            }
        }
        Ok(())
    }

    pub fn favorite_many(&self, ids: &[i64], favorite: bool) -> Result<(), String> {
        validate_ids(ids)?;
        let mut connection = self.connection.lock().map_err(db_error)?;
        let transaction = connection.transaction().map_err(db_error)?;
        for id in ids {
            if transaction
                .execute(
                    "UPDATE books SET favorite=?1 WHERE id=?2",
                    params![favorite, id],
                )
                .map_err(db_error)?
                != 1
            {
                return Err("部分書籍已不存在，未修改任何收藏；請重新載入。".into());
            }
        }
        transaction.commit().map_err(db_error)
    }

    pub fn relink(&self, id: i64, book: &book::Book) -> Result<LibraryBook, String> {
        let path = book
            .source_path()
            .canonicalize()
            .map_err(db_error)?
            .to_string_lossy()
            .into_owned();
        let _cover_guard = self.cover_lock.lock().map_err(db_error)?;
        let mut connection = self.connection.lock().map_err(db_error)?;
        let transaction = connection.transaction().map_err(db_error)?;
        let saved = transaction
            .query_row(
                &format!("SELECT {COLUMNS} FROM books WHERE id=?1"),
                [id],
                row_book,
            )
            .map_err(db_error)?;
        if saved.format == "video" {
            return Err("影片來源不能改成漫畫。".into());
        }
        let duplicate = source_id(&transaction, &path, Some(id))?;
        if duplicate.is_some() {
            return Err("這個來源已在書庫中，請選擇其他來源。".into());
        }
        let pages = book.page_names();
        let index = resume_index(&saved, &pages);
        let page_name = saved.last_page_name.as_ref().map(|_| pages[index].clone());
        transaction.execute("UPDATE books SET path=?1,title=?2,format=?3,page_count=?4,last_index=?5,last_page_name=?6 WHERE id=?7", params![path, book.title, book.format(), book.len(), index, page_name, id]).map_err(db_error)?;
        metadata::reconcile_source(&transaction, id, &pages)?;
        let updated = transaction
            .query_row(
                &format!("SELECT {COLUMNS} FROM books WHERE id=?1"),
                [id],
                row_book,
            )
            .map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(updated)
    }

    pub fn backup_json(&self) -> Result<Vec<u8>, String> {
        let mut guard = self.connection.lock().map_err(db_error)?;
        let connection = guard.transaction().map_err(db_error)?;
        let books = {
            let mut query = connection
                .prepare(&format!(
                    "SELECT {COLUMNS} FROM books ORDER BY last_read_at DESC,id DESC"
                ))
                .map_err(db_error)?;
            let rows = query.query_map([], row_book).map_err(db_error)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(db_error)?
        };
        let tags = metadata::tags_from_connection(&connection)?;
        if books.len() > MAX_BACKUP_BOOKS {
            return Err("備份最多支援 10,000 本書。".into());
        }
        let bytes = serde_json::to_vec_pretty(&LibraryBackup {
            application: "MangaFolio".into(),
            version: 4,
            tags,
            books,
            bookmarks: reading::bookmarks_from(&connection, None)?,
            custom_covers: covers::custom_covers_from(&connection)?,
        })
        .map_err(db_error)?;
        if bytes.len() as u64 > MAX_BACKUP_BYTES {
            return Err("備份超過 64 MiB，無法匯出；原始資料與封面已保留。".into());
        }
        connection.commit().map_err(db_error)?;
        Ok(bytes)
    }

    pub fn export_to(&self, path: &Path) -> Result<(), String> {
        let result = self.write_export(path).and_then(|()| {
            self.connection
                .lock()
                .map_err(db_error)?
                .execute(
                    "UPDATE backup_settings SET last_success=?1,last_error='' WHERE id=1",
                    [now()],
                )
                .map_err(|e| format!("備份已建立，但成功狀態更新失敗：{e}"))?;
            Ok(())
        });
        if let Err(error) = &result {
            if let Ok(conn) = self.connection.lock() {
                let _ = conn.execute(
                    "UPDATE backup_settings SET last_error=?1 WHERE id=1",
                    [error],
                );
            }
        }
        result
    }
    fn write_export(&self, path: &Path) -> Result<(), String> {
        use std::io::Write;
        let bytes = self.backup_json()?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("無法建立備份（不覆寫既有檔案，請選擇新檔名）：{e}"))?;
        let result = file.write_all(&bytes).and_then(|_| file.sync_all());
        if let Err(error) = result {
            drop(file);
            let _ = std::fs::remove_file(path);
            return Err(db_error(error));
        }
        Ok(())
    }

    fn parse_backup(bytes: &[u8]) -> Result<LibraryBackup, String> {
        if bytes.len() as u64 > MAX_BACKUP_BYTES {
            return Err("備份超過 64 MiB。".into());
        }
        // Read the version before strict payload parsing so future fields still
        // yield an explicit version error rather than a generic format error.
        #[derive(Deserialize)]
        struct BackupHeader {
            version: u64,
        }
        let header: BackupHeader =
            serde_json::from_slice(bytes).map_err(|_| "備份格式無效。".to_string())?;
        if header.version > 4 {
            return Err("不支援較新的備份版本，未還原任何項目。".into());
        }
        let mut backup: LibraryBackup =
            serde_json::from_slice(bytes).map_err(|_| "備份格式無效。".to_string())?;
        if backup.application != "MangaFolio"
            || ![1, 2, 3, 4].contains(&backup.version)
            || backup.books.len() > MAX_BACKUP_BOOKS
        {
            return Err("不支援的備份格式、版本或書籍數量。".into());
        }
        if backup.version == 1 {
            backup.tags.clear();
        }
        if backup.tags.len() > 1000 {
            return Err("備份標籤過多。".into());
        }
        let mut tag_names = std::collections::HashSet::new();
        for tag in &backup.tags {
            if !tag_names.insert(metadata::tag_key(&tag.name)?) {
                return Err("備份標籤重複。".into());
            }
        }
        let mut paths = std::collections::HashSet::new();
        for book in &mut backup.books {
            if book.format == "video" && (book.page_count != 1 || book.last_index != 0 || book.last_page_name.is_some()) {
                return Err("備份影片資料無效，不支援漫畫頁碼或進度。".into());
            }
            if backup.version == 1 {
                book.source_title = book.title.clone();
                book.custom_title.clear();
                book.series.clear();
                book.volume.clear();
                book.notes.clear();
                book.tags.clear();
                book.status_manual = false;
                book.reading_status = metadata::automatic_status(
                    book.last_index,
                    book.page_count,
                    book.last_read_at,
                    &book.preferences,
                )
                .into();
            } else {
                metadata::BookDetails {
                    custom_title: book.custom_title.clone(),
                    series: book.series.clone(),
                    volume: book.volume.clone(),
                    notes: book.notes.clone(),
                }
                .validate()?;
                metadata::valid_status(&book.reading_status)?;
                if book.source_title.is_empty() {
                    return Err("備份缺少來源名稱。".into());
                }
                metadata::valid_text(&book.source_title, 4096)?;
                if book.tags.len() > 100 {
                    return Err("備份書籍標籤過多。".into());
                }
                let mut assigned = std::collections::HashSet::new();
                for tag in &book.tags {
                    let key = metadata::tag_key(&tag.name)?;
                    if !tag_names.contains(&key) || !assigned.insert(key) {
                        return Err("備份含未知或重複標籤關聯。".into());
                    }
                }
            }
            book.preferences.validate()?;
            let path = book.path.as_bytes();
            let absolute = Path::new(&book.path).is_absolute()
                || (path.len() > 2
                    && path[0].is_ascii_alphabetic()
                    && path[1] == b':'
                    && [b'/', b'\\'].contains(&path[2]))
                || book.path.starts_with("\\\\");
            if !absolute
                || book.path.contains('\0')
                || book.path.len() > 32_768
                || book.title.is_empty()
                || book.title.len() > 4096
                || !["folder", "cbz", "video"].contains(&book.format.as_str())
                || book.page_count == 0
                || book.page_count > 1_000_000
                || book.last_index >= book.page_count
                || book
                    .last_page_name
                    .as_ref()
                    .is_some_and(|s| s.len() > 32_768)
                || book.last_read_at.is_some_and(|t| t < 0)
                || !paths.insert(source_key(&book.path))
            {
                return Err("備份含無效或重複的書籍資料，未還原任何項目。".into());
            }
        }
        reading::validate_backup_bookmarks(&backup)?;
        covers::validate_covers(&backup)?;
        Ok(backup)
    }

    pub fn restore_json(&self, bytes: &[u8]) -> Result<RestoreResult, String> {
        let backup = Self::parse_backup(bytes)?;
        let mut connection = self.connection.lock().map_err(db_error)?;
        let transaction = connection.transaction().map_err(db_error)?;
        let mut tag_ids = std::collections::HashMap::new();
        for tag in backup.tags {
            let key = metadata::tag_key(&tag.name)?;
            transaction.execute("INSERT INTO tags(name,name_key) VALUES(?1,?2) ON CONFLICT(name_key) DO NOTHING",params![tag.name.trim(),key]).map_err(db_error)?;
            let id: i64 = transaction
                .query_row("SELECT id FROM tags WHERE name_key=?1", [&key], |r| {
                    r.get(0)
                })
                .map_err(db_error)?;
            tag_ids.insert(key, id);
        }
        let count: i64 = transaction
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .map_err(db_error)?;
        if count > 1000 {
            return Err("合併後標籤超過 1000，未還原任何項目。".into());
        }
        let mut result = RestoreResult {
            added: 0,
            skipped: 0,
        };
        // Resolve existing paths once, rather than scanning/canonicalizing the
        // whole library for every item in a large backup.
        let stored_paths: std::collections::HashSet<String> = {
            let mut statement = transaction
                .prepare("SELECT path FROM books")
                .map_err(db_error)?;
            let rows = statement
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(db_error)?;
            rows.collect::<Result<_, _>>().map_err(db_error)?
        };
        let mut existing_keys: std::collections::HashSet<String> =
            stored_paths.iter().map(|p| source_key(p)).collect();
        let mut bookmark_map = std::collections::HashMap::<i64, Vec<&Bookmark>>::new();
        let cover_map: std::collections::HashMap<_,_> = backup.custom_covers.iter().map(|c|(c.book_id,&c.data)).collect();
        for bookmark in &backup.bookmarks {
            bookmark_map
                .entry(bookmark.book_id)
                .or_default()
                .push(bookmark);
        }
        for book in backup.books {
            // Compare identities while preserving the supplied storage path spelling.
            let key = source_key(&book.path);
            if existing_keys.contains(&key) && !stored_paths.contains(&book.path) {
                result.skipped += 1;
                continue;
            }
            existing_keys.insert(key);
            // New IDs are generated; existing sources keep their current metadata.
            let preferences = serde_json::to_string(&book.preferences).map_err(db_error)?;
            let added = transaction.execute("INSERT INTO books(path,title,format,page_count,favorite,last_index,last_page_name,last_read_at,preferences,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(path) DO NOTHING", params![book.path, book.source_title, book.format, book.page_count, book.favorite, book.last_index, book.last_page_name, book.last_read_at, preferences, now()]).map_err(db_error)?;
            if added == 1 {
                let id = transaction.last_insert_rowid();
                if let Some(data) = cover_map.get(&book.id) {
                    transaction.execute("INSERT INTO custom_covers(book_id,data) VALUES(?1,?2)",params![id,data]).map_err(db_error)?;
                }
                transaction.execute("UPDATE books SET custom_title=?1,series=?2,volume=?3,notes=?4,reading_status=?5,status_manual=?6 WHERE id=?7",params![book.custom_title.trim(),book.series.trim(),book.volume.trim(),book.notes,book.reading_status,book.status_manual,id]).map_err(db_error)?;
                for bookmark in bookmark_map.get(&book.id).into_iter().flatten() {
                    reading::insert_bookmark(&transaction, id, bookmark)?;
                }
                for tag in book.tags {
                    let key = metadata::tag_key(&tag.name)?;
                    transaction
                        .execute(
                            "INSERT INTO book_tags(book_id,tag_id) VALUES(?1,?2)",
                            params![id, tag_ids[&key]],
                        )
                        .map_err(db_error)?;
                }
                result.added += 1;
            } else {
                result.skipped += 1;
            }
        }
        transaction.commit().map_err(db_error)?;
        Ok(result)
    }
}

fn validate_ids(ids: &[i64]) -> Result<(), String> {
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    if ids.is_empty()
        || ids.len() > MAX_BACKUP_BOOKS
        || ids.iter().any(|id| *id <= 0)
        || unique.len() != ids.len()
    {
        return Err("請選擇 1 至 10,000 本不重複的書籍。".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn remove_library_books(ids: Vec<i64>, app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || app.state::<Library>().remove(&ids))
        .await
        .map_err(db_error)?
}

#[tauri::command]
pub async fn favorite_library_books(
    ids: Vec<i64>,
    favorite: bool,
    app: tauri::AppHandle,
) -> Result<(), String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Library>().favorite_many(&ids, favorite)
    })
    .await
    .map_err(db_error)?
}

#[tauri::command]
pub async fn relink_library_book(
    id: i64,
    path: String,
    app: tauri::AppHandle,
) -> Result<LibraryBook, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Library>().relink_source(id, &path)
    })
    .await
    .map_err(db_error)?
}

#[tauri::command]
pub async fn export_library_backup(path: String, app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || app.state::<Library>().export_to(Path::new(&path)))
        .await
        .map_err(db_error)?
}

#[tauri::command]
pub async fn restore_library_backup(
    path: String,
    app: tauri::AppHandle,
) -> Result<RestoreResult, String> {
    use std::io::Read;
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(db_error)?
            .take(MAX_BACKUP_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(db_error)?;
        app.state::<Library>().restore_json(&bytes)
    })
    .await
    .map_err(db_error)?
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
    fn automatic_status_matches_final_spreads_and_preserves_manual_states() {
        for count in 1..=8 {
            let f = Fixture::new();
            std::fs::remove_dir_all(f.0.join("pages")).unwrap();
            std::fs::create_dir(f.0.join("pages")).unwrap();
            for i in 0..count {
                image::RgbImage::from_pixel(20, 30, image::Rgb([20, 30, 40]))
                    .save(f.0.join("pages").join(format!("{i:03}.png")))
                    .unwrap();
            }
            let library = f.library();
            let b = library.register(&f.book()).unwrap();
            for mode in ["single", "double"] {
                for cover in [false, true] {
                    let prefs = ReaderPreferences {
                        page_mode: mode.into(),
                        double_cover: cover,
                        ..Default::default()
                    };
                    let last_start = if mode == "single" || count == 1 {
                        count - 1
                    } else if (cover && count % 2 == 0) || (!cover && count % 2 == 1) {
                        count - 1
                    } else {
                        count - 2
                    };
                    library.set_status(&[b.id], "auto").unwrap();
                    for index in 0..count {
                        library
                            .save_progress(b.id, index, &format!("{index:03}.png"), &prefs)
                            .unwrap();
                        let saved = library.get(b.id).unwrap();
                        assert_eq!(
                            saved.last_index, index,
                            "spread-start contract must remain unchanged"
                        );
                        assert_eq!(
                            saved.reading_status,
                            if index >= last_start {
                                "read"
                            } else {
                                "reading"
                            },
                            "count={count} mode={mode} cover={cover} index={index}"
                        );
                        library.set_status(&[b.id], "auto").unwrap();
                        assert_eq!(
                            library.get(b.id).unwrap().reading_status,
                            saved.reading_status
                        );
                    }
                    for manual in ["read", "unread"] {
                        library.set_status(&[b.id], manual).unwrap();
                        library
                            .save_progress(
                                b.id,
                                last_start,
                                &format!("{last_start:03}.png"),
                                &prefs,
                            )
                            .unwrap();
                        assert_eq!(library.get(b.id).unwrap().reading_status, manual);
                    }
                }
            }
        }
    }

    #[test]
    fn v1_backup_last_double_spread_derives_completed_status() {
        let f = Fixture::new();
        let library = f.library();
        let mut b = library.register(&f.book()).unwrap();
        b.path =
            f.0.join("offline-double.cbz")
                .to_string_lossy()
                .into_owned();
        b.page_count = 6;
        b.last_index = 4;
        b.last_read_at = Some(123);
        b.preferences.page_mode = "double".into();
        let backup = LibraryBackup {
            bookmarks: vec![],
            custom_covers: vec![],
            application: "MangaFolio".into(),
            version: 1,
            tags: vec![],
            books: vec![b],
        };
        library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .unwrap();
        let saved = library
            .list()
            .unwrap()
            .into_iter()
            .find(|b| b.last_read_at == Some(123))
            .unwrap();
        assert_eq!(saved.reading_status, "read");
        assert_eq!(saved.last_index, 4);
    }

    #[test]
    fn source_changes_recompute_auto_status_without_losing_metadata() {
        let f = Fixture::new();
        let library = f.library();
        let b = library.register(&f.book()).unwrap();
        let prefs = ReaderPreferences::default();
        library.favorite(b.id, true).unwrap();
        let details = BookDetails {
            custom_title: "Custom".into(),
            series: "Series".into(),
            volume: "2".into(),
            notes: "Notes".into(),
        };
        library.edit_details(b.id, &details).unwrap();
        let tag = library.create_tag("Tag").unwrap();
        library.assign_tag(&[b.id], tag.id, true).unwrap();
        library.save_progress(b.id, 2, "10.png", &prefs).unwrap();
        let original = library.get(b.id).unwrap();
        assert_eq!(original.reading_status, "read");
        image::RgbImage::from_pixel(20, 30, image::Rgb([20, 30, 40]))
            .save(f.0.join("pages/11.png"))
            .unwrap();
        library.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_reconcile BEFORE UPDATE OF reading_status ON books BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
        assert!(library.register(&f.book()).is_err());
        assert_eq!(
            serde_json::to_value(library.get(b.id).unwrap()).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
        library
            .connection
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_reconcile;")
            .unwrap();
        let expanded = library.register(&f.book()).unwrap();
        assert_eq!(
            (
                expanded.page_count,
                expanded.last_index,
                expanded.reading_status.as_str()
            ),
            (4, 2, "reading")
        );
        // Insert ahead of the saved page: registration must resume by name.
        std::fs::copy(f.0.join("pages/1.png"), f.0.join("pages/0.png")).unwrap();
        let relocated = library.register(&f.book()).unwrap();
        assert_eq!(
            (relocated.last_index, relocated.last_page_name.as_deref()),
            (3, Some("10.png"))
        );
        std::fs::create_dir(f.0.join("short")).unwrap();
        for name in ["1.png", "2.png", "10.png"] {
            std::fs::copy(f.0.join("pages").join(name), f.0.join("short").join(name)).unwrap();
        }
        let short = book::open(f.0.join("short").to_str().unwrap())
            .unwrap()
            .book;
        library.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_reconcile BEFORE UPDATE OF reading_status ON books BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
        assert!(library.relink(b.id, &short).is_err());
        assert_eq!(
            serde_json::to_value(library.get(b.id).unwrap()).unwrap(),
            serde_json::to_value(&relocated).unwrap()
        );
        library
            .connection
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_reconcile;")
            .unwrap();
        let linked = library.relink(b.id, &short).unwrap();
        assert_eq!(
            (
                linked.page_count,
                linked.last_index,
                linked.reading_status.as_str()
            ),
            (3, 2, "read")
        );
        for saved in [expanded, relocated, linked] {
            assert_eq!(saved.id, original.id);
            assert_eq!(saved.favorite, original.favorite);
            assert_eq!(saved.last_read_at, original.last_read_at);
            assert_eq!(saved.preferences, original.preferences);
            assert_eq!(saved.custom_title, details.custom_title);
            assert_eq!(saved.series, details.series);
            assert_eq!(saved.volume, details.volume);
            assert_eq!(saved.notes, details.notes);
            assert_eq!(saved.tags[0], tag);
        }
        // Missing saved page: clamp to the shorter source's valid index.
        std::fs::remove_file(f.0.join("short/10.png")).unwrap();
        let shrunk = library
            .register(
                &book::open(f.0.join("short").to_str().unwrap())
                    .unwrap()
                    .book,
            )
            .unwrap();
        assert_eq!(
            (
                shrunk.last_index,
                shrunk.last_page_name.as_deref(),
                shrunk.reading_status.as_str()
            ),
            (1, Some("2.png"), "read")
        );
        for manual in ["read", "unread"] {
            library.set_status(&[b.id], manual).unwrap();
            assert_eq!(
                library.relink(b.id, &f.book()).unwrap().reading_status,
                manual
            );
            assert_eq!(library.register(&f.book()).unwrap().reading_status, manual);
        }
    }

    #[test]
    fn large_offline_library_restores_checks_sources_and_exports() {
        let f = Fixture::new();
        let library = f.library();
        let template = library.register(&f.book()).unwrap();
        let books = (0..9_999)
            .map(|i| {
                let mut book = template.clone();
                book.path =
                    f.0.join(format!("offline-{i}.cbz"))
                        .to_string_lossy()
                        .into_owned();
                book.title = format!("Book {i}");
                book.source_title = book.title.clone();
                book
            })
            .collect();
        let backup = LibraryBackup {
            bookmarks: vec![],
            custom_covers: vec![],
            application: "MangaFolio".into(),
            version: 2,
            tags: vec![],
            books,
        };
        let started = std::time::Instant::now();
        assert_eq!(
            library
                .restore_json(&serde_json::to_vec(&backup).unwrap())
                .unwrap()
                .added,
            9_999
        );
        let listed = library.list().unwrap();
        assert_eq!(listed.len(), 10_000);
        assert_eq!(listed.iter().filter(|book| book.available).count(), 1);
        let exported: LibraryBackup =
            serde_json::from_slice(&library.backup_json().unwrap()).unwrap();
        assert_eq!(exported.books.len(), 10_000);
        eprintln!(
            "10,000 isolated entries restore/source checks/export: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn failed_v1_migration_rolls_back_columns_and_version() {
        let f = Fixture::new();
        let directory = f.0.join("migration-failure");
        std::fs::create_dir(&directory).unwrap();
        let conn = Connection::open(directory.join("library.sqlite3")).unwrap();
        conn.execute_batch("CREATE TABLE books(id INTEGER PRIMARY KEY AUTOINCREMENT,path TEXT NOT NULL UNIQUE,title TEXT NOT NULL,format TEXT NOT NULL,page_count INTEGER NOT NULL,favorite INTEGER NOT NULL DEFAULT 0,last_index INTEGER NOT NULL DEFAULT 0,last_page_name TEXT,last_read_at INTEGER,preferences TEXT NOT NULL,created_at INTEGER NOT NULL);CREATE TABLE tags(id INTEGER);PRAGMA user_version=1;").unwrap();
        assert!(Library::open(&directory).is_err());
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM pragma_table_info('books') WHERE name='custom_title'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn import_report_distinguishes_added_updated_and_preserves_info() {
        let f = Fixture::new();
        let library = f.library();
        let (first, created) = library.register_outcome(&f.book(), None).unwrap();
        assert!(created);
        library
            .edit_details(
                first.id,
                &BookDetails {
                    custom_title: "保留資訊".into(),
                    ..Default::default()
                },
            )
            .unwrap();
        let (second, created) = library.register_outcome(&f.book(), None).unwrap();
        assert!(!created);
        assert_eq!(first.id, second.id);
        assert_eq!(second.title, "保留資訊");
    }

    #[test]
    fn v1_database_migrates_without_losing_identity_or_progress() {
        let f = Fixture::new();
        let directory = f.0.join("old-data");
        std::fs::create_dir(&directory).unwrap();
        let conn = Connection::open(directory.join("library.sqlite3")).unwrap();
        conn.execute_batch("CREATE TABLE books(id INTEGER PRIMARY KEY AUTOINCREMENT,path TEXT NOT NULL UNIQUE,title TEXT NOT NULL,format TEXT NOT NULL,page_count INTEGER NOT NULL,favorite INTEGER NOT NULL DEFAULT 0,last_index INTEGER NOT NULL DEFAULT 0,last_page_name TEXT,last_read_at INTEGER,preferences TEXT NOT NULL,created_at INTEGER NOT NULL); PRAGMA user_version=1;").unwrap();
        conn.execute("INSERT INTO books(id,path,title,format,page_count,favorite,last_index,last_page_name,last_read_at,preferences,created_at) VALUES(42,?1,'legacy','folder',3,1,1,'2.png',123,?2,1)",params![f.0.join("pages").to_string_lossy(),serde_json::to_string(&ReaderPreferences::default()).unwrap()]).unwrap();
        let double = ReaderPreferences {
            page_mode: "double".into(),
            ..Default::default()
        };
        conn.execute("INSERT INTO books(id,path,title,format,page_count,favorite,last_index,last_page_name,last_read_at,preferences,created_at) VALUES(43,?1,'legacy-double','folder',6,1,4,'5.png',123,?2,1)",params![f.0.join("offline-double").to_string_lossy(),serde_json::to_string(&double).unwrap()]).unwrap();
        drop(conn);
        let library = Library::open(&directory).unwrap();
        assert_eq!(library.get(43).unwrap().reading_status, "read");
        let b = library.get(42).unwrap();
        assert!(b.favorite);
        assert_eq!(b.last_index, 1);
        assert_eq!(b.last_read_at, Some(123));
        assert_eq!(b.reading_status, "reading");
        assert!(b.tags.is_empty());
        assert_eq!(b.source_title, "legacy");
        assert_eq!(
            library.register_selected(&f.book(), Some(42)).unwrap().id,
            42
        );
        assert_eq!(
            library
                .connection
                .lock()
                .unwrap()
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
    }

    #[test]
    fn manual_status_is_independent_and_batch_failures_are_atomic() {
        let f = Fixture::new();
        let library = f.library();
        let b = library.register(&f.book()).unwrap();
        assert_eq!(b.reading_status, "unread");
        library
            .save_progress(b.id, 1, "2.png", &ReaderPreferences::default())
            .unwrap();
        assert_eq!(library.get(b.id).unwrap().reading_status, "reading");
        let before = library.get(b.id).unwrap();
        assert!(library.set_status(&[b.id, b.id + 99], "read").is_err());
        assert_eq!(library.get(b.id).unwrap().reading_status, "reading");
        library.set_status(&[b.id], "unread").unwrap();
        let marked = library.get(b.id).unwrap();
        assert_eq!(marked.last_index, before.last_index);
        assert_eq!(marked.last_read_at, before.last_read_at);
        assert_eq!(marked.preferences, before.preferences);
        library
            .save_progress(b.id, 2, "10.png", &ReaderPreferences::default())
            .unwrap();
        assert_eq!(library.get(b.id).unwrap().reading_status, "unread");
        library.set_status(&[b.id], "auto").unwrap();
        assert_eq!(library.get(b.id).unwrap().reading_status, "read");
        library.set_status(&[b.id], "read").unwrap();
        assert_eq!(library.register(&f.book()).unwrap().reading_status, "read");
    }

    #[test]
    fn tags_and_custom_info_survive_reimport_relink_and_backup() {
        let f = Fixture::new();
        let library = f.library();
        let b = library.register(&f.book()).unwrap();
        let details = BookDetails {
            custom_title: "我的書名".into(),
            series: "系列10".into(),
            volume: "外傳".into(),
            notes: "閱讀備註".into(),
        };
        library.edit_details(b.id, &details).unwrap();
        library.favorite(b.id, true).unwrap();
        library
            .save_progress(b.id, 1, "2.png", &ReaderPreferences::default())
            .unwrap();
        library.set_status(&[b.id], "read").unwrap();
        let tag = library.create_tag("  收藏作品  ").unwrap();
        let unused = library.create_tag("未使用").unwrap();
        library.assign_tag(&[b.id], tag.id, true).unwrap();
        let reimport = library.register(&f.book()).unwrap();
        assert_eq!(reimport.title, details.custom_title);
        assert_eq!(reimport.source_title, "pages");
        assert_eq!(reimport.series, details.series);
        assert_eq!(reimport.tags[0], tag);
        assert_eq!(reimport.reading_status, "read");
        std::fs::rename(f.0.join("pages"), f.0.join("moved")).unwrap();
        let moved = book::open(f.0.join("moved").to_str().unwrap())
            .unwrap()
            .book;
        let linked = library.relink(b.id, &moved).unwrap();
        assert_eq!(linked.id, b.id);
        assert_eq!(linked.title, details.custom_title);
        assert_eq!(linked.source_title, "moved");
        assert!(linked.favorite);
        assert_eq!(linked.last_index, 1);
        assert_eq!(linked.notes, details.notes);
        let bytes = library.backup_json().unwrap();
        library.remove(&[b.id]).unwrap();
        library.delete_tag(tag.id).unwrap();
        library.delete_tag(unused.id).unwrap();
        library.restore_json(&bytes).unwrap();
        let restored = library.list().unwrap().pop().unwrap();
        assert_eq!(restored.title, details.custom_title);
        assert_eq!(restored.tags[0].name, tag.name);
        assert_eq!(library.tags().unwrap().len(), 2);
        assert!(restored.status_manual);
        assert_eq!(restored.reading_status, "read");
        assert_eq!(restored.preferences, linked.preferences);
        library
            .edit_details(
                restored.id,
                &BookDetails {
                    custom_title: "現有資料".into(),
                    ..Default::default()
                },
            )
            .unwrap();
        let merge = library.restore_json(&bytes).unwrap();
        assert_eq!(merge.skipped, 1);
        assert_eq!(library.get(restored.id).unwrap().title, "現有資料");
    }

    #[test]
    fn tag_validation_batches_and_delete_never_delete_sources() {
        let f = Fixture::new();
        let other = Fixture::new();
        let library = f.library();
        let a = library.register(&f.book()).unwrap();
        let b = library.register(&other.book()).unwrap();
        let tag = library.create_tag("Mystery").unwrap();
        assert!(library.create_tag(" mystery ").is_err());
        assert!(library.create_tag("  ").is_err());
        assert!(library.create_tag(&"長".repeat(65)).is_err());
        assert!(library.create_tag("bad\nname").is_err());
        let second = library.create_tag("Nature").unwrap();
        assert!(library.rename_tag(second.id, "MYSTERY").is_err());
        assert!(library.assign_tag(&[a.id, 999999], tag.id, true).is_err());
        assert!(library.get(a.id).unwrap().tags.is_empty());
        library.assign_tag(&[a.id, b.id], tag.id, true).unwrap();
        library.rename_tag(tag.id, "懸疑").unwrap();
        assert_eq!(library.get(a.id).unwrap().tags[0].name, "懸疑");
        library.assign_tag(&[a.id, b.id], tag.id, false).unwrap();
        assert!(library.get(a.id).unwrap().tags.is_empty());
        library.assign_tag(&[a.id], tag.id, true).unwrap();
        library.delete_tag(tag.id).unwrap();
        assert!(library.get(a.id).unwrap().tags.is_empty());
        assert!(f.0.join("pages/1.png").exists());
        assert_eq!(library.list().unwrap().len(), 2);
    }

    #[test]
    fn restore_v1_defaults_and_v2_failures_roll_back_catalog_and_books() {
        let f = Fixture::new();
        let library = f.library();
        let b = library.register(&f.book()).unwrap();
        library
            .save_progress(b.id, 1, "2.png", &ReaderPreferences::default())
            .unwrap();
        let mut backup: LibraryBackup =
            serde_json::from_slice(&library.backup_json().unwrap()).unwrap();
        library.remove(&[b.id]).unwrap();
        backup.version = 1;
        let mut legacy = serde_json::to_value(&backup).unwrap();
        legacy.as_object_mut().unwrap().remove("tags");
        for b in legacy["books"].as_array_mut().unwrap() {
            for field in [
                "sourceTitle",
                "customTitle",
                "series",
                "volume",
                "notes",
                "readingStatus",
                "statusManual",
                "tags",
            ] {
                b.as_object_mut().unwrap().remove(field);
            }
        }
        let bytes = serde_json::to_vec(&legacy).unwrap();
        library.restore_json(&bytes).unwrap();
        let restored = library.list().unwrap().pop().unwrap();
        assert_eq!(restored.reading_status, "reading");
        assert!(restored.custom_title.is_empty());
        library.remove(&[restored.id]).unwrap();
        backup.version = 2;
        backup.tags = vec![Tag {
            id: 1,
            name: "交易測試".into(),
        }];
        backup.books[0].tags = backup.tags.clone();
        library.connection.lock().unwrap().execute_batch("CREATE TRIGGER reject_relation BEFORE INSERT ON book_tags BEGIN SELECT RAISE(ABORT,'test'); END;").unwrap();
        let before = library.backup_json().unwrap();
        assert!(library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .is_err());
        assert_eq!(library.backup_json().unwrap(), before);
        library
            .connection
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER reject_relation;")
            .unwrap();
        backup.books[0].tags[0].name = "未知標籤".into();
        assert!(library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .is_err());
        assert_eq!(library.backup_json().unwrap(), before);
        backup.version = 5;
        assert!(library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .is_err());
        assert_eq!(library.backup_json().unwrap(), before);
    }

    #[test]
    fn windows_keys_only_convert_absolute_disks_and_complete_unc() {
        for (input, expected) in [
            (r"\\?\C:\books\pages", r"C:\books\pages"),
            (r"\\?\z:\books", r"z:\books"),
            (r"\\?\UNC\server\share\pages", r"\\server\share\pages"),
            (r"\\?\Volume{123}\pages", r"\\?\Volume{123}\pages"),
            (r"\\?\GLOBALROOT\Device\disk", r"\\?\GLOBALROOT\Device\disk"),
            (r"\\.\C:\books", r"\\.\C:\books"),
            (r"\\?\C:relative", r"\\?\C:relative"),
            (r"\\?\UNC\server", r"\\?\UNC\server"),
            (r"\\?\UNC\server\", r"\\?\UNC\server\"),
        ] {
            assert_eq!(windows_source_key(input), expected);
            if input == expected {
                assert_eq!(source_key(input), expected);
            }
        }
    }

    #[test]
    fn selected_source_mismatch_and_sql_failure_leave_data_unchanged() {
        let f = Fixture::new();
        let other = Fixture::new();
        let library = f.library();
        let entry = library.register(&f.book()).unwrap();
        let before = library.backup_json().unwrap();
        assert!(library
            .register_selected(&other.book(), Some(entry.id))
            .unwrap_err()
            .contains("來源衝突"));
        assert_eq!(before, library.backup_json().unwrap());
        library.connection.lock().unwrap().execute_batch(
            "CREATE TRIGGER reject_title BEFORE UPDATE OF title ON books BEGIN SELECT RAISE(ABORT, 'test rejection'); END;"
        ).unwrap();
        assert!(library
            .register_selected(&f.book(), Some(entry.id))
            .is_err());
        assert_eq!(before, library.backup_json().unwrap());
        assert!(library
            .register_selected(&f.book(), Some(entry.id + 99))
            .is_err());
        assert_eq!(before, library.backup_json().unwrap());
    }

    #[test]
    fn restore_alias_merge_duplicates_and_reopen_preserve_state() {
        let f = Fixture::new();
        let library = f.library();
        let entry = library.register(&f.book()).unwrap();
        let prefs = ReaderPreferences {
            direction: "ltr".into(),
            ..Default::default()
        };
        library.favorite(entry.id, true).unwrap();
        library.save_progress(entry.id, 1, "2.png", &prefs).unwrap();
        let mut backup: LibraryBackup =
            serde_json::from_slice(&library.backup_json().unwrap()).unwrap();
        backup.books[0].path = f.0.join("pages/../pages").to_string_lossy().into_owned();
        let bytes = serde_json::to_vec(&backup).unwrap();
        library.favorite(entry.id, false).unwrap();
        assert_eq!(library.restore_json(&bytes).unwrap().skipped, 1);
        assert!(!library.get(entry.id).unwrap().favorite);
        assert_eq!(library.register(&f.book()).unwrap().id, entry.id);
        backup.books.push(entry.clone());
        assert!(library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .is_err());
        assert_eq!(library.list().unwrap().len(), 1);
        library.remove(&[entry.id]).unwrap();
        library.restore_json(&bytes).unwrap();
        let restored = library.list().unwrap().pop().unwrap();
        let reopened = library
            .register_selected(&f.book(), Some(restored.id))
            .unwrap();
        assert_eq!(reopened.id, restored.id);
        assert!(reopened.favorite);
        assert_eq!(reopened.last_index, 1);
        assert_eq!(reopened.last_page_name.as_deref(), Some("2.png"));
        assert_eq!(reopened.preferences, prefs);
        assert_eq!(library.list().unwrap().len(), 1);
    }

    #[test]
    fn offline_restore_reconnect_keeps_identity_and_settings() {
        let f = Fixture::new();
        let library = f.library();
        let entry = library.register(&f.book()).unwrap();
        library.favorite(entry.id, true).unwrap();
        library
            .save_progress(entry.id, 1, "2.png", &ReaderPreferences::default())
            .unwrap();
        let bytes = library.backup_json().unwrap();
        library.remove(&[entry.id]).unwrap();
        std::fs::rename(f.0.join("pages"), f.0.join("offline")).unwrap();
        library.restore_json(&bytes).unwrap();
        let restored = library.list().unwrap().pop().unwrap();
        assert!(!restored.available);
        std::fs::rename(f.0.join("offline"), f.0.join("pages")).unwrap();
        let reopened = library
            .register_selected(&f.book(), Some(restored.id))
            .unwrap();
        assert_eq!(restored.id, reopened.id);
        assert!(reopened.favorite);
        assert_eq!(reopened.last_index, 1);
        assert_eq!(library.list().unwrap().len(), 1);
    }

    #[test]
    fn windows_offline_spellings_merge_and_reject_duplicate_backup() {
        let f = Fixture::new();
        let library = f.library();
        let mut entry = library.register(&f.book()).unwrap();
        library.remove(&[entry.id]).unwrap();
        entry.path = r"Z:\mangafolio-missing\pages".into();
        let mut backup = LibraryBackup {
            bookmarks: vec![],
            custom_covers: vec![],
            application: "MangaFolio".into(),
            version: 1,
            tags: Vec::new(),
            books: vec![entry.clone()],
        };
        library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .unwrap();
        backup.books[0].path = r"\\?\Z:\mangafolio-missing\pages".into();
        assert_eq!(
            library
                .restore_json(&serde_json::to_vec(&backup).unwrap())
                .unwrap()
                .skipped,
            1
        );
        backup.books.push(entry);
        assert!(library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .is_err());
        assert_eq!(
            source_key(r"\\?\UNC\server\share\pages"),
            source_key(r"\\server\share\pages")
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_normal_restore_then_verbatim_open_keeps_id() {
        let f = Fixture::new();
        let library = f.library();
        let entry = library.register(&f.book()).unwrap();
        library.favorite(entry.id, true).unwrap();
        library
            .save_progress(entry.id, 1, "2.png", &ReaderPreferences::default())
            .unwrap();
        let mut backup: LibraryBackup =
            serde_json::from_slice(&library.backup_json().unwrap()).unwrap();
        backup.books[0].path = source_key(&entry.path);
        library.remove(&[entry.id]).unwrap();
        library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .unwrap();
        let restored = library.list().unwrap().pop().unwrap();
        let reopened = library
            .register_selected(&f.book(), Some(restored.id))
            .unwrap();
        assert_eq!(restored.id, reopened.id);
        assert!(reopened.favorite);
        assert_eq!(reopened.last_index, 1);
        assert_eq!(library.list().unwrap().len(), 1);
    }

    #[test]
    fn archive_titles_reimport_preserves_state_and_folder_dots() {
        use std::io::Write;
        let f = Fixture::new();
        let library = f.library();
        for extension in ["zip", "cbz"] {
            let path = f.0.join(format!("Volume.01.{extension}"));
            let mut archive = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            for name in ["1.png", "2.png"] {
                archive
                    .start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                archive
                    .write_all(&std::fs::read(f.0.join("pages").join(name)).unwrap())
                    .unwrap();
            }
            archive.finish().unwrap();
            let opened = book::open(path.to_str().unwrap()).unwrap().book;
            assert_eq!(opened.title, "Volume.01");
            let entry = library.register(&opened).unwrap();
            let prefs = ReaderPreferences {
                direction: "ltr".into(),
                ..Default::default()
            };
            library.favorite(entry.id, true).unwrap();
            library.save_progress(entry.id, 1, "2.png", &prefs).unwrap();
            library
                .connection
                .lock()
                .unwrap()
                .execute(
                    "UPDATE books SET title=?1 WHERE id=?2",
                    params![format!("Volume.01.{extension}"), entry.id],
                )
                .unwrap();
            let before = library.get(entry.id).unwrap();
            let updated = library.register(&opened).unwrap();
            assert_eq!(updated.title, "Volume.01");
            assert_eq!(updated.last_read_at, before.last_read_at);
            assert_eq!(updated.last_page_name, before.last_page_name);
            assert_eq!(entry.id, updated.id);
            assert!(updated.favorite);
            assert_eq!(updated.last_index, 1);
            assert_eq!(updated.preferences, prefs);
        }
        std::fs::rename(f.0.join("pages"), f.0.join("Folder.01")).unwrap();
        for path in [f.0.join("Folder.01"), f.0.join("Folder.01/1.png")] {
            assert_eq!(
                book::open(path.to_str().unwrap()).unwrap().book.title,
                "Folder.01"
            );
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
            .execute_batch("PRAGMA user_version=6")
            .unwrap();
        drop(library);
        assert!(f.library_result().is_err());
    }

    #[test]
    fn batch_mutations_are_atomic_and_never_delete_sources() {
        let f = Fixture::new();
        let library = f.library();
        let entry = library.register(&f.book()).unwrap();
        assert!(library
            .favorite_many(&[entry.id, entry.id + 1], true)
            .is_err());
        assert!(!library.get(entry.id).unwrap().favorite);
        assert!(library.remove(&[entry.id, entry.id + 1]).is_err());
        assert!(library.get(entry.id).is_ok());
        assert!(library.remove(&[entry.id, entry.id]).is_err());
        library.favorite_many(&[entry.id], true).unwrap();
        library.cover(entry.id).unwrap();
        library.remove(&[entry.id]).unwrap();
        assert!(library.list().unwrap().is_empty());
        assert!(f.0.join("pages/1.png").exists());
        assert!(!f.0.join(format!("data/covers/{}.png", entry.id)).exists());
        assert!(library.register(&f.book()).unwrap().id > entry.id);
    }

    #[test]
    fn relink_preserves_identity_preferences_and_matches_page_name() {
        let f = Fixture::new();
        let library = f.library();
        let entry = library.register(&f.book()).unwrap();
        let prefs = ReaderPreferences {
            direction: "ltr".into(),
            ..Default::default()
        };
        library.favorite(entry.id, true).unwrap();
        library.save_progress(entry.id, 1, "2.png", &prefs).unwrap();
        let before = library.get(entry.id).unwrap();
        std::fs::rename(f.0.join("pages"), f.0.join("moved")).unwrap();
        image::RgbImage::new(20, 30)
            .save(f.0.join("moved/0.png"))
            .unwrap();
        let new_book = book::open(f.0.join("moved").to_str().unwrap())
            .unwrap()
            .book;
        let updated = library.relink(entry.id, &new_book).unwrap();
        assert_eq!(updated.id, entry.id);
        assert!(updated.favorite && updated.available);
        assert_eq!(updated.preferences, prefs);
        assert_eq!(updated.last_read_at, before.last_read_at);
        assert_eq!(updated.last_index, 2);
        assert_eq!(updated.last_page_name.as_deref(), Some("2.png"));
    }

    #[test]
    fn relink_conflict_keeps_both_entries_intact() {
        let f = Fixture::new();
        let other = Fixture::new();
        let library = f.library();
        let first = library.register(&f.book()).unwrap();
        let second = library.register(&other.book()).unwrap();
        assert!(library.relink(first.id, &other.book()).is_err());
        assert_eq!(library.get(first.id).unwrap().path, first.path);
        assert_eq!(library.get(second.id).unwrap().path, second.path);
    }

    #[test]
    fn backup_roundtrip_merge_preserves_existing_and_restores_offline_metadata() {
        let f = Fixture::new();
        let library = f.library();
        let entry = library.register(&f.book()).unwrap();
        library.favorite(entry.id, true).unwrap();
        library
            .save_progress(entry.id, 1, "2.png", &ReaderPreferences::default())
            .unwrap();
        let bytes = library.backup_json().unwrap();
        library.favorite(entry.id, false).unwrap();
        let merge = library.restore_json(&bytes).unwrap();
        assert_eq!((merge.added, merge.skipped), (0, 1));
        assert!(!library.get(entry.id).unwrap().favorite);
        library.remove(&[entry.id]).unwrap();
        std::fs::remove_dir_all(f.0.join("pages")).unwrap();
        let result = library.restore_json(&bytes).unwrap();
        assert_eq!((result.added, result.skipped), (1, 0));
        drop(library);
        let restored = f.library().list().unwrap().pop().unwrap();
        assert!(restored.favorite && !restored.available);
        assert_eq!(restored.last_index, 1);
        assert_ne!(restored.id, entry.id);
    }

    #[test]
    fn invalid_backup_cannot_partially_restore_or_change_existing() {
        let f = Fixture::new();
        let other = Fixture::new();
        let library = f.library();
        let mut entries = vec![
            library.register(&f.book()).unwrap(),
            library.register(&other.book()).unwrap(),
        ];
        library.remove(&[entries[1].id]).unwrap();
        entries[0].favorite = true;
        entries[1].preferences.zoom = "bad".into();
        let backup = LibraryBackup {
            bookmarks: vec![],
            custom_covers: vec![],
            application: "MangaFolio".into(),
            version: 1,
            tags: Vec::new(),
            books: entries,
        };
        assert!(library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .is_err());
        assert_eq!(library.list().unwrap().len(), 1);
        assert!(!library.list().unwrap()[0].favorite);
        assert!(library.restore_json(b"{not-json}").is_err());
        assert!(library
            .restore_json(&vec![b' '; MAX_BACKUP_BYTES as usize + 1])
            .is_err());
    }

    #[test]
    fn backup_rejects_future_version_duplicates_and_invalid_paths() {
        let f = Fixture::new();
        let library = f.library();
        let entry = library.register(&f.book()).unwrap();
        let mut backup = LibraryBackup {
            bookmarks: vec![],
            custom_covers: vec![],
            application: "MangaFolio".into(),
            version: 5,
            tags: Vec::new(),
            books: vec![entry.clone()],
        };
        assert!(library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .is_err());
        let mut future = serde_json::to_value(&backup).unwrap();
        future["futureField"] = serde_json::json!({"newContract": true});
        assert!(library
            .restore_json(&serde_json::to_vec(&future).unwrap())
            .err()
            .unwrap()
            .contains("較新的備份版本"));
        backup.version = 1;
        backup.books.push(entry);
        assert!(library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .is_err());
        backup.books.pop();
        backup.books[0].path = "relative/book.cbz".into();
        assert!(library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .is_err());
        assert_eq!(library.list().unwrap().len(), 1);
    }

    #[test]
    fn export_writes_valid_backup_and_refuses_to_overwrite_sources_or_backup() {
        let f = Fixture::new();
        let library = f.library();
        library.register(&f.book()).unwrap();
        let destination = f.0.join("backup.json");
        library.export_to(&destination).unwrap();
        let bytes = std::fs::read(&destination).unwrap();
        assert!(library.export_to(&destination).is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), bytes);
        assert_eq!(library.restore_json(&bytes).unwrap().skipped, 1);
        let source = f.0.join("pages/1.png");
        let original = std::fs::read(&source).unwrap();
        assert!(library.export_to(&source).is_err());
        assert_eq!(std::fs::read(&source).unwrap(), original);
    }
    impl Fixture {
        fn library_result(&self) -> Result<Library, String> {
            Library::open(&self.0.join("data"))
        }
    }
}
