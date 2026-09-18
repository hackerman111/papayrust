use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Wrap};
use ratatui::Frame;
use std::collections::HashMap;

use crate::app::{ActivePanel, App};

/// Renders the collections list in the left panel.
pub fn render_collections(app: &App, frame: &mut Frame, area: Rect) {
    let border_style = if app.active_panel == ActivePanel::Collections {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let collections_block = Block::default()
        .title("Collections")
        .borders(Borders::ALL)
        .border_style(border_style);

    let collection_items: Vec<ListItem> = app
        .collections
        .iter()
        .enumerate()
        .map(|(idx, col)| {
            let is_selected = idx == app.selected_collection;
            let prefix = if is_selected { "> " } else { "  " };
            let tree_indent = if col.depth > 0 {
                format!("{}└─ ", "  ".repeat(col.depth - 1))
            } else {
                String::new()
            };
            let content = format!("{prefix}{tree_indent}{} ({})", col.name, col.paper_count);

            let mut style = Style::default();
            if is_selected {
                if app.active_panel == ActivePanel::Collections {
                    style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                } else {
                    style = style.fg(Color::White).add_modifier(Modifier::UNDERLINED);
                }
            }
            ListItem::new(content).style(style)
        })
        .collect();

    let collections_list = List::new(collection_items).block(collections_block);
    frame.render_widget(collections_list, area);
}

/// Renders the papers table in the middle panel.
pub fn render_papers(app: &App, frame: &mut Frame, area: Rect) {
    let border_style = if app.active_panel == ActivePanel::Papers {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let papers_block = Block::default()
        .title("Papers")
        .borders(Borders::ALL)
        .border_style(border_style);

    let header = Row::new(vec!["Title", "Authors", "Year"])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

    let rows: Vec<Row> = app
        .papers
        .iter()
        .enumerate()
        .map(|(idx, paper)| {
            let is_selected = idx == app.selected_paper;
            let prefix = if is_selected { "> " } else { "  " };
            let tags_badge = app
                .tags_by_paper
                .get(&paper.id)
                .filter(|t| !t.is_empty())
                .map(|t| format!(" [{}]", t.join(", ")))
                .unwrap_or_default();
            let title = format!(
                "{}{}{}",
                prefix,
                paper.title.as_deref().unwrap_or("[Untitled]"),
                tags_badge
            );
            let authors = paper.authors.as_deref().unwrap_or("-").to_string();
            let year = paper
                .year
                .map(|y| y.to_string())
                .unwrap_or_else(|| "-".to_string());

            let mut style = Style::default();
            if is_selected {
                if app.active_panel == ActivePanel::Papers {
                    style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                } else {
                    style = style.fg(Color::White).add_modifier(Modifier::UNDERLINED);
                }
            }
            Row::new(vec![title, authors, year]).style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(55),
        Constraint::Percentage(35),
        Constraint::Percentage(10),
    ];

    let papers_table = Table::new(rows, widths).header(header).block(papers_block);
    frame.render_widget(papers_table, area);
}

/// Renders the paper details, metadata, and TOC tree in the right panel.
pub fn render_details(app: &App, frame: &mut Frame, area: Rect) {
    let border_style = if app.active_panel == ActivePanel::Details {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let details_block = Block::default()
        .title("Details / Metadata")
        .borders(Borders::ALL)
        .border_style(border_style);

    if let Some(paper) = app.current_paper() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    "Title: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::raw(paper.title.as_deref().unwrap_or("[Untitled]")),
            ]),
            Line::from(vec![
                Span::styled(
                    "Authors: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::raw(paper.authors.as_deref().unwrap_or("-")),
            ]),
            Line::from(vec![
                Span::styled(
                    "Year: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::raw(
                    paper
                        .year
                        .map(|y| y.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Journal: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::raw(paper.journal.as_deref().unwrap_or("-")),
            ]),
            Line::from(vec![
                Span::styled(
                    "DOI: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::raw(paper.doi.as_deref().unwrap_or("-")),
            ]),
            Line::from(vec![
                Span::styled(
                    "Content Hash: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::raw(&paper.content_hash),
            ]),
            Line::from(vec![
                Span::styled(
                    "File Path: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::raw(&paper.file_path),
            ]),
            Line::from(vec![
                Span::styled(
                    "Abstract: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::raw(paper.abstract_text.as_deref().unwrap_or("-")),
            ]),
            Line::from(vec![
                Span::styled(
                    "Tags: ",
                    Style::default()
                        .add_modifier(Modifier::BOLD)
                        .fg(Color::Cyan),
                ),
                Span::styled(
                    {
                        let tags = app.current_paper_tags();
                        if tags.is_empty() {
                            "-".to_string()
                        } else {
                            tags.join(", ")
                        }
                    },
                    Style::default().fg(Color::LightGreen),
                ),
            ]),
        ];

        // Notice if annotated copy is outdated
        if app.is_current_paper_annotated_outdated() {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                " [!] Annotated PDF Outdated - Press 'E' to re-embed TOC ",
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "Table of Contents:",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
        )]));

        if app.toc_preview.is_empty() {
            lines.push(Line::from("  (No TOC entries)"));
        } else {
            // Build parent-to-level mapping for proper tree indentation
            let mut id_to_level = HashMap::new();
            for entry in &app.toc_preview {
                let level = match entry.parent_id {
                    Some(pid) => id_to_level.get(&pid).copied().unwrap_or(0) + 1,
                    None => 0,
                };
                id_to_level.insert(entry.id, level);
            }

            for (idx, entry) in app.toc_preview.iter().enumerate() {
                let is_selected =
                    idx == app.selected_toc && app.active_panel == ActivePanel::Details;
                let prefix = if is_selected { "> " } else { "  " };
                let level = id_to_level.get(&entry.id).copied().unwrap_or(0);
                let indent = "  ".repeat(level);
                let page_info = format!(" (p. {})", entry.page_number);
                let src_badge = format!(" [{}]", entry.source.as_str());
                let toc_text = format!("{prefix}{indent}- {}{page_info}{src_badge}", entry.title);

                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                lines.push(Line::styled(toc_text, style));
            }
        }

        let visible_height = area.height.saturating_sub(2) as usize;
        let mut scroll_y = 0u16;
        if app.active_panel == ActivePanel::Details && !app.toc_preview.is_empty() {
            let metadata_lines_count = lines.len().saturating_sub(app.toc_preview.len());
            let current_line = metadata_lines_count + app.selected_toc;
            if current_line >= visible_height {
                scroll_y = (current_line - visible_height + 2) as u16;
            }
        }

        let paragraph = Paragraph::new(lines)
            .block(details_block)
            .scroll((scroll_y, 0))
            .wrap(Wrap { trim: false });
        frame.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("No paper selected").block(details_block);
        frame.render_widget(paragraph, area);
    }
}

/// Renders the dedicated fullscreen scrollable Table of Contents view.
pub fn render_fullscreen_toc(app: &App, frame: &mut Frame, area: Rect) {
    let paper_title = app
        .current_paper()
        .and_then(|p| p.title.as_deref())
        .unwrap_or("[Untitled Paper]");

    let total_tocs = app.toc_preview.len();
    let current_idx = if total_tocs > 0 {
        app.selected_toc + 1
    } else {
        0
    };

    let block = Block::default()
        .title(format!(
            " Table of Contents: {} [{}/{}] ",
            paper_title, current_idx, total_tocs
        ))
        .borders(Borders::ALL)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if app.toc_preview.is_empty() {
        let empty_p = Paragraph::new(
            "No table of contents entries for this paper. Press Esc, 'q' or 't' to exit.",
        )
        .style(Style::default().fg(Color::DarkGray))
        .alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(empty_p, inner_area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Entries list
            Constraint::Length(1), // Footer shortcuts
        ])
        .split(inner_area);

    let visible_height = chunks[0].height as usize;
    let scroll_offset = if app.selected_toc < app.toc_scroll_offset {
        app.selected_toc
    } else if app.selected_toc >= app.toc_scroll_offset + visible_height {
        app.selected_toc.saturating_sub(visible_height) + 1
    } else {
        app.toc_scroll_offset
    };

    let mut id_to_level = HashMap::new();
    for entry in &app.toc_preview {
        let level = match entry.parent_id {
            Some(pid) => id_to_level.get(&pid).copied().unwrap_or(0) + 1,
            None => 0,
        };
        id_to_level.insert(entry.id, level);
    }

    let mut lines = Vec::new();
    let end_idx = (scroll_offset + visible_height).min(app.toc_preview.len());

    for (idx, entry) in app.toc_preview[scroll_offset..end_idx].iter().enumerate() {
        let actual_idx = scroll_offset + idx;
        let is_selected = actual_idx == app.selected_toc;
        let prefix = if is_selected { "▶ " } else { "  " };
        let level = id_to_level.get(&entry.id).copied().unwrap_or(0);
        let indent = "    ".repeat(level);
        let page_info = format!(" (p. {})", entry.page_number);
        let src_badge = format!(" [{}]", entry.source.as_str());

        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let line_text = format!("{prefix}{indent}{} {}{}", entry.title, page_info, src_badge);
        lines.push(Line::styled(line_text, style));
    }

    let toc_p = Paragraph::new(lines);
    frame.render_widget(toc_p, chunks[0]);

    let footer_text =
        " Enter: Open at page | j/k: Scroll | Home/End: Jump | Esc/q/t: Exit Fullscreen TOC ";
    let footer_p = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Cyan))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(footer_p, chunks[1]);
}
