use papyrus_core::db::{
    open_in_memory, Collection, CollectionRepo, Paper, PaperRepo, RepoError, TocEntry, TocRepo,
    TocSource,
};
use papyrus_core::Action;
use papyrus_tui::{map_key_event, render, ActivePanel, App, CollectionItem};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;
use uuid::Uuid;

fn sample_paper(name: &str, year: i64) -> Paper {
    Paper {
        id: Uuid::now_v7(),
        file_path: format!("/papers/{name}.pdf"),
        content_hash: format!("hash_{name}_{}", Uuid::now_v7()),
        title: Some(format!("Paper Title {name}")),
        authors: Some(format!("Author {name}")),
        year: Some(year),
        journal: Some("Test Journal".to_string()),
        doi: Some(format!("10.1000/{name}")),
        abstract_text: Some(format!("Abstract text for {name}")),
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

fn sample_toc(paper_id: Uuid, title: &str, page: u32, order: i32) -> TocEntry {
    TocEntry {
        id: Uuid::now_v7(),
        paper_id,
        parent_id: None,
        title: title.to_string(),
        page_number: page,
        order_index: order,
        source: TocSource::Auto,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// RT-14: Layout rendering test.
/// Renders the App on a TestBackend (120x30 cells).
/// Asserts that all 3 panels (Collections, Papers, Details/Metadata) and the status bar
/// are present in the terminal buffer.
#[test]
fn test_rt_14_layout_rendering() {
    let mut app = App::new();

    let p1 = sample_paper("Alpha", 2023);
    let p2 = sample_paper("Beta", 2024);
    let c1_id = Uuid::now_v7();

    app.add_collection(
        CollectionItem::new(Some(c1_id), "Machine Learning", 2),
        vec![p1.clone(), p2],
    );
    app.set_paper_tocs(
        p1.id,
        vec![
            sample_toc(p1.id, "1. Introduction", 1, 0),
            sample_toc(p1.id, "2. Background", 5, 1),
        ],
    );

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).expect("create TestBackend terminal");
    terminal
        .draw(|f| render(&app, f))
        .expect("render app frame");

    let rendered = buffer_to_string(&terminal);

    // 1. Collections panel presence
    assert!(
        rendered.contains("Collections"),
        "Buffer must contain 'Collections' panel title"
    );
    assert!(
        rendered.contains("Machine Learning (2)"),
        "Buffer must contain collection item with paper count"
    );

    // 2. Papers panel presence
    assert!(
        rendered.contains("Papers"),
        "Buffer must contain 'Papers' panel title"
    );
    assert!(
        rendered.contains("Paper Title Alpha"),
        "Buffer must contain paper title"
    );
    assert!(rendered.contains("2023"), "Buffer must contain paper year");

    // 3. Details / Metadata panel presence
    assert!(
        rendered.contains("Details / Metadata")
            || rendered.contains("Details")
            || rendered.contains("Metadata"),
        "Buffer must contain Details/Metadata panel title"
    );
    assert!(
        rendered.contains("Author Alpha"),
        "Buffer must contain paper authors in metadata"
    );
    assert!(
        rendered.contains("10.1000/Alpha"),
        "Buffer must contain paper DOI in metadata"
    );
    assert!(
        rendered.contains("/papers/Alpha.pdf"),
        "Buffer must contain paper file_path in metadata"
    );
    assert!(
        rendered.contains("hash_Alpha"),
        "Buffer must contain paper content_hash in metadata"
    );
    assert!(
        rendered.contains("Abstract text for Alpha"),
        "Buffer must contain paper abstract in metadata"
    );
    assert!(
        rendered.contains("1. Introduction"),
        "Buffer must contain TOC preview entry"
    );
    assert!(
        rendered.contains("(p. 1)"),
        "Buffer must contain TOC page number"
    );

    // 4. Status / help bar presence
    assert!(
        rendered.contains("Tab"),
        "Buffer must contain 'Tab' keybinding in status bar"
    );
    assert!(
        rendered.contains("BackTab"),
        "Buffer must contain 'BackTab' keybinding in status bar"
    );
    assert!(
        rendered.contains("j/k"),
        "Buffer must contain 'j/k' navigation keybinding in status bar"
    );
    assert!(
        rendered.contains("Quit") || rendered.contains("q: Quit"),
        "Buffer must contain 'Quit' keybinding in status bar"
    );
}

/// RT-15: Panel navigation test.
/// Dispatching NextPanel cycles through Collections -> Papers -> Details -> Collections.
/// Rendering asserts that the focused border / style changes to indicate the active panel.
/// Dispatching PreviousPanel cycles in reverse.
#[test]
fn test_rt_15_panel_navigation() {
    let mut app = App::new();
    let p = sample_paper("Nav", 2024);
    app.add_collection(
        CollectionItem::new(Some(Uuid::now_v7()), "Navigation Test", 1),
        vec![p],
    );

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    // Coordinates for top-left corners of the 3 panels at width 120:
    // [25% (30), 45% (54), 30% (36)] => x: 0, 30, 84
    let col_x = 0;
    let paper_x = 30;
    let details_x = 84;
    let top_y = 0;

    // --- State 1: Collections active ---
    assert_eq!(app.active_panel, ActivePanel::Collections);
    terminal.draw(|f| render(&app, f)).expect("draw");
    {
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(col_x, top_y)].fg,
            Color::Cyan,
            "Collections border should have active Color::Cyan"
        );
        assert_eq!(
            buffer[(paper_x, top_y)].fg,
            Color::DarkGray,
            "Papers border should have inactive Color::DarkGray"
        );
        assert_eq!(
            buffer[(details_x, top_y)].fg,
            Color::DarkGray,
            "Details border should have inactive Color::DarkGray"
        );
    }

    // --- NextPanel -> Papers active ---
    app.dispatch(Action::NextPanel);
    assert_eq!(app.active_panel, ActivePanel::Papers);
    terminal.draw(|f| render(&app, f)).expect("draw");
    {
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(col_x, top_y)].fg,
            Color::DarkGray,
            "Collections border should now be inactive"
        );
        assert_eq!(
            buffer[(paper_x, top_y)].fg,
            Color::Cyan,
            "Papers border should now be active Color::Cyan"
        );
        assert_eq!(
            buffer[(details_x, top_y)].fg,
            Color::DarkGray,
            "Details border should be inactive"
        );
    }

    // --- NextPanel -> Details active ---
    app.dispatch(Action::NextPanel);
    assert_eq!(app.active_panel, ActivePanel::Details);
    terminal.draw(|f| render(&app, f)).expect("draw");
    {
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(col_x, top_y)].fg,
            Color::DarkGray,
            "Collections border should be inactive"
        );
        assert_eq!(
            buffer[(paper_x, top_y)].fg,
            Color::DarkGray,
            "Papers border should be inactive"
        );
        assert_eq!(
            buffer[(details_x, top_y)].fg,
            Color::Cyan,
            "Details border should now be active Color::Cyan"
        );
    }

    // --- NextPanel -> wrap around to Collections ---
    app.dispatch(Action::NextPanel);
    assert_eq!(app.active_panel, ActivePanel::Collections);
    terminal.draw(|f| render(&app, f)).expect("draw");
    {
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(col_x, top_y)].fg,
            Color::Cyan,
            "Collections border should cycle back to active Color::Cyan"
        );
        assert_eq!(buffer[(paper_x, top_y)].fg, Color::DarkGray);
        assert_eq!(buffer[(details_x, top_y)].fg, Color::DarkGray);
    }

    // --- PreviousPanel -> reverse cycle to Details ---
    app.dispatch(Action::PreviousPanel);
    assert_eq!(app.active_panel, ActivePanel::Details);

    // --- PreviousPanel -> reverse cycle to Papers ---
    app.dispatch(Action::PreviousPanel);
    assert_eq!(app.active_panel, ActivePanel::Papers);

    // --- PreviousPanel -> reverse cycle to Collections ---
    app.dispatch(Action::PreviousPanel);
    assert_eq!(app.active_panel, ActivePanel::Collections);
}

/// RT-16: List navigation test.
/// - In Collections panel, MoveDown and MoveUp change selected collection and clamp at boundaries (0 and max).
/// - In Papers panel, MoveDown and MoveUp change selected paper, update the Details panel with the newly
///   selected paper's metadata, and clamp at boundaries.
/// - Verifies selection state and rendered content.
#[test]
fn test_rt_16_list_navigation() {
    let mut app = App::new();

    let p1_a = sample_paper("AI_One", 2021);
    let p1_b = sample_paper("AI_Two", 2022);
    let p2_a = sample_paper("Sys_One", 2023);

    let c1_id = Uuid::now_v7();
    let c2_id = Uuid::now_v7();

    app.add_collection(
        CollectionItem::new(Some(c1_id), "AI", 2),
        vec![p1_a.clone(), p1_b.clone()],
    );
    app.add_collection(
        CollectionItem::new(Some(c2_id), "Systems", 1),
        vec![p2_a.clone()],
    );

    app.set_paper_tocs(p1_a.id, vec![sample_toc(p1_a.id, "TOC AI 1", 1, 0)]);
    app.set_paper_tocs(p1_b.id, vec![sample_toc(p1_b.id, "TOC AI 2", 10, 0)]);
    app.set_paper_tocs(p2_a.id, vec![sample_toc(p2_a.id, "TOC Sys 1", 20, 0)]);

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    // Initially: Collection 0 ("AI") is selected, Paper 0 ("AI_One") is selected
    assert_eq!(app.selected_collection, 0);
    assert_eq!(app.selected_paper, 0);
    assert_eq!(app.papers.len(), 2);
    assert_eq!(
        app.current_paper().unwrap().title.as_deref(),
        Some("Paper Title AI_One")
    );

    terminal.draw(|f| render(&app, f)).expect("draw");
    let initial_rendered = buffer_to_string(&terminal);
    assert!(initial_rendered.contains("Paper Title AI_One"));
    assert!(initial_rendered.contains("TOC AI 1"));

    // --- Collections navigation: boundary clamping at 0 ---
    app.dispatch(Action::MoveUp);
    assert_eq!(
        app.selected_collection, 0,
        "MoveUp at index 0 must clamp at 0"
    );

    // --- MoveDown to Collection 1 ("Systems") ---
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_collection, 1);
    assert_eq!(app.selected_paper, 0);
    assert_eq!(app.papers.len(), 1);
    assert_eq!(
        app.current_paper().unwrap().title.as_deref(),
        Some("Paper Title Sys_One")
    );

    terminal.draw(|f| render(&app, f)).expect("draw");
    let sys_rendered = buffer_to_string(&terminal);
    assert!(sys_rendered.contains("Paper Title Sys_One"));
    assert!(sys_rendered.contains("TOC Sys 1"));
    assert!(!sys_rendered.contains("Paper Title AI_One"));

    // --- Boundary clamping at max collection index ---
    app.dispatch(Action::MoveDown);
    assert_eq!(
        app.selected_collection, 1,
        "MoveDown at max index must clamp"
    );

    // --- MoveUp back to Collection 0 ("AI") ---
    app.dispatch(Action::MoveUp);
    assert_eq!(app.selected_collection, 0);
    assert_eq!(app.selected_paper, 0);
    assert_eq!(app.papers.len(), 2);
    assert_eq!(
        app.current_paper().unwrap().title.as_deref(),
        Some("Paper Title AI_One")
    );

    // --- Switch to Papers panel ---
    app.dispatch(Action::NextPanel);
    assert_eq!(app.active_panel, ActivePanel::Papers);

    // Initial paper is index 0 ("AI_One")
    assert_eq!(app.selected_paper, 0);

    // Boundary clamping at 0 in Papers panel
    app.dispatch(Action::MoveUp);
    assert_eq!(
        app.selected_paper, 0,
        "MoveUp at index 0 in papers must clamp at 0"
    );

    // --- MoveDown to Paper 1 ("AI_Two") ---
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_paper, 1);
    assert_eq!(
        app.current_paper().unwrap().title.as_deref(),
        Some("Paper Title AI_Two")
    );

    // Verify Details panel updated to AI_Two's metadata and TOC preview
    terminal.draw(|f| render(&app, f)).expect("draw");
    let ai_two_rendered = buffer_to_string(&terminal);
    assert!(
        ai_two_rendered.contains("Paper Title AI_Two"),
        "Details panel must display newly selected paper title"
    );
    assert!(
        ai_two_rendered.contains("Author AI_Two"),
        "Details panel must display newly selected paper authors"
    );
    assert!(
        ai_two_rendered.contains("TOC AI 2"),
        "Details panel must display newly selected paper TOC preview"
    );

    // --- Boundary clamping at max paper index ---
    app.dispatch(Action::MoveDown);
    assert_eq!(
        app.selected_paper, 1,
        "MoveDown at max paper index must clamp"
    );

    // --- MoveUp back to Paper 0 ("AI_One") ---
    app.dispatch(Action::MoveUp);
    assert_eq!(app.selected_paper, 0);
    assert_eq!(
        app.current_paper().unwrap().title.as_deref(),
        Some("Paper Title AI_One")
    );

    terminal.draw(|f| render(&app, f)).expect("draw");
    let back_to_one_rendered = buffer_to_string(&terminal);
    assert!(back_to_one_rendered.contains("Paper Title AI_One"));
    assert!(back_to_one_rendered.contains("TOC AI 1"));
}

/// Tests loading App directly from SQLite database repositories.
#[test]
fn test_app_from_database_repositories() -> Result<(), RepoError> {
    let conn = open_in_memory().expect("open in memory db");

    let c1 = Collection {
        id: Uuid::now_v7(),
        name: "Databases".to_string(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &c1)?;

    let paper = sample_paper("SQLite", 2000);
    PaperRepo::insert(&conn, &paper)?;
    CollectionRepo::add_paper(&conn, paper.id, c1.id)?;

    let toc = sample_toc(paper.id, "1. Architecture", 1, 0);
    TocRepo::insert(&conn, &toc)?;

    let app = App::from_db(&conn)?;

    assert!(!app.collections.is_empty());
    // Should have "All Papers" and "Databases"
    let db_col = app.collections.iter().find(|c| c.name == "Databases");
    assert!(db_col.is_some(), "Must have Databases collection");
    assert_eq!(db_col.unwrap().paper_count, 1);

    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.current_paper().unwrap().title, paper.title);
    assert_eq!(app.toc_preview.len(), 1);
    assert_eq!(app.toc_preview[0].title, "1. Architecture");

    Ok(())
}

/// Tests that Action::Quit stops running.
#[test]
fn test_action_quit() {
    let mut app = App::new();
    assert!(app.running);
    app.dispatch(Action::Quit);
    assert!(!app.running);
}

/// Tests key event mapping integration.
#[test]
fn test_key_event_mapping_integration() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    assert_eq!(map_key_event(tab), Some(Action::NextPanel));

    let shift_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
    assert_eq!(map_key_event(shift_tab), Some(Action::PreviousPanel));

    let k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(map_key_event(k), Some(Action::MoveUp));

    let j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(map_key_event(j), Some(Action::MoveDown));

    let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    assert_eq!(map_key_event(q), Some(Action::Quit));
}

/// Tests that when a paper has an empty TOC, selected_toc remains 0
/// even when MoveUp or MoveDown is dispatched on the Details panel,
/// and that switching papers resets selected_toc to 0.
#[test]
fn test_empty_toc_navigation_and_paper_switch_reset() {
    let mut app = App::new();

    let p1 = sample_paper("WithTOC", 2023);
    let p2 = sample_paper("EmptyTOC", 2024);

    app.add_collection(
        CollectionItem::new(Some(Uuid::now_v7()), "TOC Test", 2),
        vec![p1.clone(), p2.clone()],
    );
    app.set_paper_tocs(
        p1.id,
        vec![
            sample_toc(p1.id, "1. Introduction", 1, 0),
            sample_toc(p1.id, "2. Details", 5, 1),
        ],
    );
    // p2 has no TOC entries (empty TOC)

    // Focus Details panel on p1 and move down in TOC
    app.active_panel = ActivePanel::Details;
    assert_eq!(app.selected_toc, 0);
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_toc, 1);

    // Switch to Papers panel and navigate to p2
    app.active_panel = ActivePanel::Papers;
    app.dispatch(Action::MoveDown);
    assert_eq!(app.selected_paper, 1);
    assert!(app.toc_preview.is_empty());
    assert_eq!(
        app.selected_toc, 0,
        "selected_toc must reset to 0 when switching papers"
    );

    // Switch to Details panel for paper with empty TOC
    app.active_panel = ActivePanel::Details;
    assert_eq!(app.selected_toc, 0);

    // Dispatch MoveUp on Details panel with empty TOC
    app.dispatch(Action::MoveUp);
    assert_eq!(
        app.selected_toc, 0,
        "selected_toc must remain 0 on MoveUp when TOC is empty"
    );

    // Dispatch MoveDown on Details panel with empty TOC
    app.dispatch(Action::MoveDown);
    assert_eq!(
        app.selected_toc, 0,
        "selected_toc must remain 0 on MoveDown when TOC is empty"
    );
}

/// Tests that search bar renders when search is active,
/// filters papers upon input, and removes the search bar upon Esc / cancel.
#[test]
fn test_search_mode_rendering_and_filtering() {
    let mut app = App::new();
    let p1 = sample_paper("Attention", 2017);
    let p2 = sample_paper("Residual", 2016);
    let col_id = Uuid::now_v7();

    app.add_collection(
        CollectionItem::new(Some(col_id), "ML", 2),
        vec![p1.clone(), p2.clone()],
    );

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).expect("create terminal");

    // 1. Initially, no search bar is rendered
    terminal.draw(|f| render(&app, f)).expect("draw");
    let initial_rendered = buffer_to_string(&terminal);
    assert!(!initial_rendered.contains("Search [/]"));
    assert!(initial_rendered.contains("Paper Title Attention"));
    assert!(initial_rendered.contains("Paper Title Residual"));

    // 2. Dispatch Action::Search -> activates search prompt
    app.dispatch(Action::Search);
    assert!(app.is_searching);

    terminal.draw(|f| render(&app, f)).expect("draw");
    let search_active_rendered = buffer_to_string(&terminal);
    assert!(
        search_active_rendered.contains("Search [/]"),
        "Search bar must be rendered when is_searching is true"
    );

    // 3. Type 'A' -> 't' -> 't'
    for c in ['A', 't', 't'] {
        app.dispatch(Action::SearchInput(c));
    }
    assert_eq!(app.search_query, "Att");
    assert_eq!(app.papers.len(), 1);
    assert_eq!(app.papers[0].id, p1.id);

    terminal.draw(|f| render(&app, f)).expect("draw");
    let filtered_rendered = buffer_to_string(&terminal);
    assert!(filtered_rendered.contains("Paper Title Attention"));
    assert!(!filtered_rendered.contains("Paper Title Residual"));
    assert!(filtered_rendered.contains("/ Att"));

    // 4. Confirm search (Enter): exits typing mode, keeps filtered results
    app.dispatch(Action::SearchConfirm);
    assert!(!app.is_searching);
    assert_eq!(app.papers.len(), 1);

    terminal.draw(|f| render(&app, f)).expect("draw");
    let confirmed_rendered = buffer_to_string(&terminal);
    assert!(
        confirmed_rendered.contains("Filtered"),
        "When search_query is non-empty, search bar displays Filtered title"
    );

    // 5. Cancel search (Esc): clears query and restores full list
    app.dispatch(Action::SearchCancel);
    assert!(!app.is_searching);
    assert!(app.search_query.is_empty());
    assert_eq!(app.papers.len(), 2);

    terminal.draw(|f| render(&app, f)).expect("draw");
    let cancelled_rendered = buffer_to_string(&terminal);
    assert!(!cancelled_rendered.contains("Search [/]"));
    assert!(!cancelled_rendered.contains("Filtered"));
    assert!(cancelled_rendered.contains("Paper Title Attention"));
    assert!(cancelled_rendered.contains("Paper Title Residual"));
}

#[test]
fn test_add_paper_modal_flow_and_rendering() {
    let mut app = App::new();
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).expect("init test terminal");

    // 1. Key event mapping: 'a' in Papers panel opens modal
    let key_event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('a'),
        crossterm::event::KeyModifiers::NONE,
    );
    let action =
        papyrus_tui::event::map_key_event_with_context(key_event, ActivePanel::Papers, None);
    assert_eq!(action, Some(Action::AddPaperModalOpen));

    // 2. Open modal and check rendering
    app.dispatch(Action::AddPaperModalOpen);
    assert!(app.is_adding_paper);

    terminal.draw(|f| render(&app, f)).expect("draw");
    let rendered = buffer_to_string(&terminal);
    assert!(rendered.contains("Add Paper to Library"));
    assert!(rendered.contains("Path to PDF file"));

    // 3. Type characters
    for c in "my_paper.pdf".chars() {
        app.dispatch(Action::AddPaperModalInput(c));
    }
    assert_eq!(app.add_paper_path_buffer, "my_paper.pdf");

    terminal.draw(|f| render(&app, f)).expect("draw");
    let rendered_typed = buffer_to_string(&terminal);
    assert!(rendered_typed.contains("my_paper.pdf"));

    // 4. Cancel modal
    app.dispatch(Action::AddPaperModalCancel);
    assert!(!app.is_adding_paper);
    assert!(app.add_paper_path_buffer.is_empty());

    terminal.draw(|f| render(&app, f)).expect("draw");
    let rendered_closed = buffer_to_string(&terminal);
    assert!(!rendered_closed.contains("Add Paper to Library"));
}
