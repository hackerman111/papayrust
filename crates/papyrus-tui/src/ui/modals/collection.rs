use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Renders the centered modal dialog for creating a new collection (folder).
pub fn render_create_collection_modal(app: &App, frame: &mut Frame) {
    let frame_area = frame.area();
    let modal_height = 8.min(frame_area.height);
    let modal_width = 56.max(frame_area.width / 2).min(frame_area.width);

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let parent_name = app.create_collection_parent_id.and_then(|pid| {
        app.collections
            .iter()
            .find(|c| c.id == Some(pid))
            .map(|c| c.name.as_str())
    });
    let title_text = if let Some(pname) = parent_name {
        format!(" Create Subcollection inside '{pname}' ")
    } else {
        " Create New Collection ".to_string()
    };

    let modal_block = Block::default()
        .title(title_text)
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
            Constraint::Length(3), // Name input field
            Constraint::Length(1), // Status or spacer
            Constraint::Length(1), // Help / shortcuts
        ])
        .split(inner_area);

    let name_block = Block::default()
        .title(" Collection Name ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let content = if app.collection_name_buffer.is_empty() {
        "  e.g. Machine Learning, Neuroscience".to_string()
    } else {
        format!("  {}█", app.collection_name_buffer)
    };
    let name_style = if app.collection_name_buffer.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    };
    let name_p = Paragraph::new(content).block(name_block).style(name_style);
    frame.render_widget(name_p, chunks[0]);

    if let Some(ref msg) = app.status_message {
        let msg_p = Paragraph::new(format!(" {msg}")).style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg_p, chunks[1]);
    }

    let help_text = " Enter: Create Collection | Esc: Cancel ";
    let help_p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help_p, chunks[2]);
}

/// Renders the modal dialog for renaming a collection.
pub fn render_rename_collection_modal(app: &App, frame: &mut Frame) {
    let frame_area = frame.area();
    let modal_height = 8.min(frame_area.height);
    let modal_width = 56.max(frame_area.width / 2).min(frame_area.width);

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let col_name = app
        .current_collection()
        .map(|c| c.name.as_str())
        .unwrap_or("Collection");
    let title = format!(" Rename Collection '{}' ", col_name);

    let modal_block = Block::default()
        .title(title)
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
            Constraint::Length(3), // Name input
            Constraint::Length(1), // Status
            Constraint::Length(1), // Help
        ])
        .split(inner_area);

    let name_block = Block::default()
        .title(" New Name ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let content = if app.rename_collection_buffer.is_empty() {
        "  Type new collection name".to_string()
    } else {
        format!("  {}█", app.rename_collection_buffer)
    };
    let style = if app.rename_collection_buffer.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    };
    let name_p = Paragraph::new(content).block(name_block).style(style);
    frame.render_widget(name_p, chunks[0]);

    if let Some(ref msg) = app.status_message {
        let msg_p = Paragraph::new(format!(" {msg}")).style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg_p, chunks[1]);
    }

    let help_text = " Enter: Rename | Esc: Cancel ";
    let help_p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help_p, chunks[2]);
}

/// Renders the modal dialog for exporting a collection to a ZIP archive.
pub fn render_export_collection_modal(app: &App, frame: &mut Frame) {
    let frame_area = frame.area();
    let modal_height = 9.min(frame_area.height);
    let modal_width = ((frame_area.width * 70) / 100)
        .max(50)
        .min(frame_area.width);

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let col_name = app
        .current_collection()
        .map(|c| c.name.as_str())
        .unwrap_or("Collection");
    let title = format!(" Export Collection '{}' to ZIP ", col_name);

    let modal_block = Block::default()
        .title(title)
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
        .title(" Destination Path (Tab: autocomplete) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let content = if app.export_path_buffer.is_empty() {
        "  /path/to/export.zip (leave empty for default)".to_string()
    } else {
        format!("  {}█", app.export_path_buffer)
    };
    let style = if app.export_path_buffer.is_empty() {
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

    let help_text = " Enter: Export | Tab: Autocomplete path | Esc: Cancel ";
    let help_p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help_p, chunks[2]);
}
