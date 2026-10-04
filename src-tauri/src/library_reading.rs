use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bookmark {
    pub id: i64,
    pub book_id: i64,
    pub page_name: String,
    pub page_index: usize,
    pub name: String,
    pub note: String,
}
impl Bookmark {
    fn validate(&self) -> Result<(), String> {
        metadata::valid_text(&self.name, 80)?;
        metadata::valid_text(&self.note, 2000)?;
        if self.name.trim().is_empty()
            || self.name.chars().any(char::is_control)
            || self.page_name.is_empty()
            || self.page_name.len() > 32768
            || self.page_name.contains('\0')
            || self.page_index >= 1_000_000
        {
            return Err("書籤名稱、頁名或頁碼無效。".into());
        }
        Ok(())
    }
}
pub(super) fn migrate(conn: &mut Connection) -> Result<(), String> {
    let tx = conn.transaction().map_err(db_error)?;
    migrate_in(&tx)?;
    tx.commit().map_err(db_error)
}
pub(super) fn migrate_in(conn: &Connection) -> Result<(), String> {
    conn.execute_batch("CREATE TABLE bookmarks(id INTEGER PRIMARY KEY AUTOINCREMENT, book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE, page_name TEXT NOT NULL, page_index INTEGER NOT NULL, name TEXT NOT NULL, note TEXT NOT NULL DEFAULT '');
        CREATE INDEX bookmarks_book ON bookmarks(book_id,id);
        CREATE TABLE backup_settings(id INTEGER PRIMARY KEY CHECK(id=1), enabled INTEGER NOT NULL DEFAULT 0, retention INTEGER NOT NULL DEFAULT 5, last_success INTEGER, last_error TEXT NOT NULL DEFAULT '');
        INSERT INTO backup_settings(id) VALUES(1);
        CREATE TABLE automatic_backups(name TEXT PRIMARY KEY);
        PRAGMA user_version=3;").map_err(db_error)?;
    Ok(())
}
pub(super) fn bookmarks_from(
    conn: &Connection,
    book_id: Option<i64>,
) -> Result<Vec<Bookmark>, String> {
    let mut q=conn.prepare("SELECT id,book_id,page_name,page_index,name,note FROM bookmarks WHERE (?1 IS NULL OR book_id=?1) ORDER BY id").map_err(db_error)?;
    let rows = q
        .query_map([book_id], |r| {
            Ok(Bookmark {
                id: r.get(0)?,
                book_id: r.get(1)?,
                page_name: r.get(2)?,
                page_index: r.get(3)?,
                name: r.get(4)?,
                note: r.get(5)?,
            })
        })
        .map_err(db_error)?;
    rows.collect::<Result<_, _>>().map_err(db_error)
}
pub(super) fn insert_bookmark(
    conn: &Connection,
    book_id: i64,
    mark: &Bookmark,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO bookmarks(book_id,page_name,page_index,name,note) VALUES(?1,?2,?3,?4,?5)",
        params![
            book_id,
            mark.page_name,
            mark.page_index,
            mark.name.trim(),
            mark.note
        ],
    )
    .map_err(db_error)?;
    Ok(())
}
pub(super) fn validate_backup_bookmarks(backup: &LibraryBackup) -> Result<(), String> {
    if backup.version < 3 && !backup.bookmarks.is_empty() {
        return Err("舊版本備份不可包含書籤。".into());
    }
    let mut book_ids = std::collections::HashMap::new();
    for book in &backup.books {
        *book_ids.entry(book.id).or_insert(0usize) += 1;
    }
    let mut counts = std::collections::HashMap::new();
    let mut ids = std::collections::HashSet::new();
    for mark in &backup.bookmarks {
        mark.validate()?;
        if mark.id <= 0 || !ids.insert(mark.id) || book_ids.get(&mark.book_id) != Some(&1) {
            return Err("備份書籤關聯無效或重複。".into());
        }
        let count = counts.entry(mark.book_id).or_insert(0);
        *count += 1;
        if *count > 200 {
            return Err("每本最多 200 個書籤。".into());
        }
    }
    Ok(())
}
pub(super) fn series_name(name: &str) -> Result<String, String> {
    metadata::valid_text(name, 256)?;
    if name.chars().any(char::is_control) {
        return Err("系列名稱不可包含控制字元。".into());
    }
    Ok(name.trim().to_string())
}
impl Library {
    pub fn assign_series(&self, ids: &[i64], name: &str) -> Result<(), String> {
        validate_ids(ids)?;
        let name = series_name(name)?;
        let mut conn = self.connection.lock().map_err(db_error)?;
        let tx = conn.transaction().map_err(db_error)?;
        for id in ids {
            if tx
                .execute("UPDATE books SET series=?1 WHERE id=?2", params![name, id])
                .map_err(db_error)?
                != 1
            {
                return Err("部分書籍已不存在，未修改任何系列。".into());
            }
        }
        tx.commit().map_err(db_error)
    }
    pub fn bookmarks(&self, book_id: i64) -> Result<Vec<Bookmark>, String> {
        let conn = self.connection.lock().map_err(db_error)?;
        bookmarks_from(&conn, Some(book_id))
    }
    pub fn save_bookmark(&self, mark: &Bookmark) -> Result<Bookmark, String> {
        mark.validate()?;
        let mut conn = self.connection.lock().map_err(db_error)?;
        let tx = conn.transaction().map_err(db_error)?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM books WHERE id=?1)",
                [mark.book_id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if !exists {
            return Err("書籍已不存在。".into());
        }
        if mark.id == 0 {
            let count: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM bookmarks WHERE book_id=?1",
                    [mark.book_id],
                    |r| r.get(0),
                )
                .map_err(db_error)?;
            if count >= 200 {
                return Err("每本最多 200 個書籤。".into());
            }
            insert_bookmark(&tx, mark.book_id, mark)?;
        } else if mark.id < 0
            || tx
                .execute(
                    "UPDATE bookmarks SET name=?1,note=?2 WHERE id=?3 AND book_id=?4",
                    params![mark.name.trim(), mark.note, mark.id, mark.book_id],
                )
                .map_err(db_error)?
                != 1
        {
            return Err("書籤已不存在。".into());
        }
        let id = if mark.id == 0 {
            tx.last_insert_rowid()
        } else {
            mark.id
        };
        let saved = bookmarks_from(&tx, Some(mark.book_id))?
            .into_iter()
            .find(|m| m.id == id)
            .ok_or("書籤保存失敗。")?;
        tx.commit().map_err(db_error)?;
        Ok(saved)
    }
    pub fn delete_bookmark(&self, id: i64, book_id: i64) -> Result<(), String> {
        let conn = self.connection.lock().map_err(db_error)?;
        if conn
            .execute(
                "DELETE FROM bookmarks WHERE id=?1 AND book_id=?2",
                params![id, book_id],
            )
            .map_err(db_error)?
            != 1
        {
            return Err("書籤已不存在。".into());
        }
        Ok(())
    }
}
#[tauri::command]
pub fn assign_book_series(
    ids: Vec<i64>,
    name: String,
    library: State<Library>,
) -> Result<(), String> {
    library.assign_series(&ids, &name)
}
#[tauri::command]
pub fn list_bookmarks(book_id: i64, library: State<Library>) -> Result<Vec<Bookmark>, String> {
    library.bookmarks(book_id)
}
#[tauri::command]
pub fn save_bookmark(bookmark: Bookmark, library: State<Library>) -> Result<Bookmark, String> {
    library.save_bookmark(&bookmark)
}
#[tauri::command]
pub fn delete_bookmark(id: i64, book_id: i64, library: State<Library>) -> Result<(), String> {
    library.delete_bookmark(id, book_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "mangafolio-third-{}-{}",
                std::process::id(),
                super::super::safety::test_sequence()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn library(&self) -> Library {
            Library::open(&self.0.join("data")).unwrap()
        }
        fn book(&self) -> book::Book {
            let folder = self.0.join("sources");
            std::fs::create_dir_all(&folder).unwrap();
            for n in 1..=3 {
                image::RgbImage::new(2, 3)
                    .save(folder.join(format!("{n}.png")))
                    .unwrap();
            }
            book::open(folder.to_str().unwrap()).unwrap().book
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn mark(id: i64) -> Bookmark {
        Bookmark {
            id: 0,
            book_id: id,
            page_name: "2.png".into(),
            page_index: 1,
            name: "喜歡的一頁".into(),
            note: "<script>純文字</script>".into(),
        }
    }
    #[test]
    fn series_batch_missing_id_rolls_back_and_does_not_touch_progress() {
        let f = Fixture::new();
        let l = f.library();
        let b = l.register(&f.book()).unwrap();
        l.favorite(b.id, true).unwrap();
        let before = l.backup_json().unwrap();
        assert!(l.assign_series(&[b.id, 99999], "aaa").is_err());
        assert_eq!(before, l.backup_json().unwrap());
        l.assign_series(&[b.id], " aaa ").unwrap();
        let b = l.get(b.id).unwrap();
        assert_eq!(b.series, "aaa");
        assert!(b.favorite);
        assert!(l.assign_series(&[b.id], "bad\nname").is_err());
        l.assign_series(&[b.id], "").unwrap();
        assert_eq!(l.get(b.id).unwrap().series, "");
        assert!(f.0.join("sources/1.png").exists());
    }
    #[test]
    fn bookmarks_crud_limits_and_backup_remapping_skip_existing() {
        let f = Fixture::new();
        let l = f.library();
        let b = l.register(&f.book()).unwrap();
        let mut saved = l.save_bookmark(&mark(b.id)).unwrap();
        saved.name = "新的名稱".into();
        saved.note = "新的筆記".into();
        saved.page_name = "attempted-change.png".into();
        let updated = l.save_bookmark(&saved).unwrap();
        assert_eq!(updated.page_name, "2.png");
        let bytes = l.backup_json().unwrap();
        let target = Library::open(&f.0.join("target")).unwrap();
        // Occupy earlier IDs with a different real source so restore must remap bookmark ownership.
        let other = Fixture::new();
        target.register(&other.book()).unwrap();
        let result = target.restore_json(&bytes).unwrap();
        assert_eq!(result.added, 1);
        let id = target.list().unwrap()[0].id;
        assert_ne!(id, b.id);
        assert_eq!(target.bookmarks(id).unwrap()[0].book_id, id);
        assert_eq!(target.bookmarks(id).unwrap()[0].note, "新的筆記");
        target
            .delete_bookmark(target.bookmarks(id).unwrap()[0].id, id)
            .unwrap();
        assert_eq!(target.restore_json(&bytes).unwrap().skipped, 1);
        assert!(target.bookmarks(id).unwrap().is_empty());
        for _ in 1..200 {
            l.save_bookmark(&mark(b.id)).unwrap();
        }
        assert!(l.save_bookmark(&mark(b.id)).is_err());
        l.remove(&[b.id]).unwrap();
        assert!(l.bookmarks(b.id).unwrap().is_empty());
    }
    #[test]
    fn restore_preview_is_readonly_actual_restore_revalidates_and_bookmarks_rollback() {
        let f = Fixture::new();
        let l = f.library();
        let b = l.register(&f.book()).unwrap();
        l.save_bookmark(&mark(b.id)).unwrap();
        let bytes = l.backup_json().unwrap();
        let target = Library::open(&f.0.join("target")).unwrap();
        let before = target.backup_json().unwrap();
        assert_eq!(target.preview_json(&bytes).unwrap().added, 1);
        assert_eq!(before, target.backup_json().unwrap());
        let mut invalid: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        invalid["bookmarks"][0]["bookId"] = serde_json::json!(9999);
        assert!(
            !target
                .preview_json(&serde_json::to_vec(&invalid).unwrap())
                .unwrap()
                .can_restore
        );
        assert!(target
            .restore_json(&serde_json::to_vec(&invalid).unwrap())
            .is_err());
        target.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_mark BEFORE INSERT ON bookmarks BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
        assert!(target.restore_json(&bytes).is_err());
        assert_eq!(before, target.backup_json().unwrap());
    }
    #[test]
    fn auto_backup_retention_preserves_unknown_and_preupgrade_files_and_failure() {
        let f = Fixture::new();
        let l = f.library();
        l.register(&f.book()).unwrap();
        assert!(!l.backup_settings().unwrap().enabled);
        assert!(l.set_backup_settings(true, 0).is_err());
        l.set_backup_settings(true, 1).unwrap();
        let dir = f.0.join("data/backups");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("unknown.json"), b"keep").unwrap();
        let unknown = dir.join("mangafolio-auto-1-2-3.json");
        std::fs::write(&unknown, l.backup_json().unwrap()).unwrap();
        let snapshot = dir.join("mangafolio-preupgrade-1-2-3.sqlite3");
        std::fs::write(&snapshot, b"keep").unwrap();
        l.automatic_backup(true).unwrap();
        l.automatic_backup(true).unwrap();
        assert!(unknown.exists() && snapshot.exists());
        let count: i64 = l
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM automatic_backups", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        let before = std::fs::read_dir(&dir).unwrap().count();
        l.automatic_backup(false).unwrap();
        assert_eq!(before, std::fs::read_dir(&dir).unwrap().count());
        // Successful export followed by retention bookkeeping failure must report partial cleanup accurately.
        l.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_cleanup BEFORE DELETE ON automatic_backups BEGIN SELECT RAISE(ABORT,'cleanup failed'); END;").unwrap();
        let error = l.automatic_backup(true).unwrap_err();
        assert!(error.contains("備份已建立，但保留清理失敗"));
        assert!(l.backup_settings().unwrap().last_success.is_some());
        assert!(unknown.exists() && snapshot.exists());
        assert!(std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name() != unknown.file_name().unwrap()
                && e.path().extension().is_some_and(|s| s == "json")));
        l.connection
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_cleanup;")
            .unwrap();
        l.automatic_backup(false).unwrap(); // Retry unfinished SQL cleanup before the daily gate.
        assert!(l.backup_settings().unwrap().last_error.is_empty());
        assert_eq!(
            l.connection
                .lock()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM automatic_backups", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(unknown.exists() && snapshot.exists());
        let other = Fixture::new();
        let bad = other.library();
        std::fs::write(other.0.join("data/backups"), b"not a directory").unwrap();
        assert!(bad.automatic_backup(true).is_err());
        assert!(!bad.backup_settings().unwrap().last_error.is_empty());
        assert!(bad.backup_settings().unwrap().last_success.is_none());
    }
    #[test]
    fn consecutive_cleanup_failures_clear_error_after_recovery_without_new_backup() {
        // Cover both the daily gate and disabled scheduling; recovery must precede either.
        for enabled in [true, false] {
            let f = Fixture::new();
            let l = f.library();
            l.register(&f.book()).unwrap();
            l.set_backup_settings(true, 1).unwrap();
            l.automatic_backup(true).unwrap();
            let dir = f.0.join("data/backups");
            let unknown = dir.join("mangafolio-auto-1-2-3.json");
            let unknown_bytes = l.backup_json().unwrap();
            std::fs::write(&unknown, &unknown_bytes).unwrap();
            let other = dir.join("unknown.json");
            std::fs::write(&other, b"keep unknown").unwrap();
            let snapshot = dir.join("mangafolio-preupgrade-1-2-3.sqlite3");
            std::fs::write(&snapshot, b"keep snapshot").unwrap();
            l.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_cleanup BEFORE DELETE ON automatic_backups BEGIN SELECT RAISE(ABORT,'cleanup failed'); END;").unwrap();
            let first = l.automatic_backup(true).unwrap_err();
            assert!(first.starts_with("備份已建立，但保留清理失敗"));
            let success = l.backup_settings().unwrap().last_success;
            assert!(success.is_some());
            let second = l.automatic_backup(false).unwrap_err();
            assert!(second.starts_with("備份狀態恢復／清理失敗"));
            assert_eq!(l.backup_settings().unwrap().last_error, second);
            assert_eq!(l.backup_settings().unwrap().last_success, success);
            let kept_path = std::fs::read_dir(&dir)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path != &unknown
                        && path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with("mangafolio-auto-")
                })
                .unwrap();
            let kept_bytes = std::fs::read(&kept_path).unwrap();
            l.connection
                .lock()
                .unwrap()
                .execute_batch("DROP TRIGGER fail_cleanup;")
                .unwrap();
            l.set_backup_settings(enabled, 1).unwrap();
            let settings = l.automatic_backup(false).unwrap();
            assert!(
                settings.last_error.is_empty(),
                "recovered but stale error remains: {}",
                settings.last_error
            );
            assert_eq!(settings.last_success, success);
            assert_eq!(settings.enabled, enabled);
            let names = {
                let conn = l.connection.lock().unwrap();
                let mut q = conn.prepare("SELECT name FROM automatic_backups").unwrap();
                q.query_map([], |r| r.get::<_, String>(0))
                    .unwrap()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap()
            };
            assert_eq!(names.len(), 1);
            let path = dir.join(&names[0]);
            let bytes = std::fs::read(&path).unwrap();
            assert_eq!(path, kept_path);
            assert_eq!(bytes, kept_bytes);
            assert!(Library::parse_backup(&bytes).is_ok());
            assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 4);
            assert_eq!(std::fs::read(&unknown).unwrap(), unknown_bytes);
            assert_eq!(std::fs::read(&other).unwrap(), b"keep unknown");
            assert_eq!(std::fs::read(&snapshot).unwrap(), b"keep snapshot");
            // Recovery remains settled on the following check; no extra JSON or timestamp bump.
            let settled = l.automatic_backup(false).unwrap();
            assert!(settled.last_error.is_empty());
            assert_eq!(settled.last_success, success);
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 4);
        }
    }
    #[test]
    fn cleanup_success_does_not_clear_unrecovered_backup_creation_error() {
        let f = Fixture::new();
        let l = f.library();
        l.register(&f.book()).unwrap();
        l.set_backup_settings(true, 1).unwrap();
        l.automatic_backup(true).unwrap();
        let success = l.backup_settings().unwrap().last_success;
        l.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_register BEFORE INSERT ON automatic_backups BEGIN SELECT RAISE(ABORT,'manifest failed'); END;").unwrap();
        let error = l.automatic_backup(true).unwrap_err();
        assert!(error.starts_with("備份擁有權登記失敗"));
        for enabled in [false, true] {
            l.set_backup_settings(enabled, 1).unwrap();
            let settings = l.automatic_backup(false).unwrap();
            assert_eq!(settings.last_error, error); // A successful cleanup did not retry creation.
            assert_eq!(settings.last_success, success);
            assert_eq!(
                std::fs::read_dir(f.0.join("data/backups")).unwrap().count(),
                1
            );
        }
    }
    #[test]
    fn automatic_manifest_failure_never_leaves_complete_unregistered_json_or_success_time() {
        let f = Fixture::new();
        let l = f.library();
        l.register(&f.book()).unwrap();
        l.set_backup_settings(true, 1).unwrap();
        l.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_register BEFORE INSERT ON automatic_backups BEGIN SELECT RAISE(ABORT,'manifest failed'); END;").unwrap();
        for force in [true, false] {
            let error = l.automatic_backup(force).unwrap_err();
            assert!(error.contains("擁有權登記失敗") && error.contains("未建立完整新備份"));
            assert!(l.backup_settings().unwrap().last_success.is_none());
            assert_eq!(
                std::fs::read_dir(f.0.join("data/backups")).unwrap().count(),
                0
            );
        }
        l.connection
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_register;")
            .unwrap();
        l.automatic_backup(false).unwrap();
        assert!(l.backup_settings().unwrap().last_success.is_some());
        let prior = l.backup_settings().unwrap().last_success;
        let path = std::fs::read_dir(f.0.join("data/backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let bytes = std::fs::read(&path).unwrap();
        l.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_register BEFORE INSERT ON automatic_backups BEGIN SELECT RAISE(ABORT,'manifest failed'); END;").unwrap();
        assert!(l.automatic_backup(true).is_err());
        assert_eq!(l.backup_settings().unwrap().last_success, prior);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(
            std::fs::read_dir(f.0.join("data/backups")).unwrap().count(),
            1
        );
    }
    #[test]
    fn automatic_success_timestamp_retry_survives_restart_before_daily_gate() {
        let f = Fixture::new();
        let l = f.library();
        l.register(&f.book()).unwrap();
        l.set_backup_settings(true, 1).unwrap();
        l.connection.lock().unwrap().execute_batch("CREATE TRIGGER fail_stamp BEFORE UPDATE ON backup_settings WHEN NEW.last_success IS NOT NULL BEGIN SELECT RAISE(ABORT,'stamp failed'); END;").unwrap();
        let error = l.automatic_backup(true).unwrap_err();
        assert!(error.contains("備份已建立，但成功狀態更新失敗"));
        assert!(l.backup_settings().unwrap().last_success.is_none());
        let path = std::fs::read_dir(f.0.join("data/backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let bytes = std::fs::read(&path).unwrap();
        assert!(Library::parse_backup(&bytes).is_ok());
        assert_eq!(
            l.connection
                .lock()
                .unwrap()
                .query_row("SELECT COUNT(*) FROM automatic_backups", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        l.connection
            .lock()
            .unwrap()
            .execute_batch("DROP TRIGGER fail_stamp;")
            .unwrap();
        l.set_backup_settings(false, 1).unwrap(); // Disabled scheduling must still repair bookkeeping.
        drop(l);
        let restarted = f.library();
        restarted.automatic_backup(false).unwrap();
        assert!(restarted.backup_settings().unwrap().last_success.is_some());
        assert!(restarted.backup_settings().unwrap().last_error.is_empty());
        assert!(!restarted.backup_settings().unwrap().enabled);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(
            std::fs::read_dir(f.0.join("data/backups")).unwrap().count(),
            1
        );
        restarted.automatic_backup(false).unwrap();
        assert_eq!(
            std::fs::read_dir(f.0.join("data/backups")).unwrap().count(),
            1
        );
    }
    #[test]
    fn migration_sql_failure_rolls_back_new_tables_and_keeps_safe_snapshot() {
        let f = Fixture::new();
        let dir = f.0.join("migration-failure");
        std::fs::create_dir_all(&dir).unwrap();
        let db = Connection::open(dir.join("library.sqlite3")).unwrap();
        // Force failure after bookmarks creation, to verify DDL is in one transaction.
        db.execute_batch("CREATE TABLE backup_settings(original TEXT); INSERT INTO backup_settings VALUES('keep'); PRAGMA user_version=2;").unwrap();
        let failure = Library::open(&dir).err().unwrap();
        assert!(
            failure.contains("backup_settings") && failure.contains("already exists"),
            "must reach the injected DDL failure: {failure}"
        );
        assert_eq!(
            db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='bookmarks'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT original FROM backup_settings", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "keep"
        );
        assert_eq!(std::fs::read_dir(dir.join("backups")).unwrap().count(), 1);
    }
    #[test]
    fn upgrade_snapshot_includes_uncheckpointed_wal_and_failure_keeps_v2() {
        let f = Fixture::new();
        let dir = f.0.join("old");
        std::fs::create_dir_all(&dir).unwrap();
        let conn = Connection::open(dir.join("library.sqlite3")).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;PRAGMA wal_autocheckpoint=0;CREATE TABLE custom_test(value TEXT);INSERT INTO custom_test VALUES('wal-value');PRAGMA user_version=2;").unwrap();
        let l = Library::open(&dir).unwrap();
        drop(l);
        let snapshot = std::fs::read_dir(dir.join("backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let snapshot = Connection::open(snapshot).unwrap();
        assert_eq!(
            snapshot
                .query_row("SELECT value FROM custom_test", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "wal-value"
        );
        assert_eq!(
            snapshot
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
        let bad = f.0.join("blocked");
        std::fs::create_dir_all(&bad).unwrap();
        let db = Connection::open(bad.join("library.sqlite3")).unwrap();
        db.execute_batch("PRAGMA user_version=2;").unwrap();
        std::fs::write(bad.join("backups"), b"blocked").unwrap();
        assert!(Library::open(&bad).is_err());
        assert_eq!(
            db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
}
