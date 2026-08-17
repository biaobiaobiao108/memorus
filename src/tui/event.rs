use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};

use crate::tui::app::{App, AppMode, EditFocus};

pub fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(Duration::from_millis(50))? {
        match event::read()? {
            Event::Key(key) => handle_key_event(app, key)?,
            Event::Mouse(mouse) => {
                if app.mode == AppMode::Normal {
                    if let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                        app.select_at_row(mouse.row);
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
    match app.mode {
        AppMode::Normal => handle_normal_mode_key(app, key)?,
        AppMode::Editing => handle_editing_mode_key(app, key)?,
        AppMode::DeleteConfirm => handle_delete_confirm_mode_key(app, key)?,
        AppMode::Searching => handle_searching_mode_key(app, key),
    }
    Ok(())
}

fn handle_normal_mode_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                app.clear_search();
            } else {
                app.should_quit = true;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char('u') | KeyCode::PageUp => app.scroll_detail_up(),
        KeyCode::Char(' ') | KeyCode::PageDown => app.scroll_detail_down(),

        KeyCode::Tab => app.toggle_list_filter(),
        KeyCode::Char('g') | KeyCode::Char('G') => app.toggle_archive()?,

        KeyCode::Char('a') | KeyCode::Char('n') => app.start_create(),
        KeyCode::Char('e') | KeyCode::Enter => app.start_edit(),
        KeyCode::Char('d') | KeyCode::Delete => app.prompt_delete(),
        KeyCode::Char('/') => app.start_search(),
        _ => {}
    }
    Ok(())
}

fn handle_editing_mode_key(app: &mut App, key: KeyEvent) -> Result<()> {
    // 处理 Ctrl+S 保存
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        app.save_edit()?;
        return Ok(());
    }

    match key.code {
        KeyCode::Esc => app.cancel_edit(),
        KeyCode::Tab => app.toggle_edit_focus(),
        KeyCode::BackTab => app.toggle_edit_focus(),
        KeyCode::Backspace => match app.edit_focus {
            EditFocus::Title => {
                app.edit_title.pop();
            }
            EditFocus::Content => {
                app.edit_content.pop();
            }
        },
        KeyCode::Enter => match app.edit_focus {
            EditFocus::Title => {
                // 标题输入完成后按 Enter 自动跳到内容框
                app.edit_focus = EditFocus::Content;
            }
            EditFocus::Content => {
                // 多行内容换行
                app.edit_content.push('\n');
            }
        },
        KeyCode::Char(c) => match app.edit_focus {
            EditFocus::Title => app.edit_title.push(c),
            EditFocus::Content => app.edit_content.push(c),
        },
        _ => {}
    }
    Ok(())
}

fn handle_delete_confirm_mode_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            app.confirm_delete()?;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_delete();
        }
        _ => {}
    }
    Ok(())
}

fn handle_searching_mode_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            // 锁定当前搜索词，返回正常浏览模式
            app.mode = AppMode::Normal;
        }
        KeyCode::Esc => {
            app.clear_search();
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_filter();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.apply_filter();
        }
        _ => {}
    }
}
