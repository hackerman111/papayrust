use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

/// Renders the centered metadata editing modal dialog over the active screen.
pub fn render_metadata_modal(app: &App, frame: &mut Frame) {
    let frame_area = frame.area();
    let modal_height = ((frame_area.height * 75) / 100)
        .max(18)
        .min(frame_area.height);
    let modal_width = ((frame_area.width * 70) / 100)
        .max(50)
        .min(frame_area.width);

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    // Clear background behind floating modal
    frame.render_widget(Clear, modal_area);

    let modal_block = Block::default()
        .title(" Edit Paper Metadata ")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let inner_area = modal_block.inner(modal_area);
    frame.render_widget(modal_block, modal_area);

    let field_labels = [
        "Title (required)",
        "Authors",
        "Year",
        "Journal",
        "DOI",
        "Abstract",
    ];

    if inner_area.height >= 19 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Authors
                Constraint::Length(3), // Year
                Constraint::Length(3), // Journal
                Constraint::Length(3), // DOI
                Constraint::Min(3),    // Abstract
                Constraint::Length(1), // Help text
            ])
            .split(inner_area);

        for i in 0..6 {
            let is_focused = app.editing_field_index == i;
            let border_style = if is_focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let field_block = Block::default()
                .title(format!(" {} ", field_labels[i]))
                .borders(Borders::ALL)
                .border_style(border_style);

            let cursor = if is_focused { "█" } else { "" };
            let content = format!("{}{cursor}", app.edit_buffers[i]);

            let paragraph = Paragraph::new(content)
                .block(field_block)
                .wrap(Wrap { trim: false })
                .style(if is_focused {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                });

            frame.render_widget(paragraph, chunks[i]);
        }

        let help_text = " Tab: Next field | Enter / Ctrl+S: Save | Esc: Cancel ";
        let help_bar = Paragraph::new(help_text)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(help_bar, chunks[6]);
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Length(2), // Authors
                Constraint::Length(2), // Year
                Constraint::Length(2), // Journal
                Constraint::Length(2), // DOI
                Constraint::Min(2),    // Abstract
                Constraint::Length(1), // Help text
            ])
            .split(inner_area);

        for i in 0..6 {
            let is_focused = app.editing_field_index == i;
            let cursor = if is_focused { "█" } else { "" };
            let label_style = if is_focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let text_style = if is_focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            let line = Line::from(vec![
                Span::styled(format!("{}: ", field_labels[i]), label_style),
                Span::styled(format!("{}{cursor}", app.edit_buffers[i]), text_style),
            ]);
            let paragraph = Paragraph::new(vec![line]).wrap(Wrap { trim: false });
            frame.render_widget(paragraph, chunks[i]);
        }

        let help_text = " Tab: Next field | Enter / Ctrl+S: Save | Esc: Cancel ";
        let help_bar = Paragraph::new(help_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(help_bar, chunks[6]);
    }
}

/// Renders the modal dialog for importing metadata from a JSON file.
pub fn render_import_metadata_modal(app: &App, frame: &mut Frame) {
    let frame_area = frame.area();
    let modal_height = 9.min(frame_area.height);
    let modal_width = ((frame_area.width * 70) / 100)
        .max(50)
        .min(frame_area.width);

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let modal_block = Block::default()
        .title(" Import Metadata from JSON ")
        .borders(Borders::ALL)
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
            Constraint::Length(3), // Path input
            Constraint::Length(1), // Status
            Constraint::Length(1), // Help
        ])
        .split(inner_area);

    let path_block = Block::default()
        .title(" Path to JSON file or folder (Tab: autocomplete) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let content = if app.import_metadata_buffer.is_empty() {
        "  /path/to/metadata.json or /path/to/folder".to_string()
    } else {
        format!("  {}█", app.import_metadata_buffer)
    };
    let style = if app.import_metadata_buffer.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    };
    let path_p = Paragraph::new(content).block(path_block).style(style);
    frame.render_widget(path_p, chunks[0]);

    if let Some(ref msg) = app.status_message {
        let msg_p = Paragraph::new(format!(" {msg}")).style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg_p, chunks[1]);
    }

    let help_text = " Enter: Import | Tab: Autocomplete path | Esc: Cancel ";
    let help_p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help_p, chunks[2]);
}
