use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

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
            Span::raw("Tab/Shift+Tab / h/l panel | W/z / Ctrl-w zoom | / search | ? help | q/ZZ/ZQ quit"),
        ]),
        Line::from(vec![
            Span::styled(
                "Motions: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("j/k move | gg/G first/last | }/{ (J/K) / Ctrl-d/u half page | [count] prefix"),
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
            Span::raw("p / Ctrl-p Quick Open | c Collection picker | t Tag picker | V Visual (d rem / D del)"),
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
