use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

/// Renders the Add Paper modal dialog for importing a new PDF file.
pub fn render_add_paper_modal(app: &App, frame: &mut Frame) {
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
        .title(" Add Paper to Library (or Folder) ")
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
            Constraint::Length(1), // Target collection label
            Constraint::Length(3), // Path input field
            Constraint::Length(1), // Status or spacer
            Constraint::Length(1), // Help / shortcuts
        ])
        .split(inner_area);

    // 1. Target collection label
    let target_coll = if !app.collections.is_empty() {
        &app.collections[app.selected_collection].name
    } else {
        "All Papers"
    };
    let coll_text = format!(" Target Collection: {target_coll}");
    let coll_p = Paragraph::new(coll_text).style(Style::default().fg(Color::LightCyan));
    frame.render_widget(coll_p, chunks[0]);

    // 2. Path input field
    let path_block = Block::default()
        .title(" Path to PDF file (or Folder) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let content = if app.add_paper_path_buffer.is_empty() {
        "  /path/to/paper.pdf or /path/to/folder (type path)".to_string()
    } else {
        format!("  {}█", app.add_paper_path_buffer)
    };
    let path_style = if app.add_paper_path_buffer.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    };
    let path_p = Paragraph::new(content).block(path_block).style(path_style);
    frame.render_widget(path_p, chunks[1]);

    // 3. Status message if any
    if let Some(ref msg) = app.status_message {
        let msg_p = Paragraph::new(format!(" {msg}")).style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg_p, chunks[2]);
    }

    // 4. Help text
    let help_text = " Enter: Import PDF or Folder | Tab: Autocomplete | Esc: Cancel ";
    let help_p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help_p, chunks[3]);
}

/// Renders the modal dialog for editing tags for the currently selected paper.
pub fn render_edit_tags_modal(app: &App, frame: &mut Frame) {
    let frame_area = frame.area();
    let modal_height = 9.min(frame_area.height);
    let modal_width = 64.max(frame_area.width / 2).min(frame_area.width);

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let modal_block = Block::default()
        .title(" Edit Paper Tags ")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        );

    let inner_area = modal_block.inner(modal_area);
    frame.render_widget(modal_block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Paper title info
            Constraint::Length(3), // Tags input
            Constraint::Length(1), // Spacer or status
            Constraint::Length(1), // Help / shortcuts
        ])
        .split(inner_area);

    let paper_title = app
        .current_paper()
        .and_then(|p| p.title.as_deref())
        .unwrap_or("Untitled Paper");
    let title_line = Paragraph::new(format!(" Paper: {paper_title}"))
        .style(Style::default().fg(Color::LightCyan));
    frame.render_widget(title_line, chunks[0]);

    let input_block = Block::default()
        .title(" Tags (comma-separated, e.g. ml, attention, arxiv) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let content = if app.tags_input_buffer.is_empty() {
        "  e.g. ai, math, nlp".to_string()
    } else {
        format!("  {}█", app.tags_input_buffer)
    };
    let content_style = if app.tags_input_buffer.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    };
    let input_p = Paragraph::new(content)
        .block(input_block)
        .style(content_style);
    frame.render_widget(input_p, chunks[1]);

    if let Some(ref msg) = app.status_message {
        let msg_p = Paragraph::new(format!(" {msg}")).style(Style::default().fg(Color::Yellow));
        frame.render_widget(msg_p, chunks[2]);
    }

    let help_text = " Enter: Save Tags | Esc: Cancel ";
    let help_p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help_p, chunks[3]);
}

/// Renders the confirmation modal dialog for deleting collections or papers.
pub fn render_delete_confirm_modal(app: &App, frame: &mut Frame) {
    let frame_area = frame.area();
    let modal_height = 8.min(frame_area.height);
    let modal_width = 58.max(frame_area.width / 2).min(frame_area.width);

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let modal_block = Block::default()
        .title(" Confirm Deletion ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

    let inner_area = modal_block.inner(modal_area);
    frame.render_widget(modal_block, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Warning line
            Constraint::Length(2), // Item name
            Constraint::Length(1), // Actions
        ])
        .split(inner_area);

    let warning_p = Paragraph::new(" Are you sure you want to delete:")
        .style(Style::default().fg(Color::LightRed));
    frame.render_widget(warning_p, chunks[0]);

    let desc = &app.delete_target_description;
    let item_p = Paragraph::new(format!("   {desc}"))
        .style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(item_p, chunks[1]);

    let help_text = " Enter: Delete Permanently | Esc / q: Cancel ";
    let help_p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help_p, chunks[2]);
}
