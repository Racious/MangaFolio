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
        self.register_selected(book, None)
    }

    pub(crate) fn register_selected(
        &self,
        book: &book::Book,
        selected: Option<i64>,
    ) -> Result<LibraryBook, String> {
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
            let updated = connection
                .query_row(
                    &format!("SELECT {COLUMNS} FROM books WHERE id=?1"),
                    [id],
                    row_book,
                )
                .map_err(db_error)?;
            connection.commit().map_err(db_error)?;
            return Ok(updated);
        }
        connection.execute("INSERT INTO books(path,title,format,page_count,preferences,created_at)
            VALUES(?1,?2,?3,?4,?5,?6)
            ON CONFLICT(path) DO UPDATE SET title=excluded.title,format=excluded.format,page_count=excluded.page_count",
            params![path, book.title, book.format(), book.len(), preferences, now()]).map_err(db_error)?;
        let updated = connection
            .query_row(
                &format!("SELECT {COLUMNS} FROM books WHERE path=?1"),
                [path],
                row_book,
            )
            .map_err(db_error)?;
        connection.commit().map_err(db_error)?;
        Ok(updated)
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
        let _guard = self.cover_lock.lock().map_err(db_error)?;
        let entry = self.get(id)?;
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
    pub books: Vec<LibraryBook>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub added: usize,
    pub skipped: usize,
}

const MAX_BACKUP_BYTES: u64 = 16 * 1024 * 1024;
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
        let duplicate = source_id(&transaction, &path, Some(id))?;
        if duplicate.is_some() {
            return Err("這個來源已在書庫中，請選擇其他來源。".into());
        }
        let pages = book.page_names();
        let index = resume_index(&saved, &pages);
        let page_name = saved.last_page_name.as_ref().map(|_| pages[index].clone());
        transaction.execute("UPDATE books SET path=?1,title=?2,format=?3,page_count=?4,last_index=?5,last_page_name=?6 WHERE id=?7", params![path, book.title, book.format(), book.len(), index, page_name, id]).map_err(db_error)?;
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
        let books = self.list()?;
        if books.len() > MAX_BACKUP_BOOKS {
            return Err("備份最多支援 10,000 本書。".into());
        }
        let bytes = serde_json::to_vec_pretty(&LibraryBackup {
            application: "MangaFolio".into(),
            version: 1,
            books,
        })
        .map_err(db_error)?;
        if bytes.len() as u64 > MAX_BACKUP_BYTES {
            return Err("備份超過 16 MiB，無法匯出。".into());
        }
        Ok(bytes)
    }

    pub fn export_to(&self, path: &Path) -> Result<(), String> {
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

    pub fn restore_json(&self, bytes: &[u8]) -> Result<RestoreResult, String> {
        if bytes.len() as u64 > MAX_BACKUP_BYTES {
            return Err("備份超過 16 MiB。".into());
        }
        let backup: LibraryBackup =
            serde_json::from_slice(bytes).map_err(|_| "備份格式無效。".to_string())?;
        if backup.application != "MangaFolio"
            || backup.version != 1
            || backup.books.len() > MAX_BACKUP_BOOKS
        {
            return Err("不支援的備份格式、版本或書籍數量。".into());
        }
        let mut paths = std::collections::HashSet::new();
        for book in &backup.books {
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
                || !["folder", "cbz"].contains(&book.format.as_str())
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
        let mut connection = self.connection.lock().map_err(db_error)?;
        let transaction = connection.transaction().map_err(db_error)?;
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
            let added = transaction.execute("INSERT INTO books(path,title,format,page_count,favorite,last_index,last_page_name,last_read_at,preferences,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(path) DO NOTHING", params![book.path, book.title, book.format, book.page_count, book.favorite, book.last_index, book.last_page_name, book.last_read_at, preferences, now()]).map_err(db_error)?;
            if added == 1 {
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
        let opened = book::open(&path)?;
        app.state::<Library>().relink(id, &opened.book)
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
            application: "MangaFolio".into(),
            version: 1,
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
            .execute_batch("PRAGMA user_version=2")
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
            application: "MangaFolio".into(),
            version: 1,
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
            application: "MangaFolio".into(),
            version: 2,
            books: vec![entry.clone()],
        };
        assert!(library
            .restore_json(&serde_json::to_vec(&backup).unwrap())
            .is_err());
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
