use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use directories::ProjectDirs;
use rusqlite::{params, Connection};

use crate::model::Memo;

pub struct Database {
    conn: Connection,
}

impl Database {
    /// 打开默认系统数据路径的数据库，如果不存在则自动创建
    pub fn open_default() -> Result<Self> {
        let db_path = Self::get_default_db_path()?;
        Self::open(&db_path)
    }

    /// 打开指定路径的数据库
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("无法创建数据库目录: {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("无法打开数据库文件: {}", path.display()))?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// 创建一个纯内存数据库（用于单元测试）
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// 获取默认的数据库存储路径 (~/.local/share/memos/memos.db)
    pub fn get_default_db_path() -> Result<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("", "", "memos") {
            let data_dir = proj_dirs.data_dir();
            Ok(data_dir.join("memos.db"))
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            Ok(PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("memos")
                .join("memos.db"))
        }
    }

    /// 初始化数据表与索引
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS memos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_memos_created_at ON memos(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_memos_updated_at ON memos(updated_at DESC);
            ",
        )?;
        Ok(())
    }

    /// 获取最近的 N 条备忘录（默认按创建时间倒序）
    pub fn get_recent(&self, limit: usize) -> Result<Vec<Memo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, created_at, updated_at 
             FROM memos 
             ORDER BY created_at DESC 
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let content: String = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;

            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());

            Ok(Memo {
                id,
                title,
                content,
                created_at,
                updated_at,
            })
        })?;

        let mut memos = Vec::new();
        for row in rows {
            memos.push(row?);
        }
        Ok(memos)
    }

    /// 获取全部备忘录（按更新时间倒序排列）
    pub fn get_all(&self) -> Result<Vec<Memo>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, created_at, updated_at 
             FROM memos 
             ORDER BY updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let content: String = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            let updated_at_str: String = row.get(4)?;

            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());

            Ok(Memo {
                id,
                title,
                content,
                created_at,
                updated_at,
            })
        })?;

        let mut memos = Vec::new();
        for row in rows {
            memos.push(row?);
        }
        Ok(memos)
    }

    /// 插入一条新备忘录
    pub fn insert(&self, title: &str, content: &str) -> Result<Memo> {
        let now = Local::now();
        let now_str = now.to_rfc3339();

        self.conn.execute(
            "INSERT INTO memos (title, content, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![title, content, now_str, now_str],
        )?;

        let id = self.conn.last_insert_rowid();
        Ok(Memo {
            id,
            title: title.to_string(),
            content: content.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// 更新一条备忘录
    pub fn update(&self, id: i64, title: &str, content: &str) -> Result<()> {
        let now = Local::now().to_rfc3339();
        self.conn.execute(
            "UPDATE memos SET title = ?1, content = ?2, updated_at = ?3 WHERE id = ?4",
            params![title, content, now, id],
        )?;
        Ok(())
    }

    /// 删除一条备忘录
    pub fn delete(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM memos WHERE id = ?1", params![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crud_operations() -> Result<()> {
        let db = Database::open_in_memory()?;

        // 插入测试
        let memo1 = db.insert("测试标题1", "测试内容1")?;
        assert_eq!(memo1.id, 1);
        assert_eq!(memo1.title, "测试标题1");

        let memo2 = db.insert("测试标题2", "测试内容2")?;
        assert_eq!(memo2.id, 2);

        // 查全部
        let all = db.get_all()?;
        assert_eq!(all.len(), 2);

        // 查最近
        let recent = db.get_recent(1)?;
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].title, "测试标题2");

        // 更新
        db.update(memo1.id, "修改后标题1", "修改后内容1")?;
        let all_after_update = db.get_all()?;
        let updated = all_after_update.iter().find(|m| m.id == memo1.id).unwrap();
        assert_eq!(updated.title, "修改后标题1");
        assert_eq!(updated.content, "修改后内容1");

        // 删除
        db.delete(memo2.id)?;
        let all_after_delete = db.get_all()?;
        assert_eq!(all_after_delete.len(), 1);
        assert_eq!(all_after_delete[0].id, memo1.id);

        Ok(())
    }
}
