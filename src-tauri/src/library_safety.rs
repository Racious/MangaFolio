use super::*;
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
const RECOVERY_FAILURE_PREFIX: &str = "備份狀態恢復／清理失敗";
pub(super) fn migrate(conn: &mut Connection, directory: &Path) -> Result<(), String> {
    let default = std::fs::canonicalize(directory)
        .map_err(db_error)?
        .join("backups");
    let default = if default.exists() {
        std::fs::canonicalize(default).map_err(db_error)?
    } else {
        default
    };
    let tx = conn.transaction().map_err(db_error)?;
    tx.execute_batch(
        "ALTER TABLE backup_settings ADD COLUMN custom_directory TEXT;
        ALTER TABLE backup_settings ADD COLUMN last_directory TEXT NOT NULL DEFAULT '';
        ALTER TABLE backup_settings ADD COLUMN directory_warning TEXT NOT NULL DEFAULT '';
        ALTER TABLE automatic_backups RENAME COLUMN name TO path;",
    )
    .map_err(db_error)?;
    let names = {
        let mut q = tx
            .prepare("SELECT path FROM automatic_backups")
            .map_err(db_error)?;
        let rows = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_error)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db_error)?
    };
    for name in names {
        let path = if owned_name(&name) {
            default.join(&name).to_string_lossy().into_owned()
        } else {
            // Preserve unrecognised records without adopting arbitrary absolute paths.
            format!("unrecognized:{name}")
        };
        tx.execute(
            "UPDATE automatic_backups SET path=?1 WHERE path=?2",
            params![path, name],
        )
        .map_err(db_error)?;
    }
    tx.execute_batch("PRAGMA user_version=4;")
        .map_err(db_error)?;
    tx.commit().map_err(db_error)
}
fn unique_name(prefix: &str, extension: &str) -> String {
    format!(
        "mangafolio-{prefix}-{}-{}-{}.{}",
        now(),
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed),
        extension
    )
}
pub(super) fn upgrade_snapshot(conn: &Connection, directory: &Path) -> Result<(), String> {
    let dir = directory.join("backups");
    std::fs::create_dir_all(&dir).map_err(db_error)?;
    let path = dir.join(unique_name("preupgrade", "sqlite3"));
    let owned = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(db_error)?;
    drop(owned);
    if let Err(e) = conn.backup("main", &path, None) {
        let _ = std::fs::remove_file(&path);
        return Err(format!("升級前安全備份失敗，未升級資料庫：{e}"));
    }
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .and_then(|f| f.sync_all())
        .map_err(db_error)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub enabled: bool,
    pub retention: usize,
    pub last_success: Option<i64>,
    pub last_error: String,
    pub custom_directory: Option<String>,
    pub directory: String,
    pub directory_warning: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestorePreview {
    pub added: usize,
    pub skipped: usize,
    pub conflicts: usize,
    pub unsupported: usize,
    pub issues: Vec<String>,
    pub can_restore: bool,
    pub version: Option<u32>,
}
impl Library {
    pub fn preview_json(&self, bytes: &[u8]) -> Result<RestorePreview, String> {
        let version = serde_json::from_slice::<serde_json::Value>(bytes)
            .ok()
            .and_then(|v| v.get("version").and_then(|v| v.as_u64()))
            .and_then(|v| u32::try_from(v).ok());
        let backup = match Self::parse_backup(bytes) {
            Ok(b) => b,
            Err(error) => {
                return Ok(RestorePreview {
                    added: 0,
                    skipped: 0,
                    conflicts: usize::from(error.contains("重複")),
                    unsupported: usize::from(version.is_some_and(|v| v > 3)),
                    issues: vec![error],
                    can_restore: false,
                    version,
                })
            }
        };
        let conn = self.connection.lock().map_err(db_error)?;
        let mut q = conn.prepare("SELECT path FROM books").map_err(db_error)?;
        let existing = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_error)?;
        let mut keys = std::collections::HashMap::new();
        for path in existing {
            *keys.entry(source_key(&path)).or_insert(0usize) += 1;
        }
        let mut result = RestorePreview {
            added: 0,
            skipped: 0,
            conflicts: 0,
            unsupported: 0,
            issues: vec![],
            can_restore: true,
            version,
        };
        for book in backup.books {
            match keys.get(&source_key(&book.path)) {
                None => result.added += 1,
                Some(count) => {
                    result.skipped += 1;
                    if *count > 1 {
                        result.conflicts += 1;
                        result.issues.push(format!(
                            "{}：現有多筆來源別名，會略過，不合併。",
                            book.title
                        ));
                    }
                }
            }
        }
        Ok(result)
    }
    pub fn backup_settings(&self) -> Result<BackupSettings, String> {
        let conn = self.connection.lock().map_err(db_error)?;
        conn.query_row(
            "SELECT enabled,retention,last_success,last_error,custom_directory,last_directory,directory_warning FROM backup_settings WHERE id=1",
            [],
            |r| {
                let custom_directory: Option<String> = r.get(4)?;
                let last_directory: String = r.get(5)?;
                Ok(BackupSettings {
                    enabled: r.get(0)?,
                    retention: r.get(1)?,
                    last_success: r.get(2)?,
                    last_error: r.get(3)?,
                    directory: if last_directory.is_empty() {
                        custom_directory.clone().unwrap_or_else(|| self.directory.join("backups").to_string_lossy().into_owned())
                    } else { last_directory },
                    custom_directory,
                    directory_warning: r.get(6)?,
                })
            },
        )
        .map_err(db_error)
    }
    pub fn set_backup_settings(
        &self,
        enabled: bool,
        retention: usize,
    ) -> Result<BackupSettings, String> {
        if !(1..=20).contains(&retention) {
            return Err("自動備份保留數量須為 1–20。".into());
        }
        self.connection
            .lock()
            .map_err(db_error)?
            .execute(
                "UPDATE backup_settings SET enabled=?1,retention=?2 WHERE id=1",
                params![enabled, retention],
            )
            .map_err(db_error)?;
        self.backup_settings()
    }
    pub fn set_backup_directory(&self, custom: Option<String>) -> Result<BackupSettings, String> {
        let _guard = self.backup_lock.lock().map_err(db_error)?;
        let directory = match custom.as_deref() {
            Some(path) => validate_directory(Path::new(path))?,
            None => self.default_backup_directory()?,
        };
        self.connection.lock().map_err(db_error)?.execute(
            "UPDATE backup_settings SET custom_directory=?1,last_directory=?2,directory_warning='' WHERE id=1",
            params![custom.map(|_| directory.to_string_lossy().into_owned()), directory.to_string_lossy()],
        ).map_err(db_error)?;
        self.backup_settings()
    }
    fn default_backup_directory(&self) -> Result<PathBuf, String> {
        let directory = self.directory.join("backups");
        std::fs::create_dir_all(&directory).map_err(|e| format!("預設備份資料夾無法建立：{e}"))?;
        validate_directory(&directory)
    }
    fn fallback_directory(&self, error: &str) -> Result<(PathBuf, String), String> {
        let directory = self.default_backup_directory()?;
        let warning = format!("自訂資料夾無法使用，本次已改存預設位置。原因：{error}");
        Ok((directory, warning))
    }
    fn backup_destination(&self, settings: &BackupSettings) -> Result<(PathBuf, String), String> {
        if let Some(path) = &settings.custom_directory {
            match validate_directory(Path::new(path)) {
                Ok(directory) => Ok((directory, String::new())),
                Err(error) => self.fallback_directory(&error),
            }
        } else {
            Ok((self.default_backup_directory()?, String::new()))
        }
    }
    fn directory_to_open(&self) -> Result<PathBuf, String> {
        let path = PathBuf::from(self.backup_settings()?.directory);
        let meta = std::fs::metadata(&path).map_err(|e| {
            format!("備份資料夾不存在或無法存取，請先建立備份或重新選擇資料夾：{e}")
        })?;
        if !meta.is_dir() {
            return Err("備份位置不是資料夾，請重新選擇資料夾。".into());
        }
        Ok(path)
    }
    // Ownership is recorded before writing JSON. Only registered, complete regular files
    // can repair a failed success timestamp or enter retention; unknown files stay untouched.
    fn automatic_files(&self) -> Result<Vec<(PathBuf, i64)>, String> {
        let names = {
            let conn = self.connection.lock().map_err(db_error)?;
            let mut q = conn
                .prepare("SELECT path FROM automatic_backups")
                .map_err(db_error)?;
            let rows = q
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(db_error)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(db_error)?
        };
        let mut files = Vec::new();
        for name in names {
            let path = PathBuf::from(&name);
            if !registered_path(&path) {
                continue;
            }
            // An unavailable or redirected old folder is never adopted at its new target.
            let Some(parent) = path.parent() else {
                continue;
            };
            if std::fs::canonicalize(parent).ok().as_deref() != Some(parent) {
                continue;
            }
            let meta = match std::fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // A previous retention delete may have removed the file before SQL failed.
                    self.connection
                        .lock()
                        .map_err(db_error)?
                        .execute("DELETE FROM automatic_backups WHERE path=?1", [&name])
                        .map_err(db_error)?;
                    continue;
                }
                Err(_) => continue, // Keep ownership for offline/inaccessible folders; retry later.
            };
            if !meta.file_type().is_file() {
                continue;
            }
            let Ok(bytes) = read_backup(&path) else {
                continue;
            };
            if Self::parse_backup(&bytes).is_err() {
                continue;
            }
            let stamp = meta
                .modified()
                .map_err(db_error)?
                .duration_since(UNIX_EPOCH)
                .map_err(db_error)?
                .as_millis()
                .min(i64::MAX as u128) as i64;
            files.push((path, stamp));
        }
        files.sort_by_key(|(_, stamp)| *stamp);
        Ok(files)
    }
    fn clean_automatic_files(
        &self,
        files: &[(PathBuf, i64)],
        retention: usize,
    ) -> Result<(), String> {
        for (path, _) in files.iter().take(files.len().saturating_sub(retention)) {
            if !registered_path(path)
                || std::fs::canonicalize(path.parent().unwrap())
                    .ok()
                    .as_deref()
                    != path.parent()
                || !std::fs::symlink_metadata(path)
                    .map_err(db_error)?
                    .file_type()
                    .is_file()
            {
                continue;
            }
            std::fs::remove_file(path).map_err(db_error)?;
            self.connection
                .lock()
                .map_err(db_error)?
                .execute(
                    "DELETE FROM automatic_backups WHERE path=?1",
                    [path.to_string_lossy().as_ref()],
                )
                .map_err(db_error)?;
        }
        Ok(())
    }
    pub fn automatic_backup(&self, force: bool) -> Result<BackupSettings, String> {
        use std::io::Write;
        let _guard = self.backup_lock.lock().map_err(db_error)?;
        let mut phase = "狀態恢復";
        let mut created = false;
        let mut destination_notice = String::new();
        let result =
            (|| -> Result<(), String> {
                let files = self.automatic_files()?;
                let mut settings = self.backup_settings()?;
                let recovered = files
                    .last()
                    .is_some_and(|(_, t)| Some(*t) > settings.last_success);
                if recovered {
                    // Retry before the daily gate, including after process restart or disabled scheduling.
                    let stamp = files.last().unwrap().1;
                    self.connection
                        .lock()
                        .map_err(db_error)?
                        .execute(
                            "UPDATE backup_settings SET last_success=?1 WHERE id=1",
                            [stamp],
                        )
                        .map_err(db_error)?;
                    settings.last_success = Some(stamp);
                }
                if !force
                    && (!settings.enabled
                        || settings
                            .last_success
                            .is_some_and(|t| now().saturating_sub(t) < 86_400_000))
                {
                    phase = "保留清理";
                    self.clean_automatic_files(&files, settings.retention)?;
                    // Both recovery and retention succeeded. Clear their prior errors,
                    // including a failed retry; creation errors remain until creation is retried.
                    if recovered
                        || settings.last_error.starts_with("備份已建立")
                        || settings.last_error.starts_with(RECOVERY_FAILURE_PREFIX)
                    {
                        self.connection
                            .lock()
                            .map_err(db_error)?
                            .execute("UPDATE backup_settings SET last_error='' WHERE id=1", [])
                            .map_err(db_error)?;
                    }
                    return Ok(());
                }
                phase = "建立";
                let bytes = self.backup_json()?;
                let (mut dir, mut warning) = self.backup_destination(&settings)?;
                let path = loop {
                    phase = "建立";
                    let path = dir.join(unique_name("auto", "json"));
                    // Reserve a new empty file, never adopt/overwrite a colliding user file.
                    let reserved = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path);
                    let mut file = match reserved {
                        Ok(file) => file,
                        Err(error) if warning.is_empty() && settings.custom_directory.is_some() => {
                            (dir, warning) = self.fallback_directory(&error.to_string())?;
                            continue;
                        }
                        Err(error) => return Err(db_error(error)),
                    };
                    phase = "擁有權登記";
                    let registered = self.connection.lock().map_err(db_error)?.execute(
                        "INSERT INTO automatic_backups(path) VALUES(?1)",
                        [path.to_string_lossy().as_ref()],
                    );
                    if let Err(error) = registered {
                        drop(file);
                        let _ = std::fs::remove_file(&path); // Only this newly reserved, empty file.
                        return Err(db_error(error));
                    }
                    phase = "寫入與同步";
                    if let Err(error) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
                        drop(file);
                        let _ = std::fs::remove_file(&path);
                        let _ = self.connection.lock().map_err(db_error)?.execute(
                            "DELETE FROM automatic_backups WHERE path=?1",
                            [path.to_string_lossy().as_ref()],
                        );
                        if warning.is_empty() && settings.custom_directory.is_some() {
                            (dir, warning) = self.fallback_directory(&error.to_string())?;
                            continue;
                        }
                        return Err(db_error(error));
                    }
                    drop(file);
                    break path;
                };
                created = true;
                if !warning.is_empty() {
                    destination_notice =
                        format!(" {warning}位置：{}", path.parent().unwrap().display());
                }
                phase = "成功狀態更新";
                // Persist the actual destination before the existing timestamp operation;
                // timestamp recovery must not undo a later explicit folder selection.
                self.connection.lock().map_err(db_error)?.execute(
                "UPDATE backup_settings SET last_directory=?1,directory_warning=?2 WHERE id=1",
                params![path.parent().unwrap().to_string_lossy(), warning],
            ).map_err(db_error)?;
                self.connection
                    .lock()
                    .map_err(db_error)?
                    .execute(
                        "UPDATE backup_settings SET last_success=?1,last_error='' WHERE id=1",
                        [now()],
                    )
                    .map_err(db_error)?;
                phase = "保留清理";
                self.clean_automatic_files(&self.automatic_files()?, settings.retention)?;
                Ok(())
            })();
        if let Err(error) = result {
            let mut message = if created {
                format!(
                    "備份已建立，但{phase}失敗；新備份與未清理檔案保留，下次檢查會重試：{error}"
                )
            } else if phase == "成功狀態更新" || phase == "狀態恢復" || phase == "保留清理"
            {
                format!("{RECOVERY_FAILURE_PREFIX}；未清理檔案保留，下次檢查會重試：{error}")
            } else {
                format!("備份{phase}失敗；未建立完整新備份，既有成功備份已保留：{error}")
            };
            message.push_str(&destination_notice);
            let _ = self.connection.lock().map_err(db_error)?.execute(
                "UPDATE backup_settings SET last_error=?1 WHERE id=1",
                [&message],
            );
            return Err(message);
        }
        self.backup_settings()
    }
}
fn owned_name(name: &str) -> bool {
    let Some(rest) = name
        .strip_prefix("mangafolio-auto-")
        .and_then(|s| s.strip_suffix(".json"))
    else {
        return false;
    };
    let parts: Vec<_> = rest.split('-').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()))
}
pub(super) fn read_backup(path: &Path) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(db_error)?
        .take(MAX_BACKUP_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(db_error)?;
    Ok(bytes)
}
#[tauri::command]
pub async fn preview_library_backup(
    path: String,
    app: tauri::AppHandle,
) -> Result<RestorePreview, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Library>()
            .preview_json(&read_backup(Path::new(&path))?)
    })
    .await
    .map_err(db_error)?
}
#[tauri::command]
pub fn get_backup_settings(library: State<Library>) -> Result<BackupSettings, String> {
    library.backup_settings()
}
fn registered_path(path: &Path) -> bool {
    path.is_absolute()
        && !path.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
        && path
            .file_name()
            .is_some_and(|name| owned_name(&name.to_string_lossy()))
}
fn validate_directory(path: &Path) -> Result<PathBuf, String> {
    use std::io::Write;
    if !path.is_absolute() {
        return Err("備份資料夾必須是絕對路徑。".into());
    }
    let meta = std::fs::metadata(path).map_err(|e| format!("備份資料夾不存在或無法存取：{e}"))?;
    if !meta.is_dir() {
        return Err("備份位置不是資料夾。".into());
    }
    let directory = std::fs::canonicalize(path).map_err(db_error)?;
    let probe = directory.join(unique_name("write-test", "tmp"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .map_err(|e| format!("備份資料夾無法寫入：{e}"))?;
    let result = file
        .write_all(b"MangaFolio write test")
        .and_then(|_| file.sync_all());
    drop(file);
    let removed = std::fs::remove_file(&probe);
    result.map_err(|e| format!("備份資料夾無法寫入：{e}"))?;
    removed.map_err(|e| format!("備份資料夾無法刪除測試檔：{e}"))?;
    Ok(directory)
}
#[tauri::command]
pub fn get_backup_directory(library: State<Library>) -> Result<String, String> {
    Ok(library.backup_settings()?.directory)
}
#[tauri::command]
pub fn open_backup_directory(library: State<Library>, app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let directory = library.directory_to_open()?;
    app.opener()
        .open_path(directory.to_string_lossy().into_owned(), None::<&str>)
        .map_err(|e| format!("無法開啟備份資料夾，請確認資料夾可存取後再試：{e}"))
}
#[tauri::command]
pub async fn set_backup_directory(
    path: Option<String>,
    app: tauri::AppHandle,
) -> Result<BackupSettings, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || app.state::<Library>().set_backup_directory(path))
        .await
        .map_err(db_error)?
}
#[tauri::command]
pub fn set_backup_settings(
    enabled: bool,
    retention: usize,
    library: State<Library>,
) -> Result<BackupSettings, String> {
    library.set_backup_settings(enabled, retention)
}
#[tauri::command]
pub async fn run_automatic_backup(
    force: bool,
    app: tauri::AppHandle,
) -> Result<BackupSettings, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || app.state::<Library>().automatic_backup(force))
        .await
        .map_err(db_error)?
}
#[cfg(test)]
pub(super) fn test_sequence() -> u64 {
    SEQUENCE.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod directory_tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "mangafolio-directory-{}-{}",
                std::process::id(),
                test_sequence()
            ));
            std::fs::create_dir_all(&root).unwrap();
            Self(root)
        }
        fn library(&self) -> Library {
            Library::open(&self.0.join("data")).unwrap()
        }
        fn folder(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            std::fs::create_dir_all(&path).unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn automatic_custom_directory_validation_and_retention_never_adopt_same_name_in_new_folder() {
        let f = Fixture::new();
        let l = f.library();
        assert!(l
            .set_backup_directory(Some("relative".into()))
            .unwrap_err()
            .contains("絕對路徑"));
        assert!(l
            .set_backup_directory(Some(f.0.join("missing").to_string_lossy().into()))
            .unwrap_err()
            .contains("不存在"));
        let file = f.0.join("file");
        std::fs::write(&file, b"keep").unwrap();
        assert!(l
            .set_backup_directory(Some(file.to_string_lossy().into()))
            .unwrap_err()
            .contains("不是資料夾"));
        assert!(l.backup_settings().unwrap().custom_directory.is_none());
        assert!(l.directory_to_open().unwrap_err().contains("先建立備份"));
        let a = f.folder("a");
        let b = f.folder("b");
        let configured = l
            .set_backup_directory(Some(a.to_string_lossy().into()))
            .unwrap();
        assert_eq!(
            configured.directory,
            std::fs::canonicalize(&a).unwrap().to_string_lossy()
        );
        assert_eq!(
            std::fs::read_dir(&a).unwrap().count(),
            0,
            "writability probe is removed"
        );
        l.set_backup_settings(true, 1).unwrap();
        l.automatic_backup(true).unwrap();
        let owned = l.automatic_files().unwrap()[0].0.clone();
        let old_bytes = std::fs::read(&owned).unwrap();
        let unrelated = b.join(owned.file_name().unwrap());
        std::fs::write(&unrelated, &old_bytes).unwrap();
        l.set_backup_directory(Some(b.to_string_lossy().into()))
            .unwrap();
        assert_eq!(
            std::fs::read(&owned).unwrap(),
            old_bytes,
            "changing settings does not move/delete old files"
        );
        l.automatic_backup(true).unwrap();
        assert!(
            !owned.exists(),
            "retention deletes the registered file in its original directory"
        );
        assert_eq!(
            std::fs::read(&unrelated).unwrap(),
            old_bytes,
            "unregistered same-name backup remains intact"
        );
        assert!(l
            .automatic_files()
            .unwrap()
            .iter()
            .all(|(path, _)| path.parent() == Some(std::fs::canonicalize(&b).unwrap().as_path())));
        let bytes = l.backup_json().unwrap();
        assert!(!String::from_utf8(bytes)
            .unwrap()
            .contains("customDirectory"));
        let reset = l.set_backup_directory(None).unwrap();
        assert!(reset.custom_directory.is_none());
        assert!(reset.directory_warning.is_empty());
        assert_eq!(
            l.directory_to_open().unwrap(),
            std::fs::canonicalize(f.0.join("data/backups")).unwrap()
        );
        assert_eq!(std::fs::read(&unrelated).unwrap(), old_bytes);
    }

    #[test]
    fn automatic_custom_directory_unavailable_falls_back_and_keeps_warning_across_restart() {
        let f = Fixture::new();
        let l = f.library();
        let custom = f.folder("custom");
        l.set_backup_directory(Some(custom.to_string_lossy().into()))
            .unwrap();
        std::fs::remove_dir(&custom).unwrap();
        assert!(l.directory_to_open().unwrap_err().contains("不存在"));
        let fallback = l.automatic_backup(true).unwrap();
        assert!(fallback.directory_warning.contains("本次已改存預設位置"));
        assert_eq!(
            fallback.directory,
            std::fs::canonicalize(f.0.join("data/backups"))
                .unwrap()
                .to_string_lossy()
        );
        assert!(fallback.custom_directory.is_some());
        assert!(fallback.last_error.is_empty());
        assert!(l.automatic_files().unwrap()[0]
            .0
            .starts_with(&fallback.directory));
        drop(l);
        let l = f.library();
        assert_eq!(
            l.backup_settings().unwrap().directory_warning,
            fallback.directory_warning
        );
        // A file replacing the selected folder is also unavailable and must remain untouched.
        std::fs::write(&custom, b"user file").unwrap();
        assert!(l
            .automatic_backup(true)
            .unwrap()
            .directory_warning
            .contains("不是資料夾"));
        assert_eq!(std::fs::read(&custom).unwrap(), b"user file");
        std::fs::remove_file(&custom).unwrap();
        std::fs::create_dir(&custom).unwrap();
        let recovered = l.automatic_backup(true).unwrap();
        assert!(recovered.directory_warning.is_empty());
        assert_eq!(
            recovered.directory,
            std::fs::canonicalize(&custom).unwrap().to_string_lossy()
        );
    }

    #[test]
    fn v3_directory_migration_preserves_manifest_and_settings_and_snapshots_before_v4() {
        let f = Fixture::new();
        let l = f.library();
        l.set_backup_settings(true, 7).unwrap();
        l.automatic_backup(true).unwrap();
        let before = l.backup_json().unwrap();
        let owned = l.automatic_files().unwrap()[0].0.clone();
        let name = owned.file_name().unwrap().to_string_lossy().into_owned();
        {
            let conn = l.connection.lock().unwrap();
            conn.execute_batch(
                "ALTER TABLE automatic_backups RENAME COLUMN path TO name;
                ALTER TABLE backup_settings DROP COLUMN custom_directory;
                ALTER TABLE backup_settings DROP COLUMN last_directory;
                ALTER TABLE backup_settings DROP COLUMN directory_warning;
                PRAGMA user_version=3;",
            )
            .unwrap();
            conn.execute("UPDATE automatic_backups SET name=?1", [&name])
                .unwrap();
            conn.execute(
                "INSERT INTO automatic_backups(name) VALUES(?1)",
                [f.0.join("mangafolio-auto-1-2-3.json").to_string_lossy()],
            )
            .unwrap();
        }
        drop(l);
        let l = f.library();
        assert_eq!(before, l.backup_json().unwrap());
        let settings = l.backup_settings().unwrap();
        assert!(settings.enabled);
        assert_eq!(settings.retention, 7);
        assert!(settings.custom_directory.is_none());
        assert_eq!(l.automatic_files().unwrap()[0].0, owned);
        let conn = l.connection.lock().unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            4
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM automatic_backups", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        drop(conn);
        let snapshot = std::fs::read_dir(f.0.join("data/backups"))
            .unwrap()
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "sqlite3")
            })
            .unwrap();
        let old = Connection::open(snapshot.path()).unwrap();
        assert_eq!(
            old.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert_eq!(
            old.query_row(
                "SELECT name FROM automatic_backups WHERE name=?1",
                [&name],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            name
        );
    }

    #[test]
    fn v4_directory_migration_failure_is_atomic_and_preserves_v3_snapshot() {
        let f = Fixture::new();
        let directory = f.folder("data");
        let conn = Connection::open(directory.join("library.sqlite3")).unwrap();
        conn.execute_batch("CREATE TABLE backup_settings(id INTEGER PRIMARY KEY, custom_directory TEXT);
            INSERT INTO backup_settings VALUES(1,'keep'); CREATE TABLE automatic_backups(name TEXT PRIMARY KEY);
            PRAGMA user_version=3;").unwrap();
        assert!(Library::open(&directory).is_err());
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert_eq!(
            conn.query_row("SELECT custom_directory FROM backup_settings", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "keep"
        );
        assert!(conn.prepare("SELECT name FROM automatic_backups").is_ok());
        assert!(std::fs::read_dir(directory.join("backups"))
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .path()
                .extension()
                .is_some_and(|s| s == "sqlite3")));
    }

    #[test]
    fn automatic_directory_state_survives_timestamp_recovery_without_undoing_user_selection() {
        for fallback in [false, true] {
            let f = Fixture::new();
            let l = f.library();
            let custom = f.folder("custom");
            l.set_backup_directory(Some(custom.to_string_lossy().into()))
                .unwrap();
            if fallback {
                std::fs::remove_dir(&custom).unwrap();
            }
            l.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_stamp BEFORE UPDATE ON backup_settings WHEN NEW.last_success IS NOT NULL BEGIN SELECT RAISE(ABORT,'stamp failed'); END;").unwrap();
            let error = l.automatic_backup(true).unwrap_err();
            assert!(error.contains("備份已建立，但成功狀態更新失敗"));
            let pending = l.backup_settings().unwrap();
            assert!(pending.last_success.is_none());
            if fallback {
                assert!(pending.directory_warning.contains("已改存預設"));
                assert!(error.contains("已改存預設"));
                assert_eq!(
                    pending.directory,
                    std::fs::canonicalize(f.0.join("data/backups"))
                        .unwrap()
                        .to_string_lossy()
                );
            }
            l.connection
                .lock()
                .unwrap()
                .execute_batch("DROP TRIGGER fail_stamp;")
                .unwrap();
            if !fallback {
                let other = f.folder("other");
                l.set_backup_directory(Some(other.to_string_lossy().into()))
                    .unwrap();
            }
            let selected = l.backup_settings().unwrap();
            drop(l);
            let l = f.library();
            let recovered = l.automatic_backup(false).unwrap();
            assert!(recovered.last_success.is_some());
            assert!(recovered.last_error.is_empty());
            assert_eq!(recovered.directory, selected.directory);
            assert_eq!(recovered.directory_warning, selected.directory_warning);
        }
    }
}
