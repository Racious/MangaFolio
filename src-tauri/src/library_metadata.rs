use super::*;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Tag {
    pub id: i64,
    pub name: String,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BookDetails {
    pub custom_title: String,
    pub series: String,
    pub volume: String,
    pub notes: String,
}

pub(super) fn valid_text(value: &str, maximum: usize) -> Result<(), String> {
    if value.chars().count() > maximum || value.contains('\0') {
        return Err(format!("文字內容無效，最多 {maximum} 個字元。"));
    }
    Ok(())
}
impl BookDetails {
    pub(super) fn validate(&self) -> Result<(), String> {
        valid_text(&self.custom_title, 256)?;
        valid_text(&self.series, 256)?;
        valid_text(&self.volume, 64)?;
        valid_text(&self.notes, 4000)
    }
}
pub(super) fn tag_key(name: &str) -> Result<String, String> {
    if name.trim().is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err("標籤須為 1–64 個字元，不可包含控制字元。".into());
    }
    Ok(name.trim().to_lowercase())
}
pub(super) fn valid_status(status: &str) -> Result<(), String> {
    if !["unread", "reading", "read"].contains(&status) {
        return Err("閱讀狀態無效。".into());
    }
    Ok(())
}
pub(super) fn migrate(connection: &mut Connection) -> Result<(), String> {
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(db_error)?;
    tx.execute_batch("ALTER TABLE books ADD COLUMN custom_title TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN series TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN volume TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN notes TEXT NOT NULL DEFAULT '';
        ALTER TABLE books ADD COLUMN reading_status TEXT NOT NULL DEFAULT 'unread' CHECK(reading_status IN ('unread','reading','read'));
        ALTER TABLE books ADD COLUMN status_manual INTEGER NOT NULL DEFAULT 0;
        UPDATE books SET reading_status=CASE WHEN last_read_at IS NULL THEN 'unread' WHEN last_index>=page_count-1 THEN 'read' ELSE 'reading' END;
        CREATE TABLE tags(id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, name_key TEXT NOT NULL UNIQUE);
        CREATE TABLE book_tags(book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE, tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE, PRIMARY KEY(book_id,tag_id));
        CREATE INDEX book_tags_tag ON book_tags(tag_id,book_id);
        PRAGMA user_version=2;").map_err(db_error)?;
    tx.commit().map_err(db_error)
}
impl Library {
    pub fn edit_details(&self, id: i64, details: &BookDetails) -> Result<LibraryBook, String> {
        details.validate()?;
        {
            let conn = self.connection.lock().map_err(db_error)?;
            if conn
                .execute(
                    "UPDATE books SET custom_title=?1,series=?2,volume=?3,notes=?4 WHERE id=?5",
                    params![
                        details.custom_title.trim(),
                        details.series.trim(),
                        details.volume.trim(),
                        details.notes,
                        id
                    ],
                )
                .map_err(db_error)?
                != 1
            {
                return Err("書籍已不存在，未修改資訊。".into());
            }
        }
        self.get(id)
    }
    pub fn set_status(&self, ids: &[i64], status: &str) -> Result<(), String> {
        validate_ids(ids)?;
        if status != "auto" {
            valid_status(status)?;
        }
        let mut conn = self.connection.lock().map_err(db_error)?;
        let tx = conn.transaction().map_err(db_error)?;
        for id in ids {
            let count = if status == "auto" {
                tx.execute("UPDATE books SET status_manual=0,reading_status=CASE WHEN last_read_at IS NULL THEN 'unread' WHEN last_index>=page_count-1 THEN 'read' ELSE 'reading' END WHERE id=?1", [id])
            } else {
                tx.execute("UPDATE books SET reading_status=?1,status_manual=1 WHERE id=?2", params![status,id])
            }.map_err(db_error)?;
            if count != 1 {
                return Err("部分書籍已不存在，未修改任何閱讀狀態。".into());
            }
        }
        tx.commit().map_err(db_error)
    }
    pub fn tags(&self) -> Result<Vec<Tag>, String> {
        let conn = self.connection.lock().map_err(db_error)?;
        tags_from_connection(&conn)
    }
    pub fn create_tag(&self, name: &str) -> Result<Tag, String> {
        let key = tag_key(name)?;
        let mut conn = self.connection.lock().map_err(db_error)?;
        let tx = conn.transaction().map_err(db_error)?;
        let count: i64 = tx
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .map_err(db_error)?;
        if count >= 1000 {
            return Err("最多支援 1000 個標籤。".into());
        }
        tx.execute(
            "INSERT INTO tags(name,name_key) VALUES(?1,?2)",
            params![name.trim(), key],
        )
        .map_err(|_| "標籤名稱已存在或無法建立。".to_string())?;
        let tag = Tag {
            id: tx.last_insert_rowid(),
            name: name.trim().into(),
        };
        tx.commit().map_err(db_error)?;
        Ok(tag)
    }
    pub fn rename_tag(&self, id: i64, name: &str) -> Result<(), String> {
        let key = tag_key(name)?;
        let conn = self.connection.lock().map_err(db_error)?;
        if conn
            .execute(
                "UPDATE tags SET name=?1,name_key=?2 WHERE id=?3",
                params![name.trim(), key, id],
            )
            .map_err(|_| "標籤名稱已存在或無法更新。".to_string())?
            != 1
        {
            return Err("標籤已不存在。".into());
        }
        Ok(())
    }
    pub fn delete_tag(&self, id: i64) -> Result<(), String> {
        let conn = self.connection.lock().map_err(db_error)?;
        if conn
            .execute("DELETE FROM tags WHERE id=?1", [id])
            .map_err(db_error)?
            != 1
        {
            return Err("標籤已不存在。".into());
        }
        Ok(())
    }
    pub fn assign_tag(&self, ids: &[i64], tag_id: i64, add: bool) -> Result<(), String> {
        validate_ids(ids)?;
        let mut conn = self.connection.lock().map_err(db_error)?;
        let tx = conn.transaction().map_err(db_error)?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE id=?1)",
                [tag_id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if !exists {
            return Err("標籤已不存在。".into());
        }
        for id in ids {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM books WHERE id=?1)",
                    [id],
                    |r| r.get(0),
                )
                .map_err(db_error)?;
            if !exists {
                return Err("部分書籍已不存在，未修改任何標籤。".into());
            }
            if add {
                tx.execute(
                    "INSERT OR IGNORE INTO book_tags(book_id,tag_id) VALUES(?1,?2)",
                    params![id, tag_id],
                )
                .map_err(db_error)?;
                let count: i64 = tx
                    .query_row(
                        "SELECT COUNT(*) FROM book_tags WHERE book_id=?1",
                        [id],
                        |r| r.get(0),
                    )
                    .map_err(db_error)?;
                if count > 100 {
                    return Err("單本書最多 100 個標籤，未修改任何項目。".into());
                }
            } else {
                tx.execute(
                    "DELETE FROM book_tags WHERE book_id=?1 AND tag_id=?2",
                    params![id, tag_id],
                )
                .map_err(db_error)?;
            }
        }
        tx.commit().map_err(db_error)
    }
}

#[tauri::command]
pub fn edit_library_book(
    id: i64,
    details: BookDetails,
    library: State<Library>,
) -> Result<LibraryBook, String> {
    library.edit_details(id, &details)
}
#[tauri::command]
pub fn set_reading_status(
    ids: Vec<i64>,
    status: String,
    library: State<Library>,
) -> Result<(), String> {
    library.set_status(&ids, &status)
}
#[tauri::command]
pub fn list_tags(library: State<Library>) -> Result<Vec<Tag>, String> {
    library.tags()
}
#[tauri::command]
pub fn create_tag(name: String, library: State<Library>) -> Result<Tag, String> {
    library.create_tag(&name)
}
#[tauri::command]
pub fn rename_tag(id: i64, name: String, library: State<Library>) -> Result<(), String> {
    library.rename_tag(id, &name)
}
#[tauri::command]
pub fn delete_tag(id: i64, library: State<Library>) -> Result<(), String> {
    library.delete_tag(id)
}
#[tauri::command]
pub fn assign_book_tag(
    ids: Vec<i64>,
    tag_id: i64,
    add: bool,
    library: State<Library>,
) -> Result<(), String> {
    library.assign_tag(&ids, tag_id, add)
}

pub(super) fn tags_from_connection(conn: &Connection) -> Result<Vec<Tag>, String> {
    let mut query = conn
        .prepare("SELECT id,name FROM tags ORDER BY name_key")
        .map_err(db_error)?;
    let rows = query
        .query_map([], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })
        .map_err(db_error)?;
    rows.collect::<Result<_, _>>().map_err(db_error)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub book: LibraryBook,
    pub kind: String,
}
#[tauri::command]
pub async fn import_book_result(
    path: String,
    library: State<'_, Library>,
) -> Result<ImportResult, String> {
    let opened = tauri::async_runtime::spawn_blocking(move || book::open(&path))
        .await
        .map_err(db_error)??;
    let (book, created) = library.register_outcome(&opened.book, None)?;
    Ok(ImportResult {
        book,
        kind: if created { "added" } else { "updated" }.into(),
    })
}
