use std::path::Path;

use lopdf::{dictionary, Document, Object, Stream};
use papyrus_core::db::{apply_migrations, open_in_memory, Paper, PaperRepo};
use rusqlite::Connection;
use uuid::Uuid;

pub fn setup_db() -> Connection {
    let mut conn = open_in_memory().expect("open in-memory db");
    apply_migrations(&mut conn).expect("apply schema migrations");
    conn
}

pub fn create_synthetic_pdf(path: &Path, page_count: usize) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut page_ids = Vec::new();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let count = page_count.max(1);
    for i in 1..=count {
        let content_str = format!("BT /F1 12 Tf 100 700 Td (Synthetic Page {i}) Tj ET");
        let content_id = doc.add_object(Stream::new(dictionary!(), content_str.into_bytes()));

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! {
                    "F1" => font_id,
                },
            },
        });
        page_ids.push(page_id);
    }

    doc.set_object(
        pages_id,
        dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids.into_iter().map(Into::into).collect::<Vec<Object>>(),
            "Count" => count as i64,
        },
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    doc.save(path).expect("save synthetic PDF");
}

pub fn create_synthetic_pdf_with_outline(path: &Path) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut page_ids = Vec::new();

    for i in 1..=5 {
        let content_str = format!("BT 100 700 Td (Page {i}) ET");
        let content_id = doc.add_object(Stream::new(dictionary!(), content_str.into_bytes()));

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });
        page_ids.push(page_id);
    }

    doc.set_object(
        pages_id,
        dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids.iter().copied().map(Into::into).collect::<Vec<Object>>(),
            "Count" => 5i64,
        },
    );

    let outlines_id = doc.new_object_id();
    let item1_id = doc.new_object_id();
    let item2_id = doc.new_object_id();

    // Item 1 -> Page 1
    doc.set_object(
        item1_id,
        dictionary! {
            "Title" => Object::string_literal("Chapter 1"),
            "Parent" => outlines_id,
            "Next" => item2_id,
            "Dest" => vec![page_ids[0].into(), Object::Name(b"Fit".to_vec())],
        },
    );

    // Item 2 -> Page 3
    doc.set_object(
        item2_id,
        dictionary! {
            "Title" => Object::string_literal("Chapter 2"),
            "Parent" => outlines_id,
            "Prev" => item1_id,
            "Dest" => vec![page_ids[2].into(), Object::Name(b"Fit".to_vec())],
        },
    );

    doc.set_object(
        outlines_id,
        dictionary! {
            "Type" => "Outlines",
            "First" => item1_id,
            "Last" => item2_id,
            "Count" => 2i64,
        },
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
        "Outlines" => outlines_id,
    });
    doc.trailer.set("Root", catalog_id);

    doc.save(path).expect("save outline PDF");
}

pub fn insert_test_paper(conn: &Connection, pdf_path: &Path) -> Paper {
    let paper = Paper {
        id: Uuid::now_v7(),
        file_path: pdf_path.to_string_lossy().to_string(),
        content_hash: "hash123".to_string(),
        title: Some("TOC Research Paper".to_string()),
        authors: Some("Ada Lovelace".to_string()),
        year: Some(2024),
        journal: None,
        doi: None,
        abstract_text: None,
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };
    PaperRepo::insert(conn, &paper).expect("insert paper");
    paper
}
