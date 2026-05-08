use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Mode, OutputKind};
use crate::highlight;

/// Phosphor green palette
const GREEN: Color = Color::Rgb(0, 255, 200);
const DIM_GREEN: Color = Color::Rgb(0, 128, 100);
const BG: Color = Color::Rgb(5, 10, 5);
const BORDER_GREEN: Color = Color::Rgb(0, 200, 156);
const ERROR_RED: Color = Color::Rgb(255, 60, 60);
const WARN_YELLOW: Color = Color::Rgb(255, 200, 60);
const OUTPUT_GREEN: Color = Color::Rgb(100, 255, 210);
const INFO_DIM: Color = Color::Rgb(100, 160, 140);
const LINE_NUM: Color = Color::Rgb(60, 120, 60);
const _CURSOR_BG: Color = Color::Rgb(0, 100, 80);

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Fill background
    let bg_block = Block::default().style(Style::default().bg(BG));
    f.render_widget(bg_block, size);

    // Layout: editor (60%), output (37%), status bar (3% / 1 line)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(37),
            Constraint::Min(1),
        ])
        .split(size);

    draw_editor(f, app, chunks[0]);
    draw_output(f, app, chunks[1]);
    draw_status(f, app, chunks[2]);

    // Overlays
    match app.mode {
        Mode::SavePrompt => draw_save_prompt(f, app, size),
        Mode::LoadBrowser | Mode::HistoryBrowser => draw_browser(f, app, size),
        _ => {}
    }
}

fn draw_editor(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " rust-pad  [editor] ",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_GREEN))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        return;
    }

    let visible_height = inner.height as usize;
    app.editor.ensure_visible(visible_height);

    let line_num_width = format!("{}", app.editor.lines.len()).len().max(3);
    let _code_width = inner.width as usize - line_num_width - 2; // 2 for "| " separator

    let start = app.editor.scroll_offset;
    let end = (start + visible_height).min(app.editor.lines.len());

    for (i, line_idx) in (start..end).enumerate() {
        let line = &app.editor.lines[line_idx];

        // Line number
        let num_str = format!("{:>width$} ", line_idx + 1, width = line_num_width);
        let num_span = Span::styled(num_str, Style::default().fg(LINE_NUM));

        // Highlighted code
        let highlighted = highlight::highlight_line(line);

        let mut spans = vec![num_span];
        spans.extend(highlighted.spans);

        let display_line = Line::from(spans);
        let y = inner.y + i as u16;

        // Render line
        let line_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };

        // If this is the cursor line, give it a subtle highlight
        if line_idx == app.editor.cursor_row {
            let highlight_block = Block::default().style(
                Style::default().bg(Color::Rgb(10, 30, 10)),
            );
            f.render_widget(highlight_block, line_area);
        }

        let para = Paragraph::new(display_line);
        f.render_widget(para, line_area);
    }

    // Set cursor position
    if app.mode == Mode::Editing {
        let cursor_screen_row = app.editor.cursor_row.saturating_sub(app.editor.scroll_offset);
        let cursor_x =
            inner.x + line_num_width as u16 + 1 + app.editor.cursor_col as u16;
        let cursor_y = inner.y + cursor_screen_row as u16;

        if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

fn draw_output(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " [output] ",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_GREEN))
        .style(Style::default().bg(BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line> = app
        .output_lines
        .iter()
        .map(|ol| {
            let color = match ol.kind {
                OutputKind::Normal => DIM_GREEN,
                OutputKind::Error => ERROR_RED,
                OutputKind::Warning => WARN_YELLOW,
                OutputKind::Success => OUTPUT_GREEN,
                OutputKind::Info => INFO_DIM,
            };
            Line::from(Span::styled(ol.text.clone(), Style::default().fg(color)))
        })
        .collect();

    // Auto-scroll to bottom
    let total = lines.len();
    let visible = inner.height as usize;
    let scroll = if total > visible { total - visible } else { 0 };

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    f.render_widget(para, inner);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let keyhints = " F5:Run  F6:Check  F2:Save  F3:Load  F4:History  Tab:Template  ^Q:Quit ";

    let status_text = if app.status_msg.is_empty() {
        keyhints.to_string()
    } else {
        format!(" {} ", app.status_msg)
    };

    let line_info = format!(
        " Ln {}, Col {} ",
        app.editor.cursor_row + 1,
        app.editor.cursor_col + 1
    );

    let left = Span::styled(
        status_text,
        Style::default()
            .fg(Color::Rgb(0, 0, 0))
            .bg(GREEN)
            .add_modifier(Modifier::BOLD),
    );

    let right = Span::styled(
        line_info,
        Style::default()
            .fg(Color::Rgb(0, 0, 0))
            .bg(GREEN)
            .add_modifier(Modifier::BOLD),
    );

    // Fill the status bar
    let left_len = left.content.len();
    let right_len = right.content.len();
    let total_width = area.width as usize;
    let padding = if total_width > left_len + right_len {
        total_width - left_len - right_len
    } else {
        0
    };

    let pad_span = Span::styled(
        " ".repeat(padding),
        Style::default().bg(GREEN),
    );

    let bar = Line::from(vec![left, pad_span, right]);
    let para = Paragraph::new(bar);
    f.render_widget(para, area);
}

fn draw_save_prompt(f: &mut Frame, _app: &App, area: Rect) {
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = 5u16;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(
            " Save Snippet ",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_GREEN))
        .style(Style::default().bg(BG));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let prompt = format!("Name: {}_", _app.save_input);
    let para = Paragraph::new(Line::from(Span::styled(
        prompt,
        Style::default().fg(GREEN),
    )));
    f.render_widget(para, inner);
}

fn draw_browser(f: &mut Frame, app: &App, area: Rect) {
    let title = match app.mode {
        Mode::LoadBrowser => " Load Snippet ",
        Mode::HistoryBrowser => " History ",
        _ => "",
    };

    let width = 60u16.min(area.width.saturating_sub(4));
    let height = 20u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_GREEN))
        .style(Style::default().bg(BG));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let items: Vec<ListItem> = app
        .browser_items
        .iter()
        .enumerate()
        .map(|(i, (name, _path))| {
            let style = if i == app.browser_index {
                Style::default()
                    .fg(Color::Rgb(0, 0, 0))
                    .bg(GREEN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DIM_GREEN)
            };
            ListItem::new(Line::from(Span::styled(
                format!("  {name}"),
                style,
            )))
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, inner);
}
