use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{App, TocImportSourceType};

/// Renders the TOC entry add/edit modal dialog.
pub fn render_toc_modal(app: &App, frame: &mut Frame) {
    let Some(ref state) = app.toc_edit_state else {
        return;
    };

    let frame_area = frame.area();
    let modal_width = 56.min(frame_area.width.saturating_sub(4));
    let modal_height = 10.min(frame_area.height.saturating_sub(2));

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let title_text = if state.entry_id.is_some() {
        " Edit Table of Contents Entry "
    } else {
        " Add Table of Contents Entry "
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
            Constraint::Length(3), // Title
            Constraint::Length(3), // Page
            Constraint::Length(1), // Help
        ])
        .split(inner_area);

    // 1. Title field
    let title_focused = state.active_field == 0;
    let title_border = if title_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title_block = Block::default()
        .title(" Title ")
        .borders(Borders::ALL)
        .border_style(title_border);

    let cursor_title = if title_focused { "█" } else { "" };
    let title_content = format!("{}{cursor_title}", state.title_buffer);
    let title_p = Paragraph::new(title_content)
        .block(title_block)
        .style(if title_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        });
    frame.render_widget(title_p, chunks[0]);

    // 2. Page field
    let page_focused = state.active_field == 1;
    let page_border = if page_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let page_block = Block::default()
        .title(" Page Number ")
        .borders(Borders::ALL)
        .border_style(page_border);

    let cursor_page = if page_focused { "█" } else { "" };
    let page_content = format!("{}{cursor_page}", state.page_buffer);
    let page_p = Paragraph::new(page_content)
        .block(page_block)
        .style(if page_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        });
    frame.render_widget(page_p, chunks[1]);

    // 3. Help bar
    let help_text = " Tab: Next field | Enter: Save | Esc: Cancel ";
    let help_p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help_p, chunks[2]);
}

/// Renders the TOC import modal dialog.
pub fn render_toc_import_modal(app: &App, frame: &mut Frame) {
    let Some(ref state) = app.toc_import_state else {
        return;
    };

    let frame_area = frame.area();
    let modal_width = 66.min(frame_area.width.saturating_sub(4));
    let modal_height = 14.min(frame_area.height.saturating_sub(2));

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let modal_block = Block::default()
        .title(" Import Table of Contents ")
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
            Constraint::Length(3), // Source selector
            Constraint::Length(3), // File path (if text/json)
            Constraint::Length(2), // Merge mode checkbox
            Constraint::Min(1),    // Error message / status
            Constraint::Length(1), // Help bar
        ])
        .split(inner_area);

    // 1. Source selector
    let src_focused = state.active_field == 0;
    let src_border = if src_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let src_block = Block::default()
        .title(" Source (Press Space or Left/Right to switch) ")
        .borders(Borders::ALL)
        .border_style(src_border);

    let src_text = format!("  < {} >", state.source_type.name());
    let src_p = Paragraph::new(src_text)
        .block(src_block)
        .style(if src_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        });
    frame.render_widget(src_p, chunks[0]);

    // 2. File path (enabled for TextFile / JsonFile)
    let path_focused = state.active_field == 1;
    let path_border = if path_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let path_block = Block::default()
        .title(" File Path ")
        .borders(Borders::ALL)
        .border_style(path_border);

    let path_content = if state.source_type == TocImportSourceType::PdfOutline {
        "  (Read directly from PDF outline)".to_string()
    } else {
        let cursor = if path_focused { "█" } else { "" };
        format!("  {}{cursor}", state.file_path_buffer)
    };
    let path_p = Paragraph::new(path_content)
        .block(path_block)
        .style(if path_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        });
    frame.render_widget(path_p, chunks[1]);

    // 3. Merge mode toggle
    let merge_focused = state.active_field == 2;
    let merge_check = if state.merge_mode { "[X]" } else { "[ ]" };
    let merge_style = if merge_focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let merge_text = format!("  {merge_check} Merge with existing TOC (otherwise replace)");
    let merge_p = Paragraph::new(merge_text).style(merge_style);
    frame.render_widget(merge_p, chunks[2]);

    // 4. Error message if present
    if let Some(ref err) = state.error_message {
        let err_p = Paragraph::new(format!(" Error: {err}"))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
        frame.render_widget(err_p, chunks[3]);
    }

    // 5. Help bar
    let help_text = " Tab: Next field | Space: Toggle | Enter: Import | Esc: Cancel ";
    let help_p = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(help_p, chunks[4]);
}
