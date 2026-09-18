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
    /// Database identifier of the collection, or None for virtual "All Papers" collection.
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
    /// Creates a new root collection item.
    pub fn new(id: Option<Uuid>, name: impl Into<String>, paper_count: usize) -> Self {
        Self::with_hierarchy(id, name, paper_count, 0, None)
    }

    /// Creates a collection item with explicit tree hierarchy depth and parent ID.
    pub fn with_hierarchy(
        id: Option<Uuid>,
        name: impl Into<String>,
        paper_count: usize,
        depth: usize,
        parent_id: Option<Uuid>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            paper_count,
            depth,
            parent_id,
        }
    }
}
