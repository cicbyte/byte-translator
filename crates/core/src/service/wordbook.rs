//! 生词本：主动收藏（幂等）、备注、列表、删除。REST / CLI / MCP 共用。

use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::Serialize;

use crate::db::Database;

/// 生词条目（对外视图）
#[derive(Debug, Clone, Serialize)]
pub struct WordEntry {
    pub id: i64,
    pub text: String,
    pub note: String,
    pub created_at: String,
}

pub struct WordbookService {
    db: Arc<Database>,
}

impl WordbookService {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// 收藏词条（幂等）：已收藏则更新备注，未收藏则新增
    pub fn add(&self, text: &str, note: &str) -> Result<()> {
        let text = text.trim();
        if text.is_empty() {
            return Err(anyhow!("词条不能为空"));
        }
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
        conn.execute(
            "INSERT INTO wordbook (text, note) VALUES (?1, ?2) \
             ON CONFLICT (text) DO UPDATE SET note=excluded.note",
            rusqlite::params![text, note],
        )?;
        Ok(())
    }

    /// 列表（可选关键词过滤，按收藏时间倒序 + 分页），返回 (条目, 总数)
    pub fn list(&self, keyword: &str, page: i64, size: i64) -> Result<(Vec<WordEntry>, i64)> {
        let page = page.max(1);
        let size = size.clamp(1, 100);
        let offset = (page - 1) * size;
        let kw = keyword.trim();
        let escaped = kw.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;

        let (count_sql, list_sql, has_kw) = if kw.is_empty() {
            (
                "SELECT COUNT(*) FROM wordbook".to_string(),
                format!(
                    "SELECT id, text, note, created_at FROM wordbook \
                     ORDER BY id DESC LIMIT {size} OFFSET {offset}"
                ),
                false,
            )
        } else {
            (
                "SELECT COUNT(*) FROM wordbook WHERE text LIKE ?1 ESCAPE '\\'"
                    .to_string(),
                format!(
                    "SELECT id, text, note, created_at FROM wordbook \
                     WHERE text LIKE ?1 ESCAPE '\\' ORDER BY id DESC LIMIT {size} OFFSET {offset}"
                ),
                true,
            )
        };

        let total: i64 = if has_kw {
            conn.query_row(&count_sql, rusqlite::params![format!("%{escaped}%")], |r| r.get(0))?
        } else {
            conn.query_row(&count_sql, [], |r| r.get(0))?
        };

        let mut stmt = conn.prepare(&list_sql)?;
        let map_row = |r: &rusqlite::Row| -> std::result::Result<WordEntry, rusqlite::Error> {
            Ok(WordEntry {
                id: r.get(0)?,
                text: r.get(1)?,
                note: r.get(2)?,
                created_at: r.get(3)?,
            })
        };
        let rows = if has_kw {
            stmt.query_map(rusqlite::params![format!("%{escaped}%")], map_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        } else {
            stmt.query_map([], map_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };
        Ok((rows, total))
    }

    /// 删除词条（按 id）
    pub fn remove(&self, id: i64) -> Result<()> {
        let conn = self.db.lock().map_err(|_| anyhow!("数据库锁中毒"))?;
        let affected = conn.execute("DELETE FROM wordbook WHERE id=?1", rusqlite::params![id])?;
        if affected == 0 {
            return Err(anyhow!("生词不存在: id={id}"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service() -> WordbookService {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("wordbook-test.db");
        std::mem::forget(tmp);
        WordbookService::new(Arc::new(Database::open(&path).unwrap()))
    }

    #[test]
    fn add_is_idempotent_updating_note() {
        let svc = service();
        svc.add("love", "n. 爱").unwrap();
        svc.add("love", "v. 热爱（更新备注）").unwrap();
        let (list, total) = svc.list("", 1, 10).unwrap();
        assert_eq!(total, 1, "重复收藏不新增条目");
        assert_eq!(list[0].note, "v. 热爱（更新备注）", "重复收藏更新备注");
    }

    #[test]
    fn empty_text_rejected() {
        let svc = service();
        assert!(svc.add("  ", "").is_err());
    }

    #[test]
    fn list_keyword_filter() {
        let svc = service();
        svc.add("love", "").unwrap();
        svc.add("beloved", "").unwrap();
        svc.add("你好", "").unwrap();
        let (list, total) = svc.list("love", 1, 10).unwrap();
        assert_eq!(total, 2);
        assert!(list.iter().all(|w| w.text.contains("love")));
    }

    #[test]
    fn remove_missing_errors() {
        let svc = service();
        svc.add("love", "").unwrap();
        let (list, _) = svc.list("", 1, 10).unwrap();
        svc.remove(list[0].id).unwrap();
        assert!(svc.remove(list[0].id).is_err(), "重复删除报错");
    }
}
