use std::path::Path;
use std::sync::Mutex;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{IndexRecordOption, Term, Value};
use tantivy::snippet::SnippetGenerator;
use tantivy::tokenizer::{Language, LowerCaser, SimpleTokenizer, StopWordFilter, TextAnalyzer};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};
use uuid::Uuid;

use crate::db::Paper;
use crate::search::error::SearchError;
use crate::search::schema::{build_schema, Fields, TOKENIZER_WITH_STOPWORDS};

/// Registers custom tokenizers with stopwords on the Tantivy index.
pub fn register_tokenizers(index: &Index) {
    let stop_words =
        StopWordFilter::new(Language::English).expect("English stopwords must be supported");
    let analyzer = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(LowerCaser)
        .filter(stop_words)
        .build();
    index
        .tokenizers()
        .register(TOKENIZER_WITH_STOPWORDS, analyzer);
}

/// A search result returned by the full-text search index.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    pub id: Uuid,
    pub score: f32,
    pub title: String,
    pub authors: Option<String>,
    pub year: Option<i64>,
    pub snippet: Option<String>,
}

/// Thread-safe full-text search index wrapping Tantivy.
pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    writer: Mutex<IndexWriter>,
    fields: Fields,
}

impl SearchIndex {
    /// Creates a new in-memory search index.
    pub fn create_in_ram() -> Result<Self, SearchError> {
        let (schema, fields) = build_schema();
        let index = Index::create_in_ram(schema);
        register_tokenizers(&index);

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let writer = index.writer(50_000_000)?;

        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
        })
    }

    /// Opens an existing search index in the specified directory, or creates a new one if it does not exist.
    pub fn open_or_create(dir: &Path) -> Result<Self, SearchError> {
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }

        let (schema, fields) = build_schema();
        let index = if dir.join("meta.json").exists() {
            Index::open_in_dir(dir)?
        } else {
            Index::create_in_dir(dir, schema)?
        };

        register_tokenizers(&index);

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        let writer = index.writer(50_000_000)?;

        Ok(Self {
            index,
            reader,
            writer: Mutex::new(writer),
            fields,
        })
    }

    /// Returns the field handles of the search schema.
    pub fn fields(&self) -> &Fields {
        &self.fields
    }

    /// Indexes or updates a paper in the search index with optional extracted body text.
    pub fn index_paper(&self, paper: &Paper, body_text: Option<&str>) -> Result<(), SearchError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| SearchError::Lock(e.to_string()))?;

        // Ensure idempotency by deleting any existing document for this paper id
        let id_term = Term::from_field_text(self.fields.id, &paper.id.to_string());
        writer.delete_term(id_term);

        let mut doc = TantivyDocument::default();
        doc.add_text(self.fields.id, paper.id.to_string());

        if let Some(ref title) = paper.title {
            doc.add_text(self.fields.title, title);
        }
        if let Some(ref authors) = paper.authors {
            doc.add_text(self.fields.authors, authors);
        }
        if let Some(year) = paper.year {
            doc.add_i64(self.fields.year, year);
        }
        if let Some(ref abstract_text) = paper.abstract_text {
            doc.add_text(self.fields.abstract_text, abstract_text);
        }
        if let Some(body) = body_text {
            doc.add_text(self.fields.body, body);
        }

        writer.add_document(doc)?;
        writer.commit()?;
        self.reader.reload()?;

        Ok(())
    }

    /// Removes a paper from the search index by its unique identifier.
    pub fn remove_paper(&self, paper_id: Uuid) -> Result<(), SearchError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| SearchError::Lock(e.to_string()))?;

        let id_term = Term::from_field_text(self.fields.id, &paper_id.to_string());
        writer.delete_term(id_term);
        writer.commit()?;
        self.reader.reload()?;

        Ok(())
    }

    /// Performs full-text search with fault-tolerant query parsing and snippet generation.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let trimmed = query_str.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let searcher = self.reader.searcher();
        let query = self.parse_query_tolerant(trimmed)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut body_snippet_gen =
            SnippetGenerator::create(&searcher, &*query, self.fields.body).ok();
        if let Some(ref mut gen) = body_snippet_gen {
            gen.set_max_num_chars(150);
        }

        let mut abstract_snippet_gen =
            SnippetGenerator::create(&searcher, &*query, self.fields.abstract_text).ok();
        if let Some(ref mut gen) = abstract_snippet_gen {
            gen.set_max_num_chars(150);
        }

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            let id_str = doc
                .get_first(self.fields.id)
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let id = Uuid::parse_str(id_str).unwrap_or_default();

            let title = doc
                .get_first(self.fields.title)
                .and_then(|v| v.as_str())
                .unwrap_or("[Untitled]")
                .to_string();

            let authors = doc
                .get_first(self.fields.authors)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let year = doc.get_first(self.fields.year).and_then(|v| v.as_i64());

            // Generate snippet highlighting matched terms
            let mut snippet = None;
            if let Some(ref gen) = body_snippet_gen {
                let s = gen.snippet_from_doc(&doc);
                let html = s.to_html();
                if !html.is_empty() && html.contains("<b>") {
                    snippet = Some(html);
                }
            }
            if snippet.is_none() {
                if let Some(ref gen) = abstract_snippet_gen {
                    let s = gen.snippet_from_doc(&doc);
                    let html = s.to_html();
                    if !html.is_empty() {
                        snippet = Some(html);
                    }
                }
            }

            results.push(SearchResult {
                id,
                score,
                title,
                authors,
                year,
                snippet,
            });
        }

        Ok(results)
    }

    /// Parses a user query string with fallback strategies to ensure resilience against syntax errors.
    fn parse_query_tolerant(&self, query_str: &str) -> Result<Box<dyn Query>, SearchError> {
        let mut query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.fields.title,
                self.fields.authors,
                self.fields.abstract_text,
                self.fields.body,
            ],
        );

        // Boost title and authors relative to body
        query_parser.set_field_boost(self.fields.title, 3.0);
        query_parser.set_field_boost(self.fields.authors, 2.0);
        query_parser.set_field_boost(self.fields.abstract_text, 1.5);
        query_parser.set_field_boost(self.fields.body, 1.0);

        // Strategy 1: Attempt direct standard parsing
        if let Ok(query) = query_parser.parse_query(query_str) {
            return Ok(query);
        }

        // Strategy 2: Sanitize query string (strip syntax characters like unmatched quotes, colons, lone operators)
        let sanitized: String = query_str
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() {
                    c
                } else {
                    ' '
                }
            })
            .collect();
        let sanitized_trimmed = sanitized.trim();

        if !sanitized_trimmed.is_empty() {
            if let Ok(query) = query_parser.parse_query(sanitized_trimmed) {
                return Ok(query);
            }
        }

        // Strategy 3: Construct BooleanQuery with TermQuery per word across all text fields
        let words: Vec<&str> = sanitized_trimmed
            .split_whitespace()
            .filter(|w| !w.is_empty())
            .collect();

        if words.is_empty() {
            // Nothing searchable left; return a match-none query safely
            return Ok(Box::new(BooleanQuery::new(Vec::new())));
        }

        let mut word_clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
        for word in words {
            let lower_word = word.to_lowercase();
            let mut field_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

            for field in [
                self.fields.title,
                self.fields.authors,
                self.fields.abstract_text,
                self.fields.body,
            ] {
                let term = Term::from_field_text(field, &lower_word);
                let tq = TermQuery::new(term, IndexRecordOption::WithFreqsAndPositions);
                field_queries.push((Occur::Should, Box::new(tq)));
            }

            let word_query = BooleanQuery::new(field_queries);
            word_clauses.push((Occur::Should, Box::new(word_query)));
        }

        Ok(Box::new(BooleanQuery::new(word_clauses)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_paper(id: Uuid, title: &str, authors: &str, year: i64, abstract_text: &str) -> Paper {
        Paper {
            id,
            file_path: format!("/papers/{id}.pdf"),
            content_hash: format!("hash_{id}"),
            title: Some(title.to_string()),
            authors: Some(authors.to_string()),
            year: Some(year),
            journal: Some("Test Journal".to_string()),
            doi: Some("10.1234/test".to_string()),
            abstract_text: Some(abstract_text.to_string()),
            text_path: None,
            annotated_pdf_path: None,
            toc_embedded_at: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_search_index_ram_and_search() {
        let index = SearchIndex::create_in_ram().expect("create in ram");
        let id1 = Uuid::now_v7();
        let p1 = test_paper(
            id1,
            "Attention Is All You Need",
            "Vaswani et al.",
            2017,
            "The dominant sequence transduction models are based on complex recurrent or convolutional neural networks.",
        );
        let body1 = "We propose the Transformer, a model architecture eschewing recurrence and instead relying entirely on an attention mechanism to draw global dependencies between input and output.";

        index.index_paper(&p1, Some(body1)).expect("index paper");

        let results = index.search("Transformer", 10).expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id1);
        assert_eq!(results[0].title, "Attention Is All You Need");
        assert!(results[0].snippet.is_some());
        let snip = results[0].snippet.as_ref().unwrap();
        assert!(snip.contains("<b>Transformer</b>") || snip.contains("Transformer"));
    }
}
