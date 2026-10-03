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
    std::fs::File::open(path)
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
    pub fn automatic_backup(&self, force: bool) -> Result<BackupSettings, String> {
        let _guard = self.backup_lock.lock().map_err(db_error)?;
        let settings = self.backup_settings()?;
        if !force
            && (!settings.enabled
                || settings
                    .last_success
                    .is_some_and(|t| now().saturating_sub(t) < 86_400_000))
        {
            return Ok(settings);
        }
        let mut backup_created = false;
        let result = (|| -> Result<(), String> {
            let dir = self.directory.join("backups");
            std::fs::create_dir_all(&dir).map_err(db_error)?;
            let path = dir.join(unique_name("auto", "json"));
            self.export_to(&path)?;
            {
                let mut conn = self.connection.lock().map_err(db_error)?;
                let tx = conn.transaction().map_err(db_error)?;
                tx.execute(
                    "INSERT INTO automatic_backups(name) VALUES(?1)",
                    [path.file_name().unwrap().to_string_lossy().as_ref()],
                )
                .map_err(db_error)?;
                tx.execute(
                    "UPDATE backup_settings SET last_success=?1,last_error='' WHERE id=1",
                    [now()],
                )
                .map_err(db_error)?;
                tx.commit().map_err(db_error)?;
            }
            backup_created = true;
            let registered = {
                let conn = self.connection.lock().map_err(db_error)?;
                let mut q = conn
                    .prepare("SELECT name FROM automatic_backups")
                    .map_err(db_error)?;
                let rows = q
                    .query_map([], |r| r.get::<_, String>(0))
                    .map_err(db_error)?;
                rows.collect::<Result<std::collections::HashSet<_>, _>>()
                    .map_err(db_error)?
            };
            // Only strict names with a valid MangaFolio payload are eligible. Symlinks are excluded.
            let mut owned = Vec::new();
            for entry in std::fs::read_dir(&dir).map_err(db_error)? {
                let entry = entry.map_err(db_error)?;
                let name = entry.file_name().to_string_lossy().to_string();
                if !entry.file_type().map_err(db_error)?.is_file()
                    || !owned_name(&name)
                    || !registered.contains(&name)
                {
                    continue;
                }
                if let Ok(bytes) = read_backup(&entry.path()) {
                    if Self::parse_backup(&bytes).is_ok() {
                        owned.push(entry.path());
                    }
                }
            }
            owned.sort_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
            let delete = owned.len().saturating_sub(settings.retention);
            for file in owned.into_iter().take(delete) {
                std::fs::remove_file(&file).map_err(db_error)?;
                self.connection
                    .lock()
                    .map_err(db_error)?
                    .execute(
                        "DELETE FROM automatic_backups WHERE name=?1",
                        [file.file_name().unwrap().to_string_lossy().as_ref()],
                    )
                    .map_err(db_error)?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            let _ = self.connection.lock().map_err(db_error)?.execute(
                "UPDATE backup_settings SET last_error=?1 WHERE id=1",
                [&error],
            );
            return Err(if backup_created {
                format!("備份已建立，但保留清理失敗；新備份與未清理檔案保留：{error}")
            } else {
                format!("備份失敗；既有成功備份已保留：{error}")
            });
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
