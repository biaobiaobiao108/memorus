use std::time::{Duration, Instant};

use anyhow::Result;

use crate::db::Database;
use crate::model::Memo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFilter {
    Active,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Editing,
    DeleteConfirm,
    Searching,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditFocus {
    Title,
    Content,
}

pub struct App {
    pub db: Database,
    pub memos: Vec<Memo>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub detail_scroll: u16,
    pub list_filter: ListFilter,

    pub mode: AppMode,
    pub edit_target_id: Option<i64>,
    pub edit_title: String,
    pub edit_content: String,
    pub edit_focus: EditFocus,

    pub search_query: String,
    pub status_message: Option<(String, Instant)>,
    pub should_quit: bool,
}

impl App {
    pub fn new(db: Database) -> Result<Self> {
        let mut app = Self {
            db,
            memos: Vec::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            detail_scroll: 0,
            list_filter: ListFilter::Active,

            mode: AppMode::Normal,
            edit_target_id: None,
            edit_title: String::new(),
            edit_content: String::new(),
            edit_focus: EditFocus::Title,

            search_query: String::new(),
            status_message: None,
            should_quit: false,
        };
        app.reload_memos()?;
        Ok(app)
    }

    /// 重新从数据库拉取备忘录数据
    pub fn reload_memos(&mut self) -> Result<()> {
        self.memos = self.db.get_all()?;
        self.apply_filter();
        Ok(())
    }

    /// 根据当前归档过滤器和搜索词过滤列表
    pub fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        let target_archived = self.list_filter == ListFilter::Archived;

        self.filtered_indices = self
            .memos
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                if m.archived != target_archived {
                    return false;
                }
                if query.trim().is_empty() {
                    true
                } else {
                    m.title.to_lowercase().contains(&query)
                        || m.content.to_lowercase().contains(&query)
                }
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.filtered_indices.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.filtered_indices.len() {
            self.selected_index = self.filtered_indices.len().saturating_sub(1);
        }
        self.detail_scroll = 0;
    }

    pub fn toggle_list_filter(&mut self) {
        self.list_filter = match self.list_filter {
            ListFilter::Active => ListFilter::Archived,
            ListFilter::Archived => ListFilter::Active,
        };
        self.apply_filter();
        self.selected_index = 0;
    }

    pub fn toggle_archive(&mut self) -> Result<()> {
        if let Some(memo) = self.selected_memo().cloned() {
            let target_archived = !memo.archived;
            self.db.set_archived(memo.id, target_archived)?;
            if target_archived {
                self.set_status(format!("📦 备忘录《{}》已归档", memo.title));
            } else {
                self.set_status(format!("📥 备忘录《{}》已取消归档", memo.title));
            }
            self.reload_memos()?;
        } else {
            self.set_status("⚠️ 当前没有选中的备忘录可操作");
        }
        Ok(())
    }

    pub fn select_at_row(&mut self, row: u16) {
        // 主体双栏内容区从 y = 1 开始，列表上方有 border (y = 1)，列表项从 y = 2 开始
        if row >= 2 {
            let item_idx = (row - 2) as usize;
            if item_idx < self.filtered_indices.len() {
                self.selected_index = item_idx;
                self.detail_scroll = 0;
            }
        }
    }

    /// 获取当前选中的备忘录引用
    pub fn selected_memo(&self) -> Option<&Memo> {
        if self.filtered_indices.is_empty() {
            None
        } else {
            let real_idx = self.filtered_indices.get(self.selected_index)?;
            self.memos.get(*real_idx)
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), Instant::now()));
    }

    pub fn get_active_status(&self) -> Option<&str> {
        if let Some((msg, time)) = &self.status_message {
            if time.elapsed() < Duration::from_secs(4) {
                return Some(msg.as_str());
            }
        }
        None
    }

    // --- 导航操作 ---

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.detail_scroll = 0;
        }
    }

    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() && self.selected_index + 1 < self.filtered_indices.len() {
            self.selected_index += 1;
            self.detail_scroll = 0;
        }
    }

    pub fn scroll_detail_up(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_sub(1);
    }

    pub fn scroll_detail_down(&mut self) {
        self.detail_scroll = self.detail_scroll.saturating_add(1);
    }

    // --- 新建与编辑 ---

    pub fn start_create(&mut self) {
        self.mode = AppMode::Editing;
        self.edit_target_id = None;
        self.edit_title.clear();
        self.edit_content.clear();
        self.edit_focus = EditFocus::Title;
    }

    pub fn start_edit(&mut self) {
        if let Some(memo) = self.selected_memo().cloned() {
            self.mode = AppMode::Editing;
            self.edit_target_id = Some(memo.id);
            self.edit_title = memo.title;
            self.edit_content = memo.content;
            self.edit_focus = EditFocus::Title;
        } else {
            self.set_status("⚠️ 当前没有选中的备忘录可供编辑");
        }
    }

    pub fn save_edit(&mut self) -> Result<()> {
        let title = self.edit_title.trim();
        if title.is_empty() {
            self.set_status("⚠️ 标题不能为空！");
            return Ok(());
        }

        if let Some(id) = self.edit_target_id {
            self.db.update(id, title, &self.edit_content)?;
            self.set_status("✅ 备忘录修改已保存");
        } else {
            let title = title.to_string();
            self.db.insert(&title, &self.edit_content)?;
            self.set_status("✅ 新建备忘录成功");
        }

        self.mode = AppMode::Normal;
        self.reload_memos()?;
        Ok(())
    }

    pub fn cancel_edit(&mut self) {
        self.mode = AppMode::Normal;
        self.edit_title.clear();
        self.edit_content.clear();
        self.edit_target_id = None;
    }

    pub fn toggle_edit_focus(&mut self) {
        self.edit_focus = match self.edit_focus {
            EditFocus::Title => EditFocus::Content,
            EditFocus::Content => EditFocus::Title,
        };
    }

    // --- 删除操作 ---

    pub fn prompt_delete(&mut self) {
        if self.selected_memo().is_some() {
            self.mode = AppMode::DeleteConfirm;
        } else {
            self.set_status("⚠️ 当前没有可删除的备忘录");
        }
    }

    pub fn confirm_delete(&mut self) -> Result<()> {
        if let Some(id) = self.selected_memo().map(|m| m.id) {
            self.db.delete(id)?;
            self.set_status("🗑️ 备忘录已删除");
            self.mode = AppMode::Normal;
            self.reload_memos()?;
        }
        Ok(())
    }

    pub fn cancel_delete(&mut self) {
        self.mode = AppMode::Normal;
    }

    // --- 搜索 ---

    pub fn start_search(&mut self) {
        self.mode = AppMode::Searching;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.apply_filter();
        self.mode = AppMode::Normal;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_flow() -> Result<()> {
        let db = Database::open_in_memory()?;
        let mut app = App::new(db)?;

        // 初始为空
        assert!(app.memos.is_empty());
        assert!(app.selected_memo().is_none());

        // 1. 新建
        app.start_create();
        assert_eq!(app.mode, AppMode::Editing);
        app.edit_title = "第一条备忘录".to_string();
        app.edit_content = "第一行内容\n第二行内容".to_string();
        app.save_edit()?;

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.memos.len(), 1);
        assert_eq!(app.selected_memo().unwrap().title, "第一条备忘录");

        // 2. 新建第二条
        app.start_create();
        app.edit_title = "Rust TUI 学习".to_string();
        app.edit_content = "深入 Ratatui 框架".to_string();
        app.save_edit()?;

        assert_eq!(app.memos.len(), 2);

        // 3. 搜索过滤
        app.search_query = "Rust".to_string();
        app.apply_filter();
        assert_eq!(app.filtered_indices.len(), 1);
        assert_eq!(app.selected_memo().unwrap().title, "Rust TUI 学习");

        app.clear_search();
        assert_eq!(app.filtered_indices.len(), 2);

        // 4. 编辑现有
        app.selected_index = 0;
        app.start_edit();
        assert_eq!(app.mode, AppMode::Editing);
        assert!(app.edit_target_id.is_some());
        app.edit_title = "修改后的标题".to_string();
        app.save_edit()?;

        assert_eq!(app.selected_memo().unwrap().title, "修改后的标题");

        // 5. 归档测试
        app.toggle_archive()?;
        assert_eq!(app.filtered_indices.len(), 1); // 活动列表中只剩 1 条

        // Tab 切换到归档列表
        app.toggle_list_filter();
        assert_eq!(app.list_filter, ListFilter::Archived);
        assert_eq!(app.filtered_indices.len(), 1); // 归档列表中有 1 条

        // 取消归档
        app.toggle_archive()?;
        assert_eq!(app.filtered_indices.len(), 0); // 归档列表变空
        app.toggle_list_filter();
        assert_eq!(app.filtered_indices.len(), 2); // 活动列表恢复 2 条

        // 6. 删除
        app.prompt_delete();
        assert_eq!(app.mode, AppMode::DeleteConfirm);
        app.confirm_delete()?;
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.memos.len(), 1);

        Ok(())
    }
}

