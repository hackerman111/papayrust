use lopdf::{dictionary, Bookmark, Document, Object, Stream};
use papyrus_core::config::Config;
use papyrus_core::db::{open_in_memory, Collection, CollectionRepo, PaperRepo, TocRepo, TocSource};
use papyrus_core::importer::{import_paper, ImportError};
use papyrus_core::pdf::{decode_pdf_string, extract_text, extract_text_from_mem};
use std::path::Path;
use tempfile::tempdir;
use uuid::Uuid;

/// Options for building a synthetic PDF document.
#[derive(Default)]
struct SyntheticPdfOptions<'a> {
    title: Option<&'a str>,
    author: Option<&'a str>,
    year: Option<i64>,
    doi: Option<&'a str>,
    journal: Option<&'a str>,
    page_count: usize,
    page_text: Option<&'a str>,
}

/// Helper to construct a synthetic PDF document using lopdf.
fn create_synthetic_pdf(path: &Path, opts: SyntheticPdfOptions<'_>) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut page_ids = Vec::new();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let count = opts.page_count.max(1);
    for i in 1..=count {
        let text = opts.page_text.unwrap_or("Hello World from synthetic PDF");
        let content_str = format!("BT /F1 12 Tf 100 700 Td ({text} - Page {i}) Tj ET");
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

    // Build Info dictionary if any metadata field is provided
    if opts.title.is_some()
        || opts.author.is_some()
        || opts.year.is_some()
        || opts.doi.is_some()
        || opts.journal.is_some()
    {
        let mut info_dict = lopdf::Dictionary::new();
        if let Some(t) = opts.title {
            info_dict.set("Title", Object::string_literal(t));
        }
        if let Some(a) = opts.author {
            info_dict.set("Author", Object::string_literal(a));
        }
        if let Some(y) = opts.year {
            info_dict.set(
                "CreationDate",
                Object::string_literal(format!("D:{y:04}0601000000Z")),
            );
        }
        if let Some(d) = opts.doi {
            info_dict.set("DOI", Object::string_literal(d));
        }
        if let Some(j) = opts.journal {
            info_dict.set("Journal", Object::string_literal(j));
        }
        let info_id = doc.add_object(info_dict);
        doc.trailer.set("Info", info_id);
    }

    doc.save(path).expect("save synthetic PDF");
}

/// Helper specification for hierarchical outlines in tests.
struct OutlineSpec {
    title: &'static str,
    page_index: usize, // 0-based page index
    children: Vec<OutlineSpec>,
}

/// Creates a multi-page PDF with hierarchical bookmarks/outlines using lopdf.
fn create_synthetic_pdf_with_outlines(path: &Path, outlines: &[OutlineSpec]) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });

    let num_pages = 4;
    let mut page_ids = Vec::new();
    for i in 1..=num_pages {
        let content_str = format!("BT /F1 12 Tf 100 700 Td (Section content on page {i}) Tj ET");
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
            "Kids" => page_ids.iter().copied().map(Into::into).collect::<Vec<Object>>(),
            "Count" => num_pages as i64,
        },
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    // Recursive helper to add bookmarks
    fn add_specs(
        doc: &mut Document,
        page_ids: &[lopdf::ObjectId],
        specs: &[OutlineSpec],
        parent_bmid: Option<u32>,
    ) {
        for spec in specs {
            let target_page = page_ids[spec.page_index.min(page_ids.len() - 1)];
            let bmid = doc.add_bookmark(
                Bookmark::new(spec.title.to_string(), [0.0, 0.0, 0.0], 0, target_page),
                parent_bmid,
            );
            add_specs(doc, page_ids, &spec.children, Some(bmid));
        }
    }

    add_specs(&mut doc, &page_ids, outlines, None);

    if let Some(outline_id) = doc.build_outline() {
        let catalog = doc.catalog_mut().expect("catalog mut");
        catalog.set("Outlines", outline_id);
    }

    // Also set basic Info metadata
    let info = dictionary! {
        "Title" => Object::string_literal("Structured Document with Outlines"),
        "Author" => Object::string_literal("Alice Researcher"),
        "CreationDate" => Object::string_literal("D:20240415100000Z"),
    };
    let info_id = doc.add_object(info);
    doc.trailer.set("Info", info_id);

    doc.save(path).expect("save synthetic PDF with outlines");
}

/// RT-09: Successful import of valid PDF (metadata extracted, stored in DB).
#[test]
fn test_rt_09_successful_import_valid_pdf() {
    let mut conn = open_in_memory().expect("open DB");
    let dir = tempdir().expect("tempdir");
    let pdf_path = dir.path().join("attention_is_all_you_need.pdf");

    create_synthetic_pdf(
        &pdf_path,
        SyntheticPdfOptions {
            title: Some("Attention Is All You Need"),
            author: Some("Vaswani et al."),
            year: Some(2017),
            doi: Some("10.48550/arXiv.1706.03762"),
            journal: Some("NeurIPS 2017"),
            page_count: 2,
            page_text: Some("Transformer architecture paper"),
        },
    );

    let config = Config::default();
    let paper = import_paper(&mut conn, &config, &pdf_path, None).expect("import valid PDF");

    // 1. Verify extracted paper fields
    assert_eq!(paper.title.as_deref(), Some("Attention Is All You Need"));
    assert_eq!(paper.authors.as_deref(), Some("Vaswani et al."));
    assert_eq!(paper.year, Some(2017));
    assert_eq!(paper.doi.as_deref(), Some("10.48550/arXiv.1706.03762"));
    assert_eq!(paper.journal.as_deref(), Some("NeurIPS 2017"));
    assert_eq!(paper.file_path, pdf_path.to_string_lossy());
    assert_eq!(
        paper.content_hash.len(),
        64,
        "SHA-256 hash must be 64 hex characters"
    );

    // 2. Verify stored in database
    let stored = PaperRepo::get_by_id(&conn, paper.id)
        .expect("query paper")
        .expect("paper must exist in DB");
    assert_eq!(stored.id, paper.id);
    assert_eq!(stored.content_hash, paper.content_hash);
    assert_eq!(stored.title, Some("Attention Is All You Need".to_string()));
    assert_eq!(stored.year, Some(2017));
}

/// RT-10: Deduplication by content hash (importing same PDF returns Duplicate, DB unchanged).
#[test]
fn test_rt_10_deduplication_by_content_hash() {
    let mut conn = open_in_memory().expect("open DB");
    let dir = tempdir().expect("tempdir");
    let pdf_path = dir.path().join("dedup_paper.pdf");

    create_synthetic_pdf(
        &pdf_path,
        SyntheticPdfOptions {
            title: Some("Dedup Test Paper"),
            author: Some("Bob"),
            year: Some(2023),
            doi: None,
            journal: None,
            page_count: 1,
            page_text: Some("Dedup content"),
        },
    );

    let config = Config::default();
    let paper1 = import_paper(&mut conn, &config, &pdf_path, None).expect("initial import");

    let initial_count = PaperRepo::list(&conn).expect("list papers").len();
    assert_eq!(initial_count, 1);

    // 1. Re-importing exact same file path returns Duplicate
    let dup_err = import_paper(&mut conn, &config, &pdf_path, None);
    assert!(dup_err.is_err(), "Duplicate import must fail");
    match dup_err.unwrap_err() {
        ImportError::Duplicate {
            existing_id,
            content_hash,
            file_path,
        } => {
            assert_eq!(existing_id, paper1.id);
            assert_eq!(content_hash, paper1.content_hash);
            assert_eq!(file_path, paper1.file_path);
        }
        other => panic!("Expected Duplicate error, got: {other:?}"),
    }

    // 2. Importing a file with identical bytes from a different path returns Duplicate
    let copy_path = dir.path().join("copy_of_dedup_paper.pdf");
    std::fs::copy(&pdf_path, &copy_path).expect("copy file");

    let copy_dup_err = import_paper(&mut conn, &config, &copy_path, None);
    assert!(copy_dup_err.is_err(), "Duplicate copy must fail");
    match copy_dup_err.unwrap_err() {
        ImportError::Duplicate {
            existing_id,
            content_hash,
            file_path,
        } => {
            assert_eq!(existing_id, paper1.id);
            assert_eq!(content_hash, paper1.content_hash);
            assert_eq!(file_path, paper1.file_path);
        }
        other => panic!("Expected Duplicate error, got: {other:?}"),
    }

    // 3. Database must remain completely unchanged
    let final_papers = PaperRepo::list(&conn).expect("list papers");
    assert_eq!(final_papers.len(), 1, "DB count must remain 1");
    assert_eq!(final_papers[0].id, paper1.id);
}

/// RT-11: Auto-extraction of TOC from PDF `/Outlines` on import (creates toc_entries with source=Auto).
#[test]
fn test_rt_11_auto_extraction_of_toc_outlines() {
    let mut conn = open_in_memory().expect("open DB");
    let dir = tempdir().expect("tempdir");
    let pdf_path = dir.path().join("outlined_paper.pdf");

    let outline_tree = vec![
        OutlineSpec {
            title: "1. Introduction",
            page_index: 0, // Page 1
            children: vec![],
        },
        OutlineSpec {
            title: "2. Architecture",
            page_index: 1, // Page 2
            children: vec![
                OutlineSpec {
                    title: "2.1 Encoder",
                    page_index: 1, // Page 2
                    children: vec![],
                },
                OutlineSpec {
                    title: "2.2 Decoder",
                    page_index: 2, // Page 3
                    children: vec![],
                },
            ],
        },
        OutlineSpec {
            title: "3. Conclusion",
            page_index: 3, // Page 4
            children: vec![],
        },
    ];

    create_synthetic_pdf_with_outlines(&pdf_path, &outline_tree);

    let config = Config::default();
    assert!(
        config.toc.auto_extract_on_import,
        "Default config must enable auto TOC extraction"
    );

    let paper =
        import_paper(&mut conn, &config, &pdf_path, None).expect("import paper with outlines");

    // Query extracted TOC entries
    let toc_entries = TocRepo::get_by_paper(&conn, paper.id).expect("get TOC entries");
    assert_eq!(toc_entries.len(), 5, "Must extract 5 outline entries");

    // All entries must have source = TocSource::Auto and paper_id = paper.id
    for entry in &toc_entries {
        assert_eq!(entry.source, TocSource::Auto);
        assert_eq!(entry.paper_id, paper.id);
    }

    // Verify root entries (parent_id == None)
    let roots: Vec<_> = toc_entries
        .iter()
        .filter(|e| e.parent_id.is_none())
        .collect();
    assert_eq!(roots.len(), 3, "There must be 3 top-level chapters");
    assert_eq!(roots[0].title, "1. Introduction");
    assert_eq!(roots[0].page_number, 1);
    assert_eq!(roots[0].order_index, 0);

    assert_eq!(roots[1].title, "2. Architecture");
    assert_eq!(roots[1].page_number, 2);
    assert_eq!(roots[1].order_index, 1);

    assert_eq!(roots[2].title, "3. Conclusion");
    assert_eq!(roots[2].page_number, 4);
    assert_eq!(roots[2].order_index, 2);

    // Verify child entries of "2. Architecture"
    let arch_id = roots[1].id;
    let arch_children: Vec<_> = toc_entries
        .iter()
        .filter(|e| e.parent_id == Some(arch_id))
        .collect();
    assert_eq!(
        arch_children.len(),
        2,
        "Architecture must have 2 child sections"
    );

    assert_eq!(arch_children[0].title, "2.1 Encoder");
    assert_eq!(arch_children[0].page_number, 2);
    assert_eq!(arch_children[0].order_index, 0);

    assert_eq!(arch_children[1].title, "2.2 Decoder");
    assert_eq!(arch_children[1].page_number, 3);
    assert_eq!(arch_children[1].order_index, 1);
}

/// RT-12: PDF with missing metadata (handles gracefully without panic: filename as title).
#[test]
fn test_rt_12_pdf_with_missing_metadata() {
    let mut conn = open_in_memory().expect("open DB");
    let dir = tempdir().expect("tempdir");
    let pdf_path = dir.path().join("neural_networks_survey.pdf");

    // PDF with no Info dictionary at all
    create_synthetic_pdf(
        &pdf_path,
        SyntheticPdfOptions {
            title: None,
            author: None,
            year: None,
            doi: None,
            journal: None,
            page_count: 1,
            page_text: Some("Missing metadata content"),
        },
    );

    let config = Config::default();
    let paper = import_paper(&mut conn, &config, &pdf_path, None)
        .expect("must import gracefully without panic");

    // Title falls back to filename without extension
    assert_eq!(
        paper.title.as_deref(),
        Some("neural_networks_survey"),
        "Missing Title must fallback to file stem"
    );
    assert_eq!(paper.year, None, "Missing date must result in year = None");
    assert_eq!(paper.authors, None);
    assert_eq!(paper.doi, None);
    assert_eq!(paper.journal, None);

    // Also test PDF with UTF-16BE BOM encoded Title
    let utf16_pdf_path = dir.path().join("utf16_paper.pdf");
    {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });
        doc.set_object(
            pages_id,
            dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_id.into()],
                "Count" => 1,
            },
        );
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        // UTF-16BE BOM for "Quantum Computing"
        let mut utf16_bytes = vec![0xFE, 0xFF];
        for unit in "Quantum Computing".encode_utf16() {
            utf16_bytes.extend_from_slice(&unit.to_be_bytes());
        }
        let info_id = doc.add_object(dictionary! {
            "Title" => Object::String(utf16_bytes, lopdf::StringFormat::Literal),
        });
        doc.trailer.set("Info", info_id);
        doc.save(&utf16_pdf_path).expect("save utf16 PDF");
    }

    let paper_utf16 = import_paper(&mut conn, &config, &utf16_pdf_path, None)
        .expect("import UTF-16BE encoded PDF");
    assert_eq!(paper_utf16.title.as_deref(), Some("Quantum Computing"));
}

/// RT-13: Corrupted / invalid PDF (returns InvalidPdf, DB unchanged).
#[test]
fn test_rt_13_corrupted_or_invalid_pdf() {
    let mut conn = open_in_memory().expect("open DB");
    let dir = tempdir().expect("tempdir");
    let corrupt_path = dir.path().join("corrupted_file.pdf");

    // Write non-PDF garbage bytes
    std::fs::write(&corrupt_path, b"NOT_A_VALID_PDF_HEADER_1234567890")
        .expect("write corrupt file");

    let config = Config::default();
    let res = import_paper(&mut conn, &config, &corrupt_path, None);

    assert!(res.is_err(), "Corrupted PDF must return error");
    match res.unwrap_err() {
        ImportError::InvalidPdf { path, reason } => {
            assert_eq!(path, corrupt_path);
            assert!(!reason.is_empty(), "Error reason must be non-empty");
        }
        other => panic!("Expected InvalidPdf, got: {other:?}"),
    }

    // Database must have 0 papers
    let papers = PaperRepo::list(&conn).expect("list papers");
    assert!(
        papers.is_empty(),
        "DB must remain empty after failed import"
    );
}

/// Tests import with collection association and transaction rollback on failure.
#[test]
fn test_import_with_collection_linking_and_rollback() {
    let mut conn = open_in_memory().expect("open DB");
    let dir = tempdir().expect("tempdir");
    let pdf_path = dir.path().join("collected_paper.pdf");

    create_synthetic_pdf(
        &pdf_path,
        SyntheticPdfOptions {
            title: Some("Collected Paper"),
            author: Some("Charlie"),
            year: Some(2024),
            doi: None,
            journal: None,
            page_count: 1,
            page_text: Some("Collected paper content"),
        },
    );

    let config = Config::default();

    // 1. Valid collection linking
    let col_id = Uuid::now_v7();
    let col = Collection {
        id: col_id,
        name: "Deep Learning".to_string(),
        parent_id: None,
    };
    CollectionRepo::insert(&conn, &col).expect("insert collection");

    let paper = import_paper(&mut conn, &config, &pdf_path, Some(col_id))
        .expect("import with valid collection");
    let col_papers = CollectionRepo::get_papers(&conn, col_id).expect("get collection papers");
    assert_eq!(col_papers.len(), 1);
    assert_eq!(col_papers[0].id, paper.id);

    // 2. Failed linking with non-existent collection must roll back transaction
    let non_existent_col_id = Uuid::now_v7();
    let pdf_path2 = dir.path().join("another_paper.pdf");
    create_synthetic_pdf(
        &pdf_path2,
        SyntheticPdfOptions {
            title: Some("Another Paper"),
            author: Some("Dave"),
            year: Some(2024),
            doi: None,
            journal: None,
            page_count: 1,
            page_text: Some("Another paper content"),
        },
    );

    let fail_res = import_paper(&mut conn, &config, &pdf_path2, Some(non_existent_col_id));
    assert!(
        fail_res.is_err(),
        "Importing with invalid collection must fail"
    );
    assert!(matches!(fail_res.unwrap_err(), ImportError::Db(_)));

    // Paper 2 must NOT be present in DB due to atomic rollback
    let papers_after = PaperRepo::list(&conn).expect("list papers");
    assert_eq!(
        papers_after.len(),
        1,
        "Paper 2 must not be in DB after rollback"
    );
}

/// Tests that setting auto_extract_on_import = false skips outline extraction.
#[test]
fn test_import_with_auto_extract_disabled() {
    let mut conn = open_in_memory().expect("open DB");
    let dir = tempdir().expect("tempdir");
    let pdf_path = dir.path().join("no_auto_extract.pdf");

    create_synthetic_pdf_with_outlines(
        &pdf_path,
        &[OutlineSpec {
            title: "Chapter 1",
            page_index: 0,
            children: vec![],
        }],
    );

    let mut config = Config::default();
    config.toc.auto_extract_on_import = false;

    let paper = import_paper(&mut conn, &config, &pdf_path, None).expect("import paper");
    let toc_entries = TocRepo::get_by_paper(&conn, paper.id).expect("get TOC entries");
    assert!(
        toc_entries.is_empty(),
        "TOC extraction must be skipped when config auto_extract_on_import = false"
    );
}

/// Tests PDF text extraction using pdf_extract wrapper.
#[test]
fn test_pdf_text_extraction() {
    let dir = tempdir().expect("tempdir");
    let pdf_path = dir.path().join("text_extraction.pdf");

    create_synthetic_pdf(
        &pdf_path,
        SyntheticPdfOptions {
            title: Some("Text Extraction Paper"),
            author: Some("Eve"),
            year: Some(2024),
            doi: None,
            journal: None,
            page_count: 2,
            page_text: Some("UniqueSearchablePhrase12345"),
        },
    );

    // File extraction
    let text = extract_text(&pdf_path).expect("extract text from path");
    assert!(
        text.contains("UniqueSearchablePhrase12345"),
        "Extracted text must contain content from page stream"
    );

    // Memory extraction
    let bytes = std::fs::read(&pdf_path).expect("read PDF bytes");
    let mem_text = extract_text_from_mem(&bytes).expect("extract text from mem");
    assert!(
        mem_text.contains("UniqueSearchablePhrase12345"),
        "Memory extracted text must contain content from page stream"
    );
}

/// Tests non-ASCII string decoding under PDFDocEncoding (e.g. copyright symbol, accented characters).
#[test]
fn test_non_ascii_pdfdocencoding_decoding() {
    // 1. Direct decoding check: 0xA9 = ©, 0xE9 = é, 0xF5 = õ
    let raw_bytes = b"Copyright \xA9 2024 Poincar\xE9 & Erd\xF5s";
    let decoded = decode_pdf_string(raw_bytes);
    assert_eq!(decoded, "Copyright © 2024 Poincaré & Erdõs");

    // 2. Integration check with imported PDF containing PDFDocEncoding non-ASCII strings in Info
    let mut conn = open_in_memory().expect("open DB");
    let dir = tempdir().expect("tempdir");
    let pdf_path = dir.path().join("pdfdocencoding_paper.pdf");

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    });
    doc.set_object(
        pages_id,
        dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        },
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let info_id = doc.add_object(dictionary! {
        "Title" => Object::String(b"Th\xE9or\xE8me de Poincar\xE9 \xA9".to_vec(), lopdf::StringFormat::Literal),
        "Author" => Object::String(b"Ren\xE9 Descartes".to_vec(), lopdf::StringFormat::Literal),
    });
    doc.trailer.set("Info", info_id);
    doc.save(&pdf_path).expect("save pdf");

    let config = Config::default();
    let paper = import_paper(&mut conn, &config, &pdf_path, None).expect("import paper");
    assert_eq!(paper.title.as_deref(), Some("Théorème de Poincaré ©"));
    assert_eq!(paper.authors.as_deref(), Some("René Descartes"));
}

/// Tests cycle detection on malformed PDFs proving termination.
#[test]
fn test_cycle_detection_on_malformed_pdf_terminates() {
    let mut conn = open_in_memory().expect("open DB");
    let dir = tempdir().expect("tempdir");
    let pdf_path = dir.path().join("cyclic_malformed.pdf");

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    });
    doc.set_object(
        pages_id,
        dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        },
    );

    // Cyclic destination references: dest1 -> dest2 -> dest1
    let dest1_id = doc.new_object_id();
    let dest2_id = doc.new_object_id();
    doc.set_object(dest1_id, Object::Reference(dest2_id));
    doc.set_object(dest2_id, Object::Reference(dest1_id));

    // Cyclic outline items: item1 -> item2 -> item1
    let outlines_id = doc.new_object_id();
    let item1_id = doc.new_object_id();
    let item2_id = doc.new_object_id();

    doc.set_object(
        item1_id,
        dictionary! {
            "Title" => Object::string_literal("Cyclic Node 1"),
            "Parent" => outlines_id,
            "Next" => item2_id,
            "Dest" => Object::Reference(dest1_id),
        },
    );
    doc.set_object(
        item2_id,
        dictionary! {
            "Title" => Object::string_literal("Cyclic Node 2"),
            "Parent" => outlines_id,
            "Next" => item1_id, // Cycle back to item1
            "Dest" => Object::Reference(dest2_id),
        },
    );
    doc.set_object(
        outlines_id,
        dictionary! {
            "Type" => "Outlines",
            "First" => item1_id,
            "Last" => item2_id,
            "Count" => 2,
        },
    );

    // Cyclic Name Tree: kid1 -> kid2 -> kid1
    let kid1_id = doc.new_object_id();
    let kid2_id = doc.new_object_id();
    doc.set_object(
        kid1_id,
        dictionary! {
            "Kids" => vec![kid2_id.into()],
        },
    );
    doc.set_object(
        kid2_id,
        dictionary! {
            "Kids" => vec![kid1_id.into()],
        },
    );
    let names_dests_id = doc.add_object(dictionary! {
        "Kids" => vec![kid1_id.into()],
    });
    let names_id = doc.add_object(dictionary! {
        "Dests" => names_dests_id,
    });

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
        "Outlines" => outlines_id,
        "Names" => names_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.save(&pdf_path).expect("save cyclic PDF");

    let config = Config::default();
    // Must terminate promptly without infinite loop or stack overflow
    let paper = import_paper(&mut conn, &config, &pdf_path, None)
        .expect("importing cyclic PDF must terminate safely");

    let entries = TocRepo::get_by_paper(&conn, paper.id).expect("get TOC");
    // Visited set ensures each cyclic item is visited at most once
    assert_eq!(
        entries.len(),
        2,
        "Must extract exactly 2 items before breaking cycle"
    );
    assert_eq!(entries[0].title, "Cyclic Node 1");
    assert_eq!(entries[1].title, "Cyclic Node 2");
}

/// Tests that metadata fields and outline titles stored as indirect Object::Reference strings resolve properly.
#[test]
fn test_indirect_object_reference_metadata_and_outlines() {
    let mut conn = open_in_memory().expect("open DB");
    let dir = tempdir().expect("tempdir");
    let pdf_path = dir.path().join("indirect_strings.pdf");

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    });
    doc.set_object(
        pages_id,
        dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        },
    );

    // Indirect string objects
    let title_str_id = doc.add_object(Object::string_literal("Indirect Title Paper"));
    let author_str_id = doc.add_object(Object::string_literal("Indirect Author Name"));
    let date_str_id = doc.add_object(Object::string_literal("D:20240901000000Z"));
    let doi_str_id = doc.add_object(Object::string_literal("10.1000/indirect.doi"));
    let journal_str_id = doc.add_object(Object::string_literal("Indirect Journal Review"));

    let info_id = doc.add_object(dictionary! {
        "Title" => Object::Reference(title_str_id),
        "Author" => Object::Reference(author_str_id),
        "CreationDate" => Object::Reference(date_str_id),
        "DOI" => Object::Reference(doi_str_id),
        "Journal" => Object::Reference(journal_str_id),
    });
    doc.trailer.set("Info", info_id);

    // Outline with indirect Title string
    let outline_title_id = doc.add_object(Object::string_literal("Indirect Outline Chapter"));
    let item_id = doc.add_object(dictionary! {
        "Title" => Object::Reference(outline_title_id),
        "Dest" => vec![page_id.into(), Object::Name(b"Fit".to_vec())],
    });
    let outlines_id = doc.add_object(dictionary! {
        "Type" => "Outlines",
        "First" => item_id,
        "Last" => item_id,
        "Count" => 1,
    });

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
        "Outlines" => outlines_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.save(&pdf_path).expect("save indirect string PDF");

    let config = Config::default();
    let paper = import_paper(&mut conn, &config, &pdf_path, None)
        .expect("import PDF with indirect strings");

    assert_eq!(paper.title.as_deref(), Some("Indirect Title Paper"));
    assert_eq!(paper.authors.as_deref(), Some("Indirect Author Name"));
    assert_eq!(paper.year, Some(2024));
    assert_eq!(paper.doi.as_deref(), Some("10.1000/indirect.doi"));
    assert_eq!(paper.journal.as_deref(), Some("Indirect Journal Review"));

    let entries = TocRepo::get_by_paper(&conn, paper.id).expect("get TOC entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].title, "Indirect Outline Chapter");
    assert_eq!(entries[0].page_number, 1);
}
