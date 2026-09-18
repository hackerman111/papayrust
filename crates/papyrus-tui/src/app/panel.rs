use crate::app::selection::CollectionKey;
use uuid::Uuid;

/// Active focus panel in the 3-panel TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePanel {
    #[default]
    Collections,
    Papers,
    Details,
}

impl ActivePanel {
    /// Cycles to the next panel: Collections -> Papers -> Details -> Collections.
    pub fn next(self) -> Self {
        match self {
            Self::Collections => Self::Papers,
            Self::Papers => Self::Details,
            Self::Details => Self::Collections,
        }
    }

    /// Cycles to the previous panel: Details -> Papers -> Collections -> Details.
    pub fn previous(self) -> Self {
        match self {
            Self::Collections => Self::Details,
            Self::Papers => Self::Collections,
            Self::Details => Self::Papers,
        }
    }

    /// Moves focus to the left panel (clamped at Collections).
    pub fn left(self) -> Self {
        match self {
            Self::Collections => Self::Collections,
            Self::Papers => Self::Collections,
            Self::Details => Self::Papers,
        }
    }

    /// Moves focus to the right panel (clamped at Details).
    pub fn right(self) -> Self {
        match self {
            Self::Collections => Self::Papers,
            Self::Papers => Self::Details,
            Self::Details => Self::Details,
        }
    }

    /// Name of the panel for display or debugging.
    pub fn name(self) -> &'static str {
        match self {
            Self::Collections => "Collections",
            Self::Papers => "Papers",
            Self::Details => "Details",
        }
    }
}

/// Item representing a collection in the Collections panel list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionItem {
    /// Authoritative collection key (real DB collection or virtual).
    pub key: CollectionKey,
    /// Database identifier of the collection, or None for virtual collections.
    pub id: Option<Uuid>,
    /// Display name of the collection.
    pub name: String,
    /// Number of papers in this collection.
    pub paper_count: usize,
    /// Nesting depth for tree hierarchy display (0 for root/virtual).
    pub depth: usize,
    /// Optional parent collection ID.
    pub parent_id: Option<Uuid>,
}

impl CollectionItem {
    /// Creates a new root collection item from an optional UUID.
    pub fn new(id: Option<Uuid>, name: impl Into<String>, paper_count: usize) -> Self {
        let key = match id {
            Some(uuid) => CollectionKey::Real(uuid),
            None => CollectionKey::All,
        };
        Self::with_key_and_hierarchy(key, name, paper_count, 0, None)
    }

    /// Creates a collection item with explicit tree hierarchy depth and parent ID.
    pub fn with_hierarchy(
        id: Option<Uuid>,
        name: impl Into<String>,
        paper_count: usize,
        depth: usize,
        parent_id: Option<Uuid>,
    ) -> Self {
        let key = match id {
            Some(uuid) => CollectionKey::Real(uuid),
            None => CollectionKey::All,
        };
        Self::with_key_and_hierarchy(key, name, paper_count, depth, parent_id)
    }

    /// Creates a collection item from an explicit CollectionKey.
    pub fn from_key(key: CollectionKey, name: impl Into<String>, paper_count: usize) -> Self {
        Self::with_key_and_hierarchy(key, name, paper_count, 0, None)
    }

    /// Creates a collection item with explicit CollectionKey and tree hierarchy depth.
    pub fn with_key_and_hierarchy(
        key: CollectionKey,
        name: impl Into<String>,
        paper_count: usize,
        depth: usize,
        parent_id: Option<Uuid>,
    ) -> Self {
        let id = key.as_uuid();
        Self {
            key,
            id,
            name: name.into(),
            paper_count,
            depth,
            parent_id,
        }
    }
}
