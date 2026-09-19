use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, GenericPicker};

/// Renders the centered generic picker modal over the screen.
pub fn render_generic_picker(_app: &App, frame: &mut Frame, picker: &GenericPicker) {
    let frame_area = frame.area();
    let modal_width = ((frame_area.width * 70) / 100)
        .max(50)
        .min(frame_area.width);
    let modal_height = ((frame_area.height * 70) / 100)
        .max(16)
        .min(frame_area.height);

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let title = format!(
        " {} · {}/{} ",
        picker.title,
        picker.visible_indices.len(),
        picker.items.len()
    );
    let modal_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let inner_area = modal_block.inner(modal_area);
    frame.render_widget(modal_block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Query input
            Constraint::Min(1),    // List of items
            Constraint::Length(1), // Help text
        ])
        .split(inner_area);

    let query_block = Block::default()
        .title(" Search Query ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let query_text = format!(" > {}█", picker.query);
    let query_p = Paragraph::new(query_text).block(query_block).style(
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(query_p, chunks[0]);

    let visible_height = chunks[1].height as usize;
    let total_visible = picker.visible_indices.len();
    if visible_height > 0 && total_visible > 0 {
        let start_idx =
            crate::ui::centered_scroll_offset(picker.selected, total_visible, visible_height);
        let end_idx = (start_idx + visible_height).min(total_visible);

        let mut lines = Vec::new();
        for i in start_idx..end_idx {
            let item_idx = picker.visible_indices[i];
            let item = &picker.items[item_idx];
            let is_cursor = i == picker.selected;
            let is_checked = picker.checked.contains(&item.id);

            let cursor_prefix = if is_cursor { "> " } else { "  " };

            let mut spans = Vec::new();
            spans.push(Span::styled(
                cursor_prefix,
                if is_cursor {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            ));

            if picker.multi_select {
                let check_str = if is_checked { "[x] " } else { "[ ] " };
                spans.push(Span::styled(
                    check_str,
                    if is_checked {
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    },
                ));
            }

            spans.push(Span::styled(
                &item.title,
                if is_cursor {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ));

            if let Some(ref sub) = item.subtitle {
                spans.push(Span::styled(
                    format!(" — {sub}"),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            if let Some(ref cat) = item.category {
                spans.push(Span::styled(
                    format!(" [{cat}]"),
                    Style::default().fg(Color::Cyan),
                ));
            }

            let line_style = if is_cursor {
                Style::default().bg(Color::Rgb(25, 45, 70))
            } else {
                Style::default()
            };

            lines.push(Line::from(spans).style(line_style));
        }

        let list_p = Paragraph::new(lines);
        frame.render_widget(list_p, chunks[1]);
    } else if total_visible == 0 {
        let empty_p = Paragraph::new(" No matching items ")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(empty_p, chunks[1]);
    }

    let help_text = if picker.multi_select {
        " Space: Toggle | j/k: Navigate | Enter: Confirm | Esc: Cancel "
    } else {
        " j/k: Navigate | Enter: Open | Esc: Cancel "
    };
    let help_p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help_p, chunks[2]);
}
