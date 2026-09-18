use uuid::Uuid;

/// Identity key for collections in the TUI (real DB collections and virtual collections).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CollectionKey {
    /// Virtual collection displaying all papers.
    All,
    /// Real SQLite database collection identified by its UUID.
    Real(Uuid),
    /// Virtual collection displaying recently added papers.
    RecentlyAdded,
    /// Virtual collection displaying papers not in any collection.
    Unfiled,
    /// Virtual collection displaying papers without any tags.
    Untagged,
}

impl CollectionKey {
    /// Returns the database UUID if this represents a real collection.
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            Self::Real(id) => Some(*id),
            _ => None,
        }
    }
}

/// Authoritative selection state in the TUI (Single Source of Truth).
///
/// List indices (`selected_collection`, `selected_paper`, `selected_toc`)
/// are derived UI caches for rendering and must be recomputed from this state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionState {
    /// Key of the currently selected collection.
    pub collection: CollectionKey,
    /// UUID of the currently selected paper, or `None` if the list is empty.
    pub paper_id: Option<Uuid>,
    /// UUID of the currently selected TOC entry, or `None` if no TOC entry is selected.
    pub toc_id: Option<Uuid>,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            collection: CollectionKey::All,
            paper_id: None,
            toc_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_key_as_uuid() {
        assert_eq!(CollectionKey::All.as_uuid(), None);
        assert_eq!(CollectionKey::RecentlyAdded.as_uuid(), None);
        assert_eq!(CollectionKey::Unfiled.as_uuid(), None);
        assert_eq!(CollectionKey::Untagged.as_uuid(), None);

        let id = Uuid::new_v4();
        assert_eq!(CollectionKey::Real(id).as_uuid(), Some(id));
    }

    #[test]
    fn test_selection_state_default() {
        let state = SelectionState::default();
        assert_eq!(state.collection, CollectionKey::All);
        assert_eq!(state.paper_id, None);
        assert_eq!(state.toc_id, None);
    }

    #[test]
    fn test_collection_item_keys() {
        use crate::app::CollectionItem;

        let all_item = CollectionItem::new(None, "All Papers", 10);
        assert_eq!(all_item.key, CollectionKey::All);
        assert_eq!(all_item.id, None);

        let uid = Uuid::new_v4();
        let real_item = CollectionItem::new(Some(uid), "Real Col", 5);
        assert_eq!(real_item.key, CollectionKey::Real(uid));
        assert_eq!(real_item.id, Some(uid));

        let unfiled_item = CollectionItem::from_key(CollectionKey::Unfiled, "Unfiled", 2);
        assert_eq!(unfiled_item.key, CollectionKey::Unfiled);
        assert_eq!(unfiled_item.id, None);
    }
}
