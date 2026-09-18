use std::cmp::Ordering;

use papyrus_core::db::Paper;

use crate::app::App;

/// Field used for sorting the papers list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaperSortField {
    #[default]
    Added,
    Year,
    Title,
    Author,
}

/// Sort direction (ascending or descending).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    Asc,
    #[default]
    Desc,
}

impl PaperSortField {
    /// Returns the display badge string for the given field and direction.
    pub fn badge(&self, direction: SortDirection) -> &'static str {
        match (self, direction) {
            (Self::Added, SortDirection::Desc) => "Added↓",
            (Self::Added, SortDirection::Asc) => "Added↑",
            (Self::Year, SortDirection::Desc) => "Year↓",
            (Self::Year, SortDirection::Asc) => "Year↑",
            (Self::Title, SortDirection::Asc) => "Title↑",
            (Self::Title, SortDirection::Desc) => "Title↓",
            (Self::Author, SortDirection::Asc) => "Author↑",
            (Self::Author, SortDirection::Desc) => "Author↓",
        }
    }

    /// Advances to the next sort field and default direction in the cycle:
    /// Added (Desc) -> Year (Desc) -> Title (Asc) -> Author (Asc) -> Added (Desc)
    pub fn next_sort(self, _current_direction: SortDirection) -> (Self, SortDirection) {
        match self {
            Self::Added => (Self::Year, SortDirection::Desc),
            Self::Year => (Self::Title, SortDirection::Asc),
            Self::Title => (Self::Author, SortDirection::Asc),
            Self::Author => (Self::Added, SortDirection::Desc),
        }
    }
}

/// Sorts a slice of `Paper` structs in-place according to the given field and direction.
/// Tie-breaks by `paper.id` for total deterministic ordering.
pub fn sort_papers(papers: &mut [Paper], field: PaperSortField, direction: SortDirection) {
    papers.sort_by(|a, b| {
        let primary = match field {
            PaperSortField::Added => {
                let cmp = a.created_at.cmp(&b.created_at);
                if direction == SortDirection::Asc {
                    cmp
                } else {
                    cmp.reverse()
                }
            }
            PaperSortField::Year => match (a.year, b.year) {
                (Some(ya), Some(yb)) => {
                    let cmp = ya.cmp(&yb);
                    if direction == SortDirection::Asc {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                }
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            },
            PaperSortField::Title => {
                let ta = a.title.as_deref().unwrap_or("");
                let tb = b.title.as_deref().unwrap_or("");
                let cmp = ta
                    .chars()
                    .flat_map(char::to_lowercase)
                    .cmp(tb.chars().flat_map(char::to_lowercase));
                if direction == SortDirection::Asc {
                    cmp
                } else {
                    cmp.reverse()
                }
            }
            PaperSortField::Author => {
                let aa = a.authors.as_deref().unwrap_or("");
                let ab = b.authors.as_deref().unwrap_or("");
                let cmp = aa
                    .chars()
                    .flat_map(char::to_lowercase)
                    .cmp(ab.chars().flat_map(char::to_lowercase));
                if direction == SortDirection::Asc {
                    cmp
                } else {
                    cmp.reverse()
                }
            }
        };

        if primary == Ordering::Equal {
            a.id.cmp(&b.id)
        } else {
            primary
        }
    });
}

impl App {
    /// Advances to the next sort field / direction and sorts the current papers list.
    pub fn cycle_paper_sort(&mut self) {
        let (next_field, next_dir) = self.sort_field.next_sort(self.sort_direction);
        self.sort_field = next_field;
        self.sort_direction = next_dir;
        self.sort_current_papers();
    }

    /// Sorts the loaded `papers` list according to `self.sort_field` and `self.sort_direction`,
    /// and restores selection by UUID.
    pub fn sort_current_papers(&mut self) {
        let col_id = self.current_collection().and_then(|col| col.id);
        if let Some(list) = self.papers_by_collection.get_mut(&col_id) {
            sort_papers(list, self.sort_field, self.sort_direction);
        }
        sort_papers(&mut self.papers, self.sort_field, self.sort_direction);
        if self.papers.is_empty() {
            self.selected_paper = 0;
            self.selection.paper_id = None;
        } else if let Some(pid) = self.selection.paper_id {
            if let Some(pos) = self.papers.iter().position(|p| p.id == pid) {
                self.selected_paper = pos;
            } else {
                self.selected_paper = self.selected_paper.min(self.papers.len() - 1);
            }
        } else {
            self.selected_paper = self.selected_paper.min(self.papers.len() - 1);
        }
        self.update_selection_from_indices();
    }
}
