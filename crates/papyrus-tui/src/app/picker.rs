use std::collections::HashSet;
use uuid::Uuid;

/// An item displayed in a generic picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerItem {
    pub id: Uuid,
    pub title: String,
    pub subtitle: Option<String>,
    pub category: Option<String>,
}

impl PickerItem {
    pub fn new(id: Uuid, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            subtitle: None,
            category: None,
        }
    }

    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }
}

/// Generic modal picker with fuzzy filtering, cursor navigation, and multi-selection support.
#[derive(Debug, Clone)]
pub struct GenericPicker {
    pub title: String,
    pub items: Vec<PickerItem>,
    pub visible_indices: Vec<usize>,
    pub selected: usize,
    pub query: String,
    pub checked: HashSet<Uuid>,
    pub multi_select: bool,
}

impl GenericPicker {
    pub fn new(title: impl Into<String>, items: Vec<PickerItem>, multi_select: bool) -> Self {
        let visible_indices = (0..items.len()).collect();
        Self {
            title: title.into(),
            items,
            visible_indices,
            selected: 0,
            query: String::new(),
            checked: HashSet::new(),
            multi_select,
        }
    }

    pub fn with_checked(mut self, checked: HashSet<Uuid>) -> Self {
        self.checked = checked;
        self
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.update_filter();
    }

    pub fn push_query_char(&mut self, c: char) {
        self.query.push(c);
        self.update_filter();
    }

    pub fn pop_query_char(&mut self) {
        self.query.pop();
        self.update_filter();
    }

    pub fn move_cursor(&mut self, delta: isize) {
        if self.visible_indices.is_empty() {
            self.selected = 0;
            return;
        }
        let max_idx = self.visible_indices.len().saturating_sub(1);
        let new_selected = (self.selected as isize + delta).clamp(0, max_idx as isize) as usize;
        self.selected = new_selected;
    }

    pub fn move_to_first(&mut self) {
        self.selected = 0;
        self.clamp_selected();
    }

    pub fn move_to_last(&mut self) {
        if !self.visible_indices.is_empty() {
            self.selected = self.visible_indices.len() - 1;
        } else {
            self.selected = 0;
        }
    }

    pub fn toggle_selected(&mut self) {
        if !self.multi_select {
            return;
        }
        if let Some(item) = self.selected_item() {
            let id = item.id;
            if self.checked.contains(&id) {
                self.checked.remove(&id);
            } else {
                self.checked.insert(id);
            }
        }
    }

    pub fn selected_item(&self) -> Option<&PickerItem> {
        self.visible_indices
            .get(self.selected)
            .and_then(|&idx| self.items.get(idx))
    }

    pub fn checked_items(&self) -> &HashSet<Uuid> {
        &self.checked
    }

    fn update_filter(&mut self) {
        let trimmed = self.query.trim();
        if trimmed.is_empty() {
            self.visible_indices = (0..self.items.len()).collect();
        } else {
            let mut scored: Vec<(usize, i64)> = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| score_item(item, trimmed).map(|score| (idx, score)))
                .collect();

            scored.sort_by(|(idx_a, score_a), (idx_b, score_b)| {
                score_b.cmp(score_a).then_with(|| idx_a.cmp(idx_b))
            });

            self.visible_indices = scored.into_iter().map(|(idx, _)| idx).collect();
        }
        self.clamp_selected();
    }

    fn clamp_selected(&mut self) {
        if self.visible_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.visible_indices.len() {
            self.selected = self.visible_indices.len() - 1;
        }
    }
}

#[inline]
fn char_eq_ci(a: char, b: char) -> bool {
    if a == b {
        return true;
    }
    if a.is_ascii() && b.is_ascii() {
        a.eq_ignore_ascii_case(&b)
    } else {
        a.to_lowercase().eq(b.to_lowercase())
    }
}

#[inline]
fn is_word_boundary(prev: Option<char>, curr: char) -> bool {
    match prev {
        None => true,
        Some(p) => {
            p.is_whitespace()
                || p.is_ascii_punctuation()
                || (p.is_lowercase() && curr.is_uppercase())
        }
    }
}

#[inline]
fn starts_with_ci(target: &str, prefix: &str) -> bool {
    let mut t_chars = target.chars();
    for p in prefix.chars() {
        match t_chars.next() {
            Some(t) if char_eq_ci(t, p) => {}
            _ => return false,
        }
    }
    true
}

struct SubstringMatch {
    char_idx: usize,
    is_word_boundary: bool,
}

fn best_exact_substring(target: &str, query: &str) -> Option<SubstringMatch> {
    let mut best: Option<SubstringMatch> = None;
    let mut prev_char: Option<char> = None;

    for (char_idx, (byte_idx, c)) in target.char_indices().enumerate() {
        let is_wb = is_word_boundary(prev_char, c);

        if starts_with_ci(&target[byte_idx..], query) {
            let m = SubstringMatch {
                char_idx,
                is_word_boundary: is_wb,
            };
            if m.char_idx == 0 {
                return Some(m);
            }
            if is_wb {
                if best.as_ref().is_none_or(|b| !b.is_word_boundary) {
                    best = Some(m);
                }
            } else if best.is_none() {
                best = Some(m);
            }
        }
        prev_char = Some(c);
    }
    best
}

fn score_subsequence(target: &str, query: &str) -> Option<i64> {
    let q_len = query.chars().count() as i64;
    let t_len = target.chars().count() as i64;

    if let Some(sub) = best_exact_substring(target, query) {
        let mut score: i64 = 5_000;
        if sub.char_idx == 0 {
            score += 5_000;
            if t_len == q_len {
                score += 10_000;
            }
        } else if sub.is_word_boundary {
            score += 3_000;
        } else {
            score += 1_000;
        }
        score += (q_len * 50).saturating_sub(t_len * 2);
        score -= (sub.char_idx as i64) * 5;
        return Some(score);
    }

    // Check if query is a subsequence of target
    let mut q_iter = query.chars().peekable();
    if q_iter.peek().is_none() {
        return Some(0);
    }

    let mut word_boundary_matches = 0usize;
    let mut consecutive_matches = 0usize;
    let mut first_match_idx: Option<usize> = None;
    let mut last_match_idx: Option<usize> = None;
    let mut prev_char: Option<char> = None;

    for (char_idx, (_byte_idx, c)) in target.char_indices().enumerate() {
        let is_wb = is_word_boundary(prev_char, c);

        if let Some(&qc) = q_iter.peek() {
            if char_eq_ci(c, qc) {
                q_iter.next();
                if is_wb {
                    word_boundary_matches += 1;
                }
                if let Some(last) = last_match_idx {
                    if char_idx == last + 1 {
                        consecutive_matches += 1;
                    }
                }
                if first_match_idx.is_none() {
                    first_match_idx = Some(char_idx);
                }
                last_match_idx = Some(char_idx);
            }
        }
        prev_char = Some(c);
    }

    if q_iter.peek().is_some() {
        return None;
    }

    let mut score: i64 = 1_000;
    score += q_len * 50;
    score += (word_boundary_matches as i64) * 400;
    score += (consecutive_matches as i64) * 100;
    if first_match_idx == Some(0) {
        score += 200;
    }
    score = score.saturating_sub(t_len * 2);
    if let Some(first) = first_match_idx {
        score = score.saturating_sub((first as i64) * 5);
    }

    Some(score)
}

fn score_item(item: &PickerItem, query: &str) -> Option<i64> {
    let title_score = score_subsequence(&item.title, query);
    let subtitle_score = item
        .subtitle
        .as_deref()
        .and_then(|s| score_subsequence(s, query));
    let category_score = item
        .category
        .as_deref()
        .and_then(|c| score_subsequence(c, query));

    if title_score.is_none() && subtitle_score.is_none() && category_score.is_none() {
        return None;
    }

    let mut total_score: i64 = 0;
    if let Some(s) = title_score {
        total_score += s + 3_000;
    }
    if let Some(s) = subtitle_score {
        total_score += s + 1_500;
    }
    if let Some(s) = category_score {
        total_score += s;
    }

    Some(total_score)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_items() -> Vec<PickerItem> {
        vec![
            PickerItem::new(Uuid::from_u128(1), "Machine Learning").with_category("Theory"),
            PickerItem::new(Uuid::from_u128(2), "Deep Learning")
                .with_subtitle("Foundations")
                .with_category("Machine Learning"),
            PickerItem::new(Uuid::from_u128(3), "Smach"),
            PickerItem::new(Uuid::from_u128(4), "Quantum Mechanics"),
        ]
    }

    #[test]
    fn test_picker_empty_query_returns_all() {
        let items = sample_items();
        let mut picker = GenericPicker::new("Select Topic", items.clone(), false);

        assert_eq!(picker.visible_indices, vec![0, 1, 2, 3]);
        assert_eq!(picker.selected, 0);
        assert_eq!(picker.selected_item(), Some(&items[0]));

        // Whitespace only query should also return all items in original order
        picker.set_query("   ");
        assert_eq!(picker.visible_indices, vec![0, 1, 2, 3]);
        assert_eq!(picker.selected, 0);
    }

    #[test]
    fn test_picker_fuzzy_matching_and_ranking() {
        let items = sample_items();
        let mut picker = GenericPicker::new("Search", items.clone(), false);

        // "mach":
        // Item 0: Title starts with "Mach" (exact prefix match -> highest score)
        // Item 1: Category starts with "Mach" (word boundary match in category)
        // Item 2: Title contains "mach" ("Smach", mid-word exact substring)
        // Item 3: "Quantum Mechanics" ("Mech" has 'e', not 'a', so not a match for "mach")
        picker.set_query("mach");
        assert_eq!(picker.visible_indices, vec![0, 1, 2]);
        assert_eq!(picker.selected_item().unwrap().title, "Machine Learning");

        // Push and pop char verification
        picker.set_query("ma");
        picker.push_query_char('c');
        picker.push_query_char('h');
        assert_eq!(picker.visible_indices, vec![0, 1, 2]);
        picker.pop_query_char(); // back to "mac"
        assert_eq!(picker.query, "mac");
        // All 4 match for "mac": prefix (0) > category prefix (1) > substring (2) > scattered subsequence (3)
        assert_eq!(picker.visible_indices, vec![0, 1, 2, 3]);

        // Acronym / word-initial subsequence match "dl" -> "Deep Learning"
        picker.set_query("dl");
        assert_eq!(picker.visible_indices, vec![1]);
        assert_eq!(picker.selected_item().unwrap().title, "Deep Learning");

        // Match subtitle "found" -> "Foundations"
        picker.set_query("found");
        assert_eq!(picker.visible_indices, vec![1]);

        // Non-matching query
        picker.set_query("nonexistent_term_xyz");
        assert!(picker.visible_indices.is_empty());
        assert_eq!(picker.selected_item(), None);
    }

    #[test]
    fn test_picker_navigation_and_clamping() {
        let items = sample_items();
        let mut picker = GenericPicker::new("Nav", items.clone(), false);

        assert_eq!(picker.selected, 0);

        picker.move_cursor(1);
        assert_eq!(picker.selected, 1);

        picker.move_cursor(2);
        assert_eq!(picker.selected, 3);

        // Clamping at end
        picker.move_cursor(10);
        assert_eq!(picker.selected, 3);

        // Move backwards
        picker.move_cursor(-2);
        assert_eq!(picker.selected, 1);

        // Clamping at beginning
        picker.move_cursor(-10);
        assert_eq!(picker.selected, 0);

        // Move to first and last
        picker.move_to_last();
        assert_eq!(picker.selected, 3);
        picker.move_to_first();
        assert_eq!(picker.selected, 0);

        // While at index 3, filter to fewer items -> should clamp within new visible_indices
        picker.move_to_last();
        assert_eq!(picker.selected, 3);
        picker.set_query("dl"); // only 1 visible item (index 0 in visible_indices)
        assert_eq!(picker.visible_indices.len(), 1);
        assert_eq!(picker.selected, 0);

        // Filter to 0 items -> selected clamped to 0
        picker.set_query("empty_filter_match");
        assert_eq!(picker.selected, 0);
        assert_eq!(picker.selected_item(), None);
    }

    #[test]
    fn test_picker_multi_select_toggle() {
        let items = sample_items();

        // Single-select mode: toggle_selected does nothing
        let mut single_picker = GenericPicker::new("Single", items.clone(), false);
        single_picker.toggle_selected();
        assert!(single_picker.checked_items().is_empty());

        // Multi-select mode: toggling adds and removes
        let mut multi_picker = GenericPicker::new("Multi", items.clone(), true);
        assert!(multi_picker.checked_items().is_empty());

        multi_picker.toggle_selected();
        assert_eq!(multi_picker.checked_items().len(), 1);
        assert!(multi_picker.checked_items().contains(&items[0].id));

        // Toggle again removes it
        multi_picker.toggle_selected();
        assert!(multi_picker.checked_items().is_empty());

        // Test with_checked builder
        let mut prechecked = HashSet::new();
        prechecked.insert(items[2].id);
        let mut multi_picker2 =
            GenericPicker::new("Multi2", items.clone(), true).with_checked(prechecked);
        assert_eq!(multi_picker2.checked_items().len(), 1);
        assert!(multi_picker2.checked_items().contains(&items[2].id));

        // Move to item 1 and toggle it
        multi_picker2.move_cursor(1);
        multi_picker2.toggle_selected();
        assert_eq!(multi_picker2.checked_items().len(), 2);
        assert!(multi_picker2.checked_items().contains(&items[1].id));
        assert!(multi_picker2.checked_items().contains(&items[2].id));
    }
}
