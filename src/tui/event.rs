use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};

use crate::tui::app::{App, AppMode};

pub fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(Duration::from_millis(50))? {
        match event::read()? {
            Event::Key(key) => handle_key_event(app, key)?,
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => match app.mode {
                    AppMode::Normal => {
                        app.select_at_row(mouse.row);
                    }
                    AppMode::Editing => {
                        if let Ok((term_w, term_h)) = crossterm::terminal::size() {
                            let popup_w = term_w * 70 / 100;
                            let popup_h = term_h * 75 / 100;
                            let popup_x = (term_w.saturating_sub(popup_w)) / 2;
                            let popup_y = (term_h.saturating_sub(popup_h)) / 2;

                            let inner_x = popup_x + 2;
                            let inner_y = popup_y + 1;
                            let inner_w = popup_w.saturating_sub(4);
                            let inner_h = popup_h.saturating_sub(2);

                            // title_chunk
                            let title_x = inner_x + 1;
                            let title_y = inner_y + 1;
                            let title_w = inner_w.saturating_sub(2);
                            let title_h = 1;

                            // content_chunk (inner_y + 3 is the border of content block, so inner content starts at +4)
                            let content_x = inner_x + 1;
                            let content_y = inner_y + 4;
                            let content_w = inner_w.saturating_sub(2);
                            let content_h = inner_h.saturating_sub(5);

                            app.click_edit_modal(
                                mouse.column,
                                mouse.row,
                                (title_x, title_y, title_w, title_h),
                                (content_x, content_y, content_w, content_h),
                            );
                        }
                    }
                    _ => {}
                },
                MouseEventKind::ScrollUp => match app.mode {
                    AppMode::Normal => app.scroll_detail_by(-3),
                    AppMode::Editing => app.scroll_edit_viewport(-2),
                    _ => {}
                },
                MouseEventKind::ScrollDown => match app.mode {
                    AppMode::Normal => app.scroll_detail_by(3),
                    AppMode::Editing => app.scroll_edit_viewport(2),
                    _ => {}
                },
                _ => {}
            },
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
    // 处理 Ctrl+D / Ctrl+U 翻页
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('d') => {
                app.scroll_detail_by(6);
                return Ok(());
            }
            KeyCode::Char('u') => {
                app.scroll_detail_by(-6);
                return Ok(());
            }
            _ => {}
        }
    }

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

        // 正文长文上下翻页
        KeyCode::Char('u') | KeyCode::PageUp | KeyCode::Char('[') | KeyCode::Char('K') => {
            app.scroll_detail_up()
        }
        KeyCode::Char(' ') | KeyCode::PageDown | KeyCode::Char(']') | KeyCode::Char('J') => {
            app.scroll_detail_down()
        }

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
    let visible_h = if let Ok((_, term_h)) = crossterm::terminal::size() {
        let popup_h = (term_h * 75 / 100).saturating_sub(2);
        (popup_h.saturating_sub(5) as usize).max(3)
    } else {
        10
    };

    // 处理 Ctrl+S 保存
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        app.save_edit()?;
        return Ok(());
    }

    // 处理 Ctrl+U / Ctrl+D 翻页滚动
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('u') => {
                app.page_scroll_edit(-5, 5);
                return Ok(());
            }
            KeyCode::Char('d') => {
                app.page_scroll_edit(5, 5);
                return Ok(());
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Esc => app.cancel_edit(),
        KeyCode::Tab => app.toggle_edit_focus(),
        KeyCode::BackTab => app.toggle_edit_focus(),

        // 光标导航
        KeyCode::Left => {
            app.move_cursor_left();
            app.ensure_cursor_visible(visible_h);
        }
        KeyCode::Right => {
            app.move_cursor_right();
            app.ensure_cursor_visible(visible_h);
        }
        KeyCode::Up => app.move_cursor_up(),
        KeyCode::Down => app.move_cursor_down(visible_h),
        KeyCode::Home => app.move_cursor_home(),
        KeyCode::End => app.move_cursor_end(),
        KeyCode::PageUp => app.page_scroll_edit(-5, 5),
        KeyCode::PageDown => app.page_scroll_edit(5, 5),

        // 删除与换行
        KeyCode::Backspace => {
            app.delete_backspace();
            app.ensure_cursor_visible(visible_h);
        }
        KeyCode::Delete => {
            app.delete_forward();
            app.ensure_cursor_visible(visible_h);
        }
        KeyCode::Enter => {
            app.insert_newline();
            app.ensure_cursor_visible(visible_h);
        }

        // 字符输入
        KeyCode::Char(c) => {
            app.insert_char(c);
            app.ensure_cursor_visible(visible_h);
        }
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
