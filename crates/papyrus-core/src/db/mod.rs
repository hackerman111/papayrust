//! Database connection, schema migrations, models, and repositories.

pub mod collection_repo;
pub mod connection;
pub mod error;
pub mod migration;
pub mod models;
pub mod paper_repo;
pub mod tag_repo;
pub mod toc_repo;

pub use collection_repo::CollectionRepo;
pub use connection::{configure_connection, open_database, open_in_memory, DbError};
pub use error::RepoError;
pub use migration::{apply_migrations, current_schema_version};
pub use models::{Collection, Paper, TocEntry, TocSource};
pub use paper_repo::PaperRepo;
pub use tag_repo::TagRepo;
pub use toc_repo::TocRepo;
