use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
    },
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::tui::app::{App, AppMode, EditFocus};

pub fn draw(f: &mut Frame, app: &mut App) {
    if app.mode == AppMode::Editing {
        draw_editor_page(f, app, f.area());
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 顶部标题栏
            Constraint::Min(10),   // 主体双栏
            Constraint::Length(2), // 底部操作/状态栏
        ])
        .split(f.area());

    // 1. 顶部标题条
    draw_header(f, app, chunks[0]);

    // 2. 主体左右分栏
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(chunks[1]);

    draw_list(f, app, main_chunks[0]);
    draw_detail(f, app, main_chunks[1]);

    // 3. 底部状态与快捷键提示
    draw_footer(f, app, chunks[2]);

    // 4. 浮层弹窗 (仅删除确认等需要小弹窗的模式)
    if app.mode == AppMode::DeleteConfirm {
        draw_delete_confirm_modal(f, app);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let active_count = app.memos.iter().filter(|m| !m.archived).count();
    let archived_count = app.memos.len().saturating_sub(active_count);

    let header_text = Line::from(vec![
        Span::styled(" 📝 MEMOS ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" 极速终端备忘录"),
        Span::styled("   📝 活动: ", Style::default().fg(Color::Yellow)),
        Span::styled(active_count.to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::styled("   📦 归档: ", Style::default().fg(Color::Green)),
        Span::styled(archived_count.to_string(), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(header_text), area);
}

fn draw_list(f: &mut Frame, app: &App, area: Rect) {
    let base_title = match app.list_filter {
        crate::tui::app::ListFilter::Active => " 📝 备忘录列表",
        crate::tui::app::ListFilter::Archived => " 📦 已归档备忘录",
    };

    let list_title = if app.search_query.is_empty() {
        format!(" {} ({}) ", base_title, app.filtered_indices.len())
    } else {
        format!(" {} 搜索结果 ({}/{}) ", base_title, app.filtered_indices.len(), app.memos.len())
    };

    let items: Vec<ListItem> = app
        .filtered_indices
        .iter()
        .enumerate()
        .filter_map(|(ui_idx, &real_idx)| {
            let memo = app.memos.get(real_idx)?;
            let time_str = memo.updated_at.format("%m-%d %H:%M").to_string();
            let is_selected = ui_idx == app.selected_index;

            let prefix = if is_selected { "▶ " } else { "  " };
            let icon = if memo.archived { "📦 " } else { "📝 " };
            let title_display = if memo.title.chars().count() > 16 {
                let s: String = memo.title.chars().take(14).collect();
                format!("{}...", s)
            } else {
                memo.title.clone()
            };

            let title_style = if memo.archived {
                Style::default().fg(Color::DarkGray)
            } else if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let line = Line::from(vec![
                Span::styled(
                    prefix,
                    if is_selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ),
                Span::raw(icon),
                Span::styled(
                    format!("[{}] ", time_str),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(title_display, title_style),
            ]);

            Some(ListItem::new(line))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            list_title,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));

    if items.is_empty() {
        let empty_tip = if app.list_filter == crate::tui::app::ListFilter::Archived {
            "📦 暂无归档备忘录\n按 [Tab] 返回活动列表"
        } else if app.memos.is_empty() {
            "📭 暂无备忘录\n按 [a] 创建第一条"
        } else {
            "未找到匹配的备忘录\n按 [Esc] 清除搜索"
        };
        let p = Paragraph::new(empty_tip)
            .block(list_block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
    } else {
        let mut state = ListState::default();
        state.select(Some(app.selected_index));

        let list_widget = List::new(items)
            .block(list_block)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(35, 45, 60))
                    .add_modifier(Modifier::BOLD),
            );
        f.render_stateful_widget(list_widget, area, &mut state);
    }
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let scroll_hint = if app.detail_scroll > 0 {
        format!(" 详情预览 (📜 滚动 +{}) ", app.detail_scroll)
    } else {
        " 详情预览 ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(Span::styled(
            scroll_hint,
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ));

    if let Some(memo) = app.selected_memo() {
        let created_str = memo.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        let updated_str = memo.updated_at.format("%Y-%m-%d %H:%M:%S").to_string();

        let mut lines = vec![
            Line::from(vec![
                Span::styled("📌 标题: ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(&memo.title, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("🕒 创建: ", Style::default().fg(Color::DarkGray)),
                Span::styled(created_str, Style::default().fg(Color::Gray)),
                Span::raw("   "),
                Span::styled("🔄 更新: ", Style::default().fg(Color::DarkGray)),
                Span::styled(updated_str, Style::default().fg(Color::Gray)),
            ]),
        ];

        if memo.archived {
            lines.push(Line::from(vec![
                Span::styled("📦 状态: ", Style::default().fg(Color::DarkGray)),
                Span::styled("已归档", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]));
        }

        lines.push(Line::from(Span::styled(
            "─".repeat(area.width.saturating_sub(4) as usize),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        for line_str in memo.content.lines() {
            lines.push(Line::from(Span::raw(line_str)));
        }

        if memo.content.is_empty() {
            lines.push(Line::from(Span::styled(
                "(正文内容为空)",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let p = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll, 0));
        f.render_widget(p, area);
    } else {
        let p = Paragraph::new("请选择左侧备忘录查看详情")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, area);
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    if let Some(status) = app.get_active_status() {
        let status_line = Line::from(vec![
            Span::styled(" 📢 ", Style::default().fg(Color::Yellow)),
            Span::styled(status, Style::default().fg(Color::LightYellow)),
        ]);
        f.render_widget(Paragraph::new(status_line), area);
        return;
    }

    if app.mode == AppMode::Searching {
        let prefix = " 🔍 搜索: ";
        let prefix_w = UnicodeWidthStr::width(prefix) as u16;
        let query_w = UnicodeWidthStr::width(app.search_query.as_str()) as u16;
        let cursor_x = (area.x + prefix_w + query_w).min(area.right().saturating_sub(1));
        let cursor_y = area.y;
        f.set_cursor_position((cursor_x, cursor_y));

        let search_line = Line::from(vec![
            Span::styled(prefix, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(&app.search_query, Style::default().fg(Color::Yellow)),
            Span::styled(" (按 Enter 锁定，Esc 取消)", Style::default().fg(Color::DarkGray)),
        ]);
        f.render_widget(Paragraph::new(search_line), area);
        return;
    }

    let archive_action_text = if app.list_filter == crate::tui::app::ListFilter::Archived {
        "取消归档 "
    } else {
        "归档 "
    };

    let help_line = Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("切换备忘/归档 "),
        Span::styled("[g]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(archive_action_text),
        Span::styled("[a]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("新建 "),
        Span::styled("[e]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("编辑 "),
        Span::styled("[d]", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw("删除 "),
        Span::styled("[/]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("搜索 "),
        Span::styled("[滚轮/Space/u]", Style::default().fg(Color::Yellow)),
        Span::raw("翻页 "),
        Span::styled("[q/Esc]", Style::default().fg(Color::DarkGray)),
        Span::raw("退出"),
    ]);

    f.render_widget(Paragraph::new(help_line), area);
}

fn draw_editor_page(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 顶部标题栏
            Constraint::Length(3), // 标题输入框
            Constraint::Min(6),    // 内容多行输入框 (占据全屏主要高度)
            Constraint::Length(1), // 底部提示
        ])
        .split(area);

    // 1. 顶部 Header
    let modal_title = if app.edit_target_id.is_some() {
        " ✏️ 编辑备忘录 "
    } else {
        " ➕ 新建备忘录 "
    };
    let header_line = Line::from(vec![
        Span::styled(
            " 📝 MEMOS ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" 终端备忘录编辑器 ─ "),
        Span::styled(
            modal_title,
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(header_line), chunks[0]);

    // 2. 标题输入框 (chunks[1])
    let title_border_color = if app.edit_focus == EditFocus::Title {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(title_border_color))
        .title(Span::styled(
            " 标题 (必填) ",
            Style::default().fg(title_border_color).add_modifier(Modifier::BOLD),
        ));
    let title_inner = title_block.inner(chunks[1]);
    f.render_widget(title_block, chunks[1]);
    let p_title = Paragraph::new(app.edit_title.as_str());
    f.render_widget(p_title, title_inner);

    // 3. 内容多行输入框 (chunks[2])
    let content_border_color = if app.edit_focus == EditFocus::Content {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let temp_block = Block::default().borders(Borders::ALL);
    let content_inner = temp_block.inner(chunks[2]);
    let content_w = content_inner.width.saturating_sub(1);
    let layout = app.compute_content_layout(content_w);
    let visible_content_h = content_inner.height as usize;

    let max_scroll = if layout.total_visual_lines >= visible_content_h && visible_content_h > 0 {
        layout.total_visual_lines - visible_content_h + 1
    } else {
        0
    };
    let final_scroll = (app.edit_scroll as usize).min(max_scroll);

    let content_title = if final_scroll > 0 {
        format!(" 正文内容 (支持多行 / 📜 滚动 +{}) ", final_scroll)
    } else {
        " 正文内容 (支持多行) ".to_string()
    };

    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(content_border_color))
        .title(Span::styled(
            content_title,
            Style::default().fg(content_border_color).add_modifier(Modifier::BOLD),
        ));

    f.render_widget(content_block, chunks[2]);

    let content_text = layout.visual_lines.join("\n");
    let p_content = Paragraph::new(content_text)
        .scroll((final_scroll as u16, 0));
    f.render_widget(p_content, content_inner);

    // 4. 硬件光标精确定位（IME / 中文输入法与任意位置编辑）
    match app.edit_focus {
        EditFocus::Title => {
            let before_cursor: String = app.edit_title.chars().take(app.title_cursor).collect();
            let cursor_w = UnicodeWidthStr::width(before_cursor.as_str()) as u16;
            let cursor_x = (title_inner.x + cursor_w).min(title_inner.right().saturating_sub(1));
            let cursor_y = title_inner.y;
            f.set_cursor_position((cursor_x, cursor_y));
        }
        EditFocus::Content => {
            if layout.visual_cursor_row >= final_scroll && layout.visual_cursor_row < final_scroll + visible_content_h {
                let cursor_y = content_inner.y + (layout.visual_cursor_row - final_scroll) as u16;
                let cursor_x = (content_inner.x + layout.visual_cursor_col as u16).min(content_inner.right().saturating_sub(1));
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    // 5. 底部操作提示 (chunks[3])
    let tip = Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" 切换标题/内容  "),
        Span::styled("[滚轮/PageUp/Down]", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" 滚动长文  "),
        Span::styled("[Ctrl+S]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" 保存  "),
        Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
        Span::raw(" 取消"),
    ]);
    f.render_widget(Paragraph::new(tip).alignment(Alignment::Center), chunks[3]);
}

fn draw_delete_confirm_modal(f: &mut Frame, app: &App) {
    let area = centered_rect(50, 25, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red))
        .title(Span::styled(
            " ⚠️ 删除确认 ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));

    let title_preview = app
        .selected_memo()
        .map(|m| m.title.as_str())
        .unwrap_or("该备忘录");

    let text = Text::from(vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("确定要删除备忘录「"),
            Span::styled(title_preview, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("」吗？"),
        ]),
        Line::from(Span::styled("此操作不可恢复！", Style::default().fg(Color::Red))),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y / Enter] 确认删除", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("    "),
            Span::styled("[n / Esc] 取消", Style::default().fg(Color::Gray)),
        ]),
    ]);

    let p = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);
    f.render_widget(p, area);
}

/// 计算居中弹窗 Rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
