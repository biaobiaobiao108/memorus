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
    pub edit_scroll: u16,
    pub list_filter: ListFilter,

    pub mode: AppMode,
    pub edit_target_id: Option<i64>,
    pub edit_title: String,
    pub title_cursor: usize,
    pub edit_lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
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
            edit_scroll: 0,
            list_filter: ListFilter::Active,

            mode: AppMode::Normal,
            edit_target_id: None,
            edit_title: String::new(),
            title_cursor: 0,
            edit_lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
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
        self.scroll_detail_by(-3);
    }

    pub fn scroll_detail_down(&mut self) {
        self.scroll_detail_by(3);
    }

    pub fn scroll_detail_by(&mut self, delta: i16) {
        if delta < 0 {
            self.detail_scroll = self.detail_scroll.saturating_sub((-delta) as u16);
        } else {
            self.detail_scroll = self.detail_scroll.saturating_add(delta as u16);
        }
    }

    /// 鼠标滚轮仅平滑滚动视口，不破坏光标所在位置
    pub fn scroll_edit_viewport(&mut self, delta: i16) {
        let total_lines = self.edit_lines.len();
        if total_lines == 0 {
            return;
        }

        if delta < 0 {
            let abs_delta = (-delta) as u16;
            self.edit_scroll = self.edit_scroll.saturating_sub(abs_delta);
        } else {
            let step = delta as usize;
            let max_scroll = total_lines.saturating_sub(1);
            self.edit_scroll = (self.edit_scroll as usize + step).min(max_scroll) as u16;
        }
    }

    /// 键盘 PageUp / PageDown 翻页：视口与光标同步平移
    pub fn page_scroll_edit(&mut self, delta: i16, page_size: usize) {
        let total_lines = self.edit_lines.len();
        if total_lines == 0 {
            return;
        }
        let step = page_size.max(1);

        if delta < 0 {
            self.edit_scroll = self.edit_scroll.saturating_sub(step as u16);
            if self.edit_focus == EditFocus::Content {
                self.cursor_row = self.cursor_row.saturating_sub(step);
                let line_len = self.edit_lines[self.cursor_row].chars().count();
                self.cursor_col = self.cursor_col.min(line_len);
            }
        } else {
            let max_scroll = total_lines.saturating_sub(1);
            self.edit_scroll = (self.edit_scroll as usize + step).min(max_scroll) as u16;
            if self.edit_focus == EditFocus::Content {
                self.cursor_row = (self.cursor_row + step).min(total_lines.saturating_sub(1));
                let line_len = self.edit_lines[self.cursor_row].chars().count();
                self.cursor_col = self.cursor_col.min(line_len);
            }
        }
    }

    /// 保证光标在可视区域内（在打字或修改内容时自动调用）
    pub fn ensure_cursor_visible(&mut self, visible_h: usize) {
        if self.edit_focus != EditFocus::Content || visible_h == 0 {
            return;
        }
        let scroll = self.edit_scroll as usize;
        if self.cursor_row < scroll {
            self.edit_scroll = self.cursor_row as u16;
        } else if self.cursor_row >= scroll + visible_h {
            self.edit_scroll = (self.cursor_row.saturating_sub(visible_h) + 1) as u16;
        }
    }

    // --- 编辑器核心方法 ---

    pub fn start_create(&mut self) {
        self.mode = AppMode::Editing;
        self.edit_target_id = None;
        self.edit_title.clear();
        self.title_cursor = 0;
        self.edit_lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.edit_focus = EditFocus::Title;
        self.edit_scroll = 0;
    }

    pub fn start_edit(&mut self) {
        if let Some(memo) = self.selected_memo().cloned() {
            self.mode = AppMode::Editing;
            self.edit_target_id = Some(memo.id);
            self.edit_title = memo.title;
            self.title_cursor = self.edit_title.chars().count();
            
            let lines: Vec<String> = memo.content.lines().map(|s| s.to_string()).collect();
            self.edit_lines = if lines.is_empty() {
                vec![String::new()]
            } else {
                lines
            };
            self.cursor_row = self.edit_lines.len().saturating_sub(1);
            self.cursor_col = self.edit_lines[self.cursor_row].chars().count();
            self.edit_focus = EditFocus::Title;
            self.edit_scroll = 0;
        } else {
            self.set_status("⚠️ 当前没有选中的备忘录可供编辑");
        }
    }

    pub fn get_combined_content(&self) -> String {
        self.edit_lines.join("\n")
    }

    pub fn save_edit(&mut self) -> Result<()> {
        let title = self.edit_title.trim();
        if title.is_empty() {
            self.set_status("⚠️ 标题不能为空！");
            return Ok(());
        }

        let content = self.get_combined_content();

        if let Some(id) = self.edit_target_id {
            self.db.update(id, title, &content)?;
            self.set_status("✅ 备忘录修改已保存");
        } else {
            let title = title.to_string();
            self.db.insert(&title, &content)?;
            self.set_status("✅ 新建备忘录成功");
        }

        self.mode = AppMode::Normal;
        self.reload_memos()?;
        Ok(())
    }

    pub fn cancel_edit(&mut self) {
        self.mode = AppMode::Normal;
        self.edit_title.clear();
        self.title_cursor = 0;
        self.edit_lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.edit_target_id = None;
    }

    pub fn toggle_edit_focus(&mut self) {
        self.edit_focus = match self.edit_focus {
            EditFocus::Title => EditFocus::Content,
            EditFocus::Content => EditFocus::Title,
        };
    }

    // --- 光标移动与文本编辑动作 ---

    pub fn insert_char(&mut self, c: char) {
        match self.edit_focus {
            EditFocus::Title => {
                let mut chars: Vec<char> = self.edit_title.chars().collect();
                let idx = self.title_cursor.min(chars.len());
                chars.insert(idx, c);
                self.edit_title = chars.into_iter().collect();
                self.title_cursor += 1;
            }
            EditFocus::Content => {
                if self.edit_lines.is_empty() {
                    self.edit_lines.push(String::new());
                }
                let row = self.cursor_row.min(self.edit_lines.len().saturating_sub(1));
                let mut chars: Vec<char> = self.edit_lines[row].chars().collect();
                let col = self.cursor_col.min(chars.len());
                chars.insert(col, c);
                self.edit_lines[row] = chars.into_iter().collect();
                self.cursor_col += 1;
            }
        }
    }

    pub fn insert_newline(&mut self) {
        match self.edit_focus {
            EditFocus::Title => {
                self.edit_focus = EditFocus::Content;
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            EditFocus::Content => {
                if self.edit_lines.is_empty() {
                    self.edit_lines.push(String::new());
                }
                let row = self.cursor_row.min(self.edit_lines.len().saturating_sub(1));
                let chars: Vec<char> = self.edit_lines[row].chars().collect();
                let col = self.cursor_col.min(chars.len());

                let before: String = chars[..col].iter().collect();
                let after: String = chars[col..].iter().collect();

                self.edit_lines[row] = before;
                self.edit_lines.insert(row + 1, after);
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
        }
    }

    pub fn delete_backspace(&mut self) {
        match self.edit_focus {
            EditFocus::Title => {
                if self.title_cursor > 0 {
                    let mut chars: Vec<char> = self.edit_title.chars().collect();
                    let idx = self.title_cursor.min(chars.len());
                    chars.remove(idx - 1);
                    self.edit_title = chars.into_iter().collect();
                    self.title_cursor -= 1;
                }
            }
            EditFocus::Content => {
                if self.edit_lines.is_empty() {
                    return;
                }
                let row = self.cursor_row.min(self.edit_lines.len().saturating_sub(1));
                if self.cursor_col > 0 {
                    let mut chars: Vec<char> = self.edit_lines[row].chars().collect();
                    let col = self.cursor_col.min(chars.len());
                    chars.remove(col - 1);
                    self.edit_lines[row] = chars.into_iter().collect();
                    self.cursor_col -= 1;
                } else if row > 0 {
                    // 与上一行合并
                    let current = self.edit_lines.remove(row);
                    let prev_len = self.edit_lines[row - 1].chars().count();
                    self.edit_lines[row - 1].push_str(&current);
                    self.cursor_row = row - 1;
                    self.cursor_col = prev_len;
                }
            }
        }
    }

    pub fn delete_forward(&mut self) {
        match self.edit_focus {
            EditFocus::Title => {
                let mut chars: Vec<char> = self.edit_title.chars().collect();
                if self.title_cursor < chars.len() {
                    chars.remove(self.title_cursor);
                    self.edit_title = chars.into_iter().collect();
                }
            }
            EditFocus::Content => {
                if self.edit_lines.is_empty() {
                    return;
                }
                let row = self.cursor_row.min(self.edit_lines.len().saturating_sub(1));
                let mut chars: Vec<char> = self.edit_lines[row].chars().collect();
                if self.cursor_col < chars.len() {
                    chars.remove(self.cursor_col);
                    self.edit_lines[row] = chars.into_iter().collect();
                } else if row + 1 < self.edit_lines.len() {
                    // 与下一行合并
                    let next = self.edit_lines.remove(row + 1);
                    self.edit_lines[row].push_str(&next);
                }
            }
        }
    }

    pub fn move_cursor_left(&mut self) {
        match self.edit_focus {
            EditFocus::Title => {
                self.title_cursor = self.title_cursor.saturating_sub(1);
            }
            EditFocus::Content => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                } else if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    self.cursor_col = self.edit_lines[self.cursor_row].chars().count();
                }
            }
        }
    }

    pub fn move_cursor_right(&mut self) {
        match self.edit_focus {
            EditFocus::Title => {
                let max = self.edit_title.chars().count();
                if self.title_cursor < max {
                    self.title_cursor += 1;
                }
            }
            EditFocus::Content => {
                if self.edit_lines.is_empty() {
                    return;
                }
                let row = self.cursor_row.min(self.edit_lines.len().saturating_sub(1));
                let line_len = self.edit_lines[row].chars().count();
                if self.cursor_col < line_len {
                    self.cursor_col += 1;
                } else if self.cursor_row + 1 < self.edit_lines.len() {
                    self.cursor_row += 1;
                    self.cursor_col = 0;
                }
            }
        }
    }

    pub fn move_cursor_up(&mut self) {
        match self.edit_focus {
            EditFocus::Title => {}
            EditFocus::Content => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                    let line_len = self.edit_lines[self.cursor_row].chars().count();
                    self.cursor_col = self.cursor_col.min(line_len);
                    if self.cursor_row < self.edit_scroll as usize {
                        self.edit_scroll = self.cursor_row as u16;
                    }
                } else {
                    self.edit_focus = EditFocus::Title;
                }
            }
        }
    }

    pub fn move_cursor_down(&mut self, visible_h: usize) {
        let h = if visible_h > 0 { visible_h } else { 6 };
        match self.edit_focus {
            EditFocus::Title => {
                self.edit_focus = EditFocus::Content;
                if !self.edit_lines.is_empty() {
                    self.cursor_col = self.cursor_col.min(self.edit_lines[0].chars().count());
                }
            }
            EditFocus::Content => {
                if self.cursor_row + 1 < self.edit_lines.len() {
                    self.cursor_row += 1;
                    let line_len = self.edit_lines[self.cursor_row].chars().count();
                    self.cursor_col = self.cursor_col.min(line_len);
                    if self.cursor_row >= (self.edit_scroll as usize) + h {
                        self.edit_scroll = (self.cursor_row.saturating_sub(h) + 1) as u16;
                    }
                }
            }
        }
    }

    pub fn move_cursor_home(&mut self) {
        match self.edit_focus {
            EditFocus::Title => self.title_cursor = 0,
            EditFocus::Content => self.cursor_col = 0,
        }
    }

    pub fn move_cursor_end(&mut self) {
        match self.edit_focus {
            EditFocus::Title => self.title_cursor = self.edit_title.chars().count(),
            EditFocus::Content => {
                if !self.edit_lines.is_empty() {
                    let row = self.cursor_row.min(self.edit_lines.len().saturating_sub(1));
                    self.cursor_col = self.edit_lines[row].chars().count();
                }
            }
        }
    }

    /// 处理在编辑弹窗中的鼠标点击定位光标
    pub fn click_edit_modal(&mut self, click_x: u16, click_y: u16, title_inner: (u16, u16, u16, u16), content_inner: (u16, u16, u16, u16)) {
        let (tx, ty, tw, th) = title_inner;
        let (cx, cy, cw, ch) = content_inner;

        // 点击在标题框
        if click_x >= tx && click_x < tx + tw && click_y >= ty && click_y < ty + th {
            self.edit_focus = EditFocus::Title;
            let rel_x = click_x.saturating_sub(tx) as usize;
            let mut acc_w = 0;
            let mut target_idx = 0;
            for (idx, c) in self.edit_title.chars().enumerate() {
                let char_w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
                if acc_w + char_w / 2 >= rel_x {
                    break;
                }
                acc_w += char_w;
                target_idx = idx + 1;
            }
            self.title_cursor = target_idx.min(self.edit_title.chars().count());
            return;
        }

        // 点击在内容框
        if click_x >= cx && click_x < cx + cw && click_y >= cy && click_y < cy + ch {
            self.edit_focus = EditFocus::Content;
            let rel_y = click_y.saturating_sub(cy);
            let target_row = (self.edit_scroll as usize) + (rel_y as usize);

            if target_row < self.edit_lines.len() {
                self.cursor_row = target_row;
            } else if !self.edit_lines.is_empty() {
                self.cursor_row = self.edit_lines.len() - 1;
            } else {
                self.cursor_row = 0;
                self.edit_lines.push(String::new());
            }

            let rel_x = click_x.saturating_sub(cx) as usize;
            let mut acc_w = 0;
            let mut target_col = 0;
            let line = &self.edit_lines[self.cursor_row];
            for (idx, c) in line.chars().enumerate() {
                let char_w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1);
                if acc_w + char_w / 2 >= rel_x {
                    break;
                }
                acc_w += char_w;
                target_col = idx + 1;
            }
            self.cursor_col = target_col.min(line.chars().count());
        }
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
        for c in "第一条备忘录".chars() {
            app.insert_char(c);
        }
        app.insert_newline();
        for c in "第一行内容".chars() {
            app.insert_char(c);
        }
        app.insert_newline();
        for c in "第二行内容".chars() {
            app.insert_char(c);
        }
        app.save_edit()?;

        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.memos.len(), 1);
        assert_eq!(app.selected_memo().unwrap().title, "第一条备忘录");
        assert_eq!(app.selected_memo().unwrap().content, "第一行内容\n第二行内容");

        // 2. 新建第二条
        app.start_create();
        for c in "Rust TUI 学习".chars() {
            app.insert_char(c);
        }
        app.insert_newline();
        for c in "深入 Ratatui 框架".chars() {
            app.insert_char(c);
        }
        app.save_edit()?;

        assert_eq!(app.memos.len(), 2);

        // 3. 搜索过滤
        app.search_query = "Rust".to_string();
        app.apply_filter();
        assert_eq!(app.filtered_indices.len(), 1);
        assert_eq!(app.selected_memo().unwrap().title, "Rust TUI 学习");

        app.clear_search();
        assert_eq!(app.filtered_indices.len(), 2);

        // 4. 编辑现有并测试光标移动与修改
        app.selected_index = 0;
        app.start_edit();
        assert_eq!(app.mode, AppMode::Editing);
        assert!(app.edit_target_id.is_some());
        
        // 移动光标并修改
        app.move_cursor_home();
        app.insert_char('★');
        app.save_edit()?;

        assert!(app.selected_memo().unwrap().title.starts_with('★'));

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

    #[test]
    fn test_editor_cursor_and_mouse_click() -> Result<()> {
        let db = Database::open_in_memory()?;
        let mut app = App::new(db)?;

        app.start_create();
        for c in "Hello Rust".chars() {
            app.insert_char(c);
        }
        assert_eq!(app.edit_title, "Hello Rust");
        assert_eq!(app.title_cursor, 10);

        // 左右移动
        app.move_cursor_left();
        app.move_cursor_left();
        assert_eq!(app.title_cursor, 8);
        app.insert_char('!');
        assert_eq!(app.edit_title, "Hello Ru!st");

        // Backspace
        app.delete_backspace();
        assert_eq!(app.edit_title, "Hello Rust");

        // 换行进入正文
        app.insert_newline();
        assert_eq!(app.edit_focus, EditFocus::Content);
        for c in "Line 1".chars() {
            app.insert_char(c);
        }
        app.insert_newline();
        for c in "Line 2".chars() {
            app.insert_char(c);
        }
        assert_eq!(app.cursor_row, 1);
        assert_eq!(app.cursor_col, 6);

        // 向上移动光标
        app.move_cursor_up();
        assert_eq!(app.cursor_row, 0);

        // 鼠标点击测试：点击标题框 (x=15, y=5)
        app.click_edit_modal(15, 5, (10, 5, 20, 1), (10, 8, 20, 10));
        assert_eq!(app.edit_focus, EditFocus::Title);

        // 鼠标点击测试：点击内容框第二行 (x=13, y=9)
        app.click_edit_modal(13, 9, (10, 5, 20, 1), (10, 8, 20, 10));
        assert_eq!(app.edit_focus, EditFocus::Content);
        assert_eq!(app.cursor_row, 1);

        // 滚轮独立滚动视口测试：添加 30 行多行文本，滚轮只滚动视口，不强行移动光标
        app.edit_lines = (0..30).map(|i| format!("Line #{i}")).collect();
        app.cursor_row = 2;
        app.edit_scroll = 0;

        app.scroll_edit_viewport(5);
        assert_eq!(app.edit_scroll, 5);
        assert_eq!(app.cursor_row, 2); // 光标位置保持不变

        // 键盘翻页测试：视口与光标同步平移
        app.page_scroll_edit(5, 5);
        assert_eq!(app.edit_scroll, 10);
        assert_eq!(app.cursor_row, 7);

        // 输入字符时自动将视口拉回到光标所在处
        app.ensure_cursor_visible(6);
        assert!(app.edit_scroll <= app.cursor_row as u16);

        Ok(())
    }
}

