use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Renders the search bar when search mode is active or query is non-empty.
pub fn render_search_bar(app: &App, frame: &mut Frame, area: Rect) {
    let border_style = if app.is_searching {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let col_name = if !app.collections.is_empty() {
        let idx = app
            .search_collection_index
            .min(app.collections.len().saturating_sub(1));
        &app.collections[idx].name
    } else {
        "All Papers"
    };

    let title = if app.is_searching {
        format!(" Search [/] in [{col_name}] (Tab to switch) ")
    } else {
        format!(" Filtered in [{col_name}] ")
    };

    let search_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let cursor = if app.is_searching { "█" } else { "" };
    let content = format!(" / {}{cursor}", app.search_query);

    let search_paragraph = Paragraph::new(content)
        .block(search_block)
        .style(if app.is_searching {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        });

    frame.render_widget(search_paragraph, area);
}
