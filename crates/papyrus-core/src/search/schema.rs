use tantivy::schema::{
    Field, IndexRecordOption, NumericOptions, Schema, SchemaBuilder, TextFieldIndexing,
    TextOptions, FAST, STORED, STRING, TEXT,
};

pub const TOKENIZER_WITH_STOPWORDS: &str = "papyrus_stopwords";

/// Field handles for the Papyrus search schema.
#[derive(Debug, Clone, Copy)]
pub struct Fields {
    pub id: Field,
    pub title: Field,
    pub authors: Field,
    pub year: Field,
    pub abstract_text: Field,
    pub body: Field,
}

/// Builds the Tantivy schema and returns the schema along with its field handles.
pub fn build_schema() -> (Schema, Fields) {
    let mut builder = SchemaBuilder::new();

    // id: raw string for exact match and deletion, stored
    let id = builder.add_text_field("id", STRING | STORED);

    // title: full-text, stored, fast
    let title = builder.add_text_field("title", TEXT | STORED | FAST);

    // authors: full-text, stored
    let authors = builder.add_text_field("authors", TEXT | STORED);

    // year: 64-bit integer, indexed, stored, fast
    let year_options = NumericOptions::default()
        .set_indexed()
        .set_stored()
        .set_fast();
    let year = builder.add_i64_field("year", year_options);

    // abstract_text: full-text, stored
    let abstract_text = builder.add_text_field("abstract_text", TEXT | STORED);

    // body: full-text, stored (snippet-capable, indexed with tokenizer & stopwords)
    let body_indexing = TextFieldIndexing::default()
        .set_tokenizer(TOKENIZER_WITH_STOPWORDS)
        .set_index_option(IndexRecordOption::WithFreqsAndPositions);
    let body_options = TextOptions::default()
        .set_indexing_options(body_indexing)
        .set_stored();
    let body = builder.add_text_field("body", body_options);

    let fields = Fields {
        id,
        title,
        authors,
        year,
        abstract_text,
        body,
    };

    (builder.build(), fields)
}
