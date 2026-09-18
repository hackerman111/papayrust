use papyrus_core::db::Paper;
use papyrus_core::search::SearchIndex;
use tempfile::tempdir;
use uuid::Uuid;

fn create_test_paper(title: &str, authors: &str, year: i64, abstract_text: &str) -> Paper {
    let id = Uuid::now_v7();
    Paper {
        id,
        file_path: format!("/papers/{id}.pdf"),
        content_hash: format!("hash_{id}"),
        title: Some(title.to_string()),
        authors: Some(authors.to_string()),
        year: Some(year),
        journal: Some("Journal of Machine Learning Research".to_string()),
        doi: Some(format!("10.5555/{id}")),
        abstract_text: Some(abstract_text.to_string()),
        text_path: None,
        annotated_pdf_path: None,
        toc_embedded_at: None,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    }
}

/// RT-20: Index creation in RAM and on filesystem directory.
/// - Verifies RAM index creation and initial query execution on empty index.
/// - Verifies filesystem index creation in a fresh directory (creates meta.json).
/// - Verifies persistence: indexing a document, dropping/re-opening the filesystem index,
///   and searching for the persisted document.
#[test]
fn test_rt_20_index_creation_ram_and_fs() {
    // 1. RAM index creation
    let ram_index = SearchIndex::create_in_ram().expect("RAM index creation must succeed");
    let empty_results = ram_index
        .search("anything", 10)
        .expect("Search on empty RAM index must succeed");
    assert!(
        empty_results.is_empty(),
        "Empty RAM index must return 0 results"
    );

    // 2. Filesystem index creation
    let dir = tempdir().expect("create temp dir");
    let index_dir = dir.path().join("search_index");

    // Initially directory does not exist
    assert!(!index_dir.exists());

    let fs_index =
        SearchIndex::open_or_create(&index_dir).expect("Filesystem index creation must succeed");
    assert!(index_dir.exists(), "Index directory must be created");
    assert!(
        index_dir.join("meta.json").exists(),
        "meta.json must be created by Tantivy"
    );

    let paper = create_test_paper(
        "Filesystem Persistence Test Paper",
        "Alice Smith",
        2024,
        "Testing persistent Tantivy indices across process or struct re-creation.",
    );
    let paper_id = paper.id;
    fs_index
        .index_paper(&paper, Some("Detailed body text about filesystem storage."))
        .expect("indexing in fs index must succeed");

    let initial_results = fs_index
        .search("Persistence", 10)
        .expect("search must succeed");
    assert_eq!(initial_results.len(), 1);
    assert_eq!(initial_results[0].id, paper_id);

    // 3. Re-opening existing filesystem index
    drop(fs_index);

    let reopened_index = SearchIndex::open_or_create(&index_dir)
        .expect("Re-opening existing filesystem index must succeed");
    let persisted_results = reopened_index
        .search("Persistence", 10)
        .expect("search on reopened index must succeed");
    assert_eq!(
        persisted_results.len(),
        1,
        "Reopened index must retain previously indexed documents"
    );
    assert_eq!(persisted_results[0].id, paper_id);
    assert_eq!(
        persisted_results[0].title,
        "Filesystem Persistence Test Paper"
    );
}

/// RT-21: Indexing papers with metadata and body text, searching by title, author, and body keywords.
/// - Indexes multiple papers with varying titles, authors, years, abstracts, and body texts.
/// - Verifies multi-field search: queries matching title, author, abstract, and body keywords.
#[test]
fn test_rt_21_indexing_and_multifield_search() {
    let index = SearchIndex::create_in_ram().expect("create RAM index");

    let p1 = create_test_paper(
        "Attention Is All You Need",
        "Ashish Vaswani, Noam Shazeer, Niki Parmar",
        2017,
        "The dominant sequence transduction models are based on complex recurrent or convolutional neural networks.",
    );
    let b1 = "We propose the Transformer, a model architecture eschewing recurrence and instead relying entirely on an attention mechanism.";

    let p2 = create_test_paper(
        "Deep Residual Learning for Image Recognition",
        "Kaiming He, Xiangyu Zhang, Shaoqing Ren, Jian Sun",
        2016,
        "Deeper neural networks are more difficult to train. We present a residual learning framework to ease the training.",
    );
    let b2 = "Our residual networks are easier to optimize and gain accuracy from considerably increased depth on ImageNet.";

    let p3 = create_test_paper(
        "Language Models are Few-Shot Learners",
        "Tom Brown, Benjamin Mann, Nick Ryder, Melanie Subbiah",
        2020,
        "Recent work has demonstrated substantial gains on many NLP tasks and benchmarks via pre-training followed by fine-tuning.",
    );
    let b3 = "Here we show that scaling up language models greatly improves task-agnostic, few-shot performance. We introduce GPT-3.";

    index.index_paper(&p1, Some(b1)).expect("index p1");
    index.index_paper(&p2, Some(b2)).expect("index p2");
    index.index_paper(&p3, Some(b3)).expect("index p3");

    // 1. Search by title keywords
    let title_res_1 = index.search("Attention", 10).expect("search Attention");
    assert_eq!(title_res_1.len(), 1);
    assert_eq!(title_res_1[0].id, p1.id);

    let title_res_2 = index
        .search("Residual Learning", 10)
        .expect("search Residual Learning");
    assert_eq!(title_res_2.len(), 1);
    assert_eq!(title_res_2[0].id, p2.id);

    let title_res_3 = index.search("Few-Shot", 10).expect("search Few-Shot");
    assert_eq!(title_res_3.len(), 1);
    assert_eq!(title_res_3[0].id, p3.id);

    // 2. Search by author keywords
    let author_res_1 = index.search("Vaswani", 10).expect("search Vaswani");
    assert_eq!(author_res_1.len(), 1);
    assert_eq!(author_res_1[0].id, p1.id);

    let author_res_2 = index.search("Kaiming", 10).expect("search Kaiming");
    assert_eq!(author_res_2.len(), 1);
    assert_eq!(author_res_2[0].id, p2.id);

    let author_res_3 = index.search("Brown", 10).expect("search Brown");
    assert_eq!(author_res_3.len(), 1);
    assert_eq!(author_res_3[0].id, p3.id);

    // 3. Search by abstract keywords
    let abstract_res_1 = index
        .search("transduction", 10)
        .expect("search transduction");
    assert_eq!(abstract_res_1.len(), 1);
    assert_eq!(abstract_res_1[0].id, p1.id);

    let abstract_res_3 = index.search("benchmarks", 10).expect("search benchmarks");
    assert_eq!(abstract_res_3.len(), 1);
    assert_eq!(abstract_res_3[0].id, p3.id);

    // 4. Search by body keywords
    let body_res_1 = index.search("Transformer", 10).expect("search Transformer");
    assert_eq!(body_res_1.len(), 1);
    assert_eq!(body_res_1[0].id, p1.id);

    let body_res_2 = index.search("ImageNet", 10).expect("search ImageNet");
    assert_eq!(body_res_2.len(), 1);
    assert_eq!(body_res_2[0].id, p2.id);

    let body_res_3 = index.search("GPT-3", 10).expect("search GPT-3");
    assert_eq!(body_res_3.len(), 1);
    assert_eq!(body_res_3[0].id, p3.id);
}

/// RT-22: Relevance ranking and snippet generation.
/// - Verifies that documents with matches in boosted fields (title) score higher than documents
///   with matches only in the body.
/// - Verifies that Tantivy snippet generator produces snippets highlighting matched keywords with `<b>...</b>`.
#[test]
fn test_rt_22_relevance_ranking_and_snippet_generation() {
    let index = SearchIndex::create_in_ram().expect("create RAM index");

    // Paper A: Has "Quantum" in Title (boosted 3.0x)
    let paper_a = create_test_paper(
        "Quantum Supremacy Using a Programmable Superconducting Processor",
        "Google Quantum AI",
        2019,
        "Demonstrating computational advantage over classical algorithms.",
    );
    let body_a = "The promise of quantum computers is that certain computational tasks might be executed exponentially faster.";

    // Paper B: Has "Quantum" mentioned only in body, but title is about thermodynamics
    let paper_b = create_test_paper(
        "Thermodynamics in Small Non-Equilibrium Systems",
        "Alice Physicist",
        2018,
        "A study of non-equilibrium fluctuations in microscopic regimes.",
    );
    let body_b = "Microscopic thermodynamics can be generalized to isolated systems experiencing quantum fluctuations.";

    index
        .index_paper(&paper_a, Some(body_a))
        .expect("index paper_a");
    index
        .index_paper(&paper_b, Some(body_b))
        .expect("index paper_b");

    let results = index.search("Quantum", 10).expect("search Quantum");
    assert_eq!(results.len(), 2);

    // Paper A has Quantum in Title + Body, Paper B only in Body.
    // Therefore Paper A must be ranked #1 with a higher score than Paper B.
    assert_eq!(
        results[0].id, paper_a.id,
        "Paper with title match must rank ahead of body-only match"
    );
    assert_eq!(results[1].id, paper_b.id);
    assert!(
        results[0].score > results[1].score,
        "Title-boosted match score ({}) must exceed body-only match score ({})",
        results[0].score,
        results[1].score
    );

    // Snippet verification:
    // Both results must generate a snippet with highlighted match
    assert!(
        results[0].snippet.is_some(),
        "Paper A must contain a snippet"
    );
    let snip_a = results[0].snippet.as_ref().unwrap();
    assert!(
        snip_a.contains("<b>") && snip_a.contains("</b>"),
        "Snippet must contain Tantivy HTML highlight tags: {snip_a}"
    );
    assert!(
        snip_a.to_lowercase().contains("quantum"),
        "Snippet must contain the matched term"
    );

    assert!(
        results[1].snippet.is_some(),
        "Paper B must contain a snippet"
    );
    let snip_b = results[1].snippet.as_ref().unwrap();
    assert!(
        snip_b.contains("<b>") && snip_b.contains("</b>"),
        "Snippet must contain Tantivy HTML highlight tags: {snip_b}"
    );
    assert!(
        snip_b.to_lowercase().contains("quantum"),
        "Snippet must contain the matched term"
    );
}

/// RT-23: Removing paper from index (`remove_paper`), verifying search no longer finds it.
/// - Verifies document removal by UUID term.
/// - Verifies that other indexed papers remain searchable.
/// - Verifies that re-indexing the paper succeeds and restores its searchability.
#[test]
fn test_rt_23_remove_paper_from_index() {
    let index = SearchIndex::create_in_ram().expect("create RAM index");

    let p1 = create_test_paper(
        "Convolutional Neural Networks for Sentence Classification",
        "Yoon Kim",
        2014,
        "We report on a series of experiments with convolutional neural networks trained on top of pre-trained word vectors.",
    );
    let b1 = "Sentence modeling using CNN layers and max pooling.";

    let p2 = create_test_paper(
        "Efficient Estimation of Word Representations in Vector Space",
        "Tomas Mikolov, Kai Chen, Greg Corrado, Jeffrey Dean",
        2013,
        "We propose two novel model architectures for computing continuous vector representations of words.",
    );
    let b2 = "Word2Vec skip-gram and CBOW models.";

    index.index_paper(&p1, Some(b1)).expect("index p1");
    index.index_paper(&p2, Some(b2)).expect("index p2");

    // Pre-condition: both papers are found
    assert_eq!(index.search("Convolutional", 10).unwrap().len(), 1);
    assert_eq!(index.search("Mikolov", 10).unwrap().len(), 1);

    // Remove p1
    index.remove_paper(p1.id).expect("remove p1");

    // p1 must not be found anymore
    let after_removal = index
        .search("Convolutional", 10)
        .expect("search Convolutional");
    assert!(
        after_removal.is_empty(),
        "Removed paper p1 must not be returned by search"
    );

    // p2 remains searchable
    let p2_search = index.search("Mikolov", 10).expect("search Mikolov");
    assert_eq!(p2_search.len(), 1);
    assert_eq!(p2_search[0].id, p2.id);

    // Re-indexing p1 restores it
    index.index_paper(&p1, Some(b1)).expect("re-index p1");
    let restored = index
        .search("Convolutional", 10)
        .expect("search Convolutional");
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].id, p1.id);
}

/// RT-24: Syntax error tolerance.
/// Verifies graceful handling without panic or error for:
/// - unclosed quotes (e.g. `"attention is`)
/// - stray colons not corresponding to valid fields (e.g. `doi:10.1234`)
/// - plus and minus operators (e.g. `+`, `-`, `++--++`, `attention +`)
/// - empty queries (e.g. `""`, `"   "`)
/// - unclosed parentheses and brackets (e.g. `(attention`, `[transformer]`)
/// - special punctuation noise (e.g. `!@#$%^&*()_+`)
#[test]
fn test_rt_24_syntax_error_tolerance() {
    let index = SearchIndex::create_in_ram().expect("create RAM index");

    let p1 = create_test_paper(
        "Attention Is All You Need",
        "Ashish Vaswani",
        2017,
        "Sequence transduction models using self-attention mechanisms.",
    );
    let b1 = "Transformer architecture for translation and language modeling.";
    index.index_paper(&p1, Some(b1)).expect("index p1");

    // 1. Unclosed quote: `"attention is`
    let res_quote = index.search("\"attention is", 10);
    assert!(
        res_quote.is_ok(),
        "Unclosed quote must be handled gracefully: {:?}",
        res_quote.err()
    );
    let papers = res_quote.unwrap();
    assert!(
        !papers.is_empty(),
        "Unclosed quote query must still find matching paper"
    );
    assert_eq!(papers[0].id, p1.id);

    // 2. Stray colon with nonexistent field: `doi:10.1234`
    let res_colon = index.search("doi:10.1234", 10);
    assert!(
        res_colon.is_ok(),
        "Stray colon must not cause query parser error: {:?}",
        res_colon.err()
    );

    // 3. Plus and minus symbols
    let res_plus = index.search("+", 10);
    assert!(res_plus.is_ok(), "Lone '+' must not fail or panic");
    assert!(res_plus.unwrap().is_empty());

    let res_minus = index.search("-", 10);
    assert!(res_minus.is_ok(), "Lone '-' must not fail or panic");
    assert!(res_minus.unwrap().is_empty());

    let res_plus_minus = index.search("++--++", 10);
    assert!(res_plus_minus.is_ok(), "'++--++' must not fail or panic");
    assert!(res_plus_minus.unwrap().is_empty());

    // 4. Dangling operators with words: `attention +`
    let res_dangling = index.search("attention +", 10);
    assert!(
        res_dangling.is_ok(),
        "Dangling operator 'attention +' must be handled gracefully: {:?}",
        res_dangling.err()
    );
    let dangling_papers = res_dangling.unwrap();
    assert!(
        !dangling_papers.is_empty(),
        "'attention +' must still find paper containing 'attention'"
    );

    // 5. Empty and whitespace queries
    assert!(index.search("", 10).unwrap().is_empty());
    assert!(index.search("   ", 10).unwrap().is_empty());
    assert!(index.search("\t\n\r", 10).unwrap().is_empty());

    // 6. Unclosed parentheses and brackets
    let res_paren = index.search("(attention", 10);
    assert!(
        res_paren.is_ok(),
        "Unclosed paren '(attention' must be handled gracefully"
    );
    assert!(!res_paren.unwrap().is_empty());

    let res_bracket = index.search("[transformer]", 10);
    assert!(
        res_bracket.is_ok(),
        "Brackets '[transformer]' must be handled gracefully"
    );
    assert!(!res_bracket.unwrap().is_empty());

    // 7. Punctuation noise
    let res_noise = index.search("::: \"\"\" +++ --- !!! ((( [[]]", 10);
    assert!(
        res_noise.is_ok(),
        "Punctuation noise must not cause errors or panic"
    );
}
