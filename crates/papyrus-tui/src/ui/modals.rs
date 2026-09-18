use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, GenericPicker, TocImportSourceType};

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

    let path_content = if app.add_paper_path_buffer.is_empty() {
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
    let path_p = Paragraph::new(path_content)
        .block(path_block)
        .style(path_style);
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

    let name_content = if app.collection_name_buffer.is_empty() {
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
    let name_p = Paragraph::new(name_content)
        .block(name_block)
        .style(name_style);
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

/// Renders the full keyboard shortcuts and help dialog.
pub fn render_help_modal(_app: &App, frame: &mut Frame) {
    let frame_area = frame.area();
    let modal_height = 25.min(frame_area.height);
    let modal_width = 86.max(frame_area.width / 2).min(frame_area.width);

    let x = frame_area.width.saturating_sub(modal_width) / 2;
    let y = frame_area.height.saturating_sub(modal_height) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    frame.render_widget(Clear, modal_area);

    let modal_block = Block::default()
        .title(" Papyrus Keyboard Shortcuts & Help (?) ")
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let inner_area = modal_block.inner(modal_area);
    frame.render_widget(modal_block, modal_area);

    let help_lines = vec![
        Line::from(vec![
            Span::styled(
                "General: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Tab/Shift+Tab / h/l panel | Ctrl-w toggle layout | / search | ? help | q quit"),
        ]),
        Line::from(vec![
            Span::styled(
                "Motions: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("j/k move | gg/G first/last | Ctrl-d/u half page | [count] prefix (e.g. 5j, 12G)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Collections Panel: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(
                "j/k move | Enter focus papers | a new root | A subcollection | r rename | E export | d delete",
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Papers Panel: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("j/k move | Enter/o open | S sort | c col | t tags | d rem col | D del DB | V visual"),
        ]),
        Line::from(vec![
            Span::raw("              "),
            Span::raw("a add PDF/folder | e edit metadata | m import JSON | T edit tags modal"),
        ]),
        Line::from(vec![
            Span::styled(
                "Fuzzy & Multi: ",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Ctrl-p Quick Open | c Collection picker | t Tag picker | V Visual (d rem / D del)"),
        ]),
        Line::from(vec![
            Span::styled(
                "Path Modals:  ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Tab autocomplete path | Enter confirm | Esc cancel"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Table of Contents / Details: ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("j/k navigate | Enter jump to page | t fullscreen TOC"),
        ]),
        Line::from(vec![
            Span::raw("                             "),
            Span::raw("a/A add sibling/child | e edit | d delete | H/L indent"),
        ]),
        Line::from(vec![
            Span::raw("                             "),
            Span::raw("K/J reorder | E embed into PDF | i import from PDF/JSON"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Search Mode: ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Type query (matches title or tags, #tag for tag-only)"),
        ]),
        Line::from(vec![
            Span::raw("             "),
            Span::raw("Tab cycle collection | Enter keep filter | Esc cancel"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Fullscreen TOC: ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("j/k scroll | Enter open at page | Esc/q/t exit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " Press Esc, q, or ? to close this help dialog ",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    let p = Paragraph::new(help_lines).wrap(Wrap { trim: false });
    frame.render_widget(p, inner_area);
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
        let start_idx = if picker.selected >= visible_height {
            picker.selected - visible_height + 1
        } else {
            0
        };
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
