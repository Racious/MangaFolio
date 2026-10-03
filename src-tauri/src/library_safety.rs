use super::*;
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
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
            "SELECT enabled,retention,last_success,last_error FROM backup_settings WHERE id=1",
            [],
            |r| {
                Ok(BackupSettings {
                    enabled: r.get(0)?,
                    retention: r.get(1)?,
                    last_success: r.get(2)?,
                    last_error: r.get(3)?,
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
    // Ownership is recorded before writing JSON. Only registered, complete regular files
    // can repair a failed success timestamp or enter retention; unknown files stay untouched.
    fn automatic_files(&self) -> Result<Vec<(PathBuf, i64)>, String> {
        let names = {
            let conn = self.connection.lock().map_err(db_error)?;
            let mut q = conn
                .prepare("SELECT name FROM automatic_backups")
                .map_err(db_error)?;
            let rows = q
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(db_error)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(db_error)?
        };
        let mut files = Vec::new();
        for name in names {
            if !owned_name(&name) {
                continue;
            }
            let path = self.directory.join("backups").join(&name);
            let meta = match std::fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    // A previous retention delete may have removed the file before SQL failed.
                    self.connection
                        .lock()
                        .map_err(db_error)?
                        .execute("DELETE FROM automatic_backups WHERE name=?1", [&name])
                        .map_err(db_error)?;
                    continue;
                }
                Err(error) => return Err(db_error(error)),
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
            std::fs::remove_file(path).map_err(db_error)?;
            self.connection
                .lock()
                .map_err(db_error)?
                .execute(
                    "DELETE FROM automatic_backups WHERE name=?1",
                    [path.file_name().unwrap().to_string_lossy().as_ref()],
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
        let result = (|| -> Result<(), String> {
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
                if recovered || settings.last_error.starts_with("備份已建立") {
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
            let dir = self.directory.join("backups");
            std::fs::create_dir_all(&dir).map_err(db_error)?;
            let path = dir.join(unique_name("auto", "json"));
            // Reserve a new empty file, never adopt/overwrite a colliding user file.
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(db_error)?;
            phase = "擁有權登記";
            let registered = self.connection.lock().map_err(db_error)?.execute(
                "INSERT INTO automatic_backups(name) VALUES(?1)",
                [path.file_name().unwrap().to_string_lossy().as_ref()],
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
                    "DELETE FROM automatic_backups WHERE name=?1",
                    [path.file_name().unwrap().to_string_lossy().as_ref()],
                );
                return Err(db_error(error));
            }
            drop(file);
            created = true;
            phase = "成功狀態更新";
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
            let message = if created {
                format!(
                    "備份已建立，但{phase}失敗；新備份與未清理檔案保留，下次檢查會重試：{error}"
                )
            } else if phase == "成功狀態更新" || phase == "狀態恢復" || phase == "保留清理"
            {
                format!("備份狀態恢復／清理失敗；未清理檔案保留，下次檢查會重試：{error}")
            } else {
                format!("備份{phase}失敗；未建立完整新備份，既有成功備份已保留：{error}")
            };
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
