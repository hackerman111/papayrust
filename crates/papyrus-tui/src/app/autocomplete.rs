use std::path::{Path, PathBuf};

/// Attempts to autocomplete a filesystem path given the user's current input.
///
/// Supports:
/// - Tilde expansion (`~/` -> user's home directory).
/// - Single match completion (appends `/` for directories).
/// - Multiple matches completion up to the Longest Common Prefix (LCP).
/// - Appending trailing `/` if the input is an existing directory missing a slash.
pub fn autocomplete_path(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let home_dir = std::env::var_os("HOME").map(PathBuf::from);

    let expanded_buf: PathBuf = if let Some(stripped) = trimmed.strip_prefix("~/") {
        if let Some(ref home) = home_dir {
            home.join(stripped)
        } else {
            PathBuf::from(trimmed)
        }
    } else if trimmed == "~" {
        if let Some(ref home) = home_dir {
            home.clone()
        } else {
            PathBuf::from(trimmed)
        }
    } else {
        PathBuf::from(trimmed)
    };

    // If input is an existing directory without a trailing slash, append '/'
    if expanded_buf.is_dir() && !trimmed.ends_with('/') {
        return Some(format!("{trimmed}/"));
    }

    let (parent_dir, prefix) = if trimmed.ends_with('/') {
        (expanded_buf.as_path(), "")
    } else {
        let parent = expanded_buf.parent().unwrap_or_else(|| Path::new("."));
        let p = if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        };
        let file_name = expanded_buf
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        (p, file_name)
    };

    let entries = std::fs::read_dir(parent_dir).ok()?;
    let prefix_lower = prefix.to_lowercase();

    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            None => continue,
        };

        // Skip hidden files unless prefix starts with '.'
        if name_str.starts_with('.') && !prefix.starts_with('.') {
            continue;
        }

        if name_str.to_lowercase().starts_with(&prefix_lower) {
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            matches.push((name_str.to_string(), is_dir));
        }
    }

    if matches.is_empty() {
        return None;
    }

    matches.sort_by(|a, b| a.0.cmp(&b.0));

    if matches.len() == 1 {
        let (name, is_dir) = &matches[0];
        let completed = build_path(trimmed, prefix, name, *is_dir);
        return Some(completed);
    }

    // Multiple matches: compute longest common prefix
    let lcp = longest_common_prefix(&matches.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>());
    if lcp.len() > prefix.len() {
        let is_dir = matches.len() == 1 && matches[0].1;
        let completed = build_path(trimmed, prefix, &lcp, is_dir);
        return Some(completed);
    }

    None
}

/// Reconstructs the input string replacing the partial prefix with the completed text.
fn build_path(original_input: &str, prefix: &str, replacement: &str, is_dir: bool) -> String {
    let mut base = if prefix.is_empty() {
        original_input.to_string()
    } else if let Some(stripped) = original_input.strip_suffix(prefix) {
        format!("{stripped}{replacement}")
    } else {
        format!("{original_input}{replacement}")
    };

    if is_dir && !base.ends_with('/') {
        base.push('/');
    }
    base
}

/// Computes the longest common prefix of a slice of strings.
fn longest_common_prefix(strings: &[&str]) -> String {
    if strings.is_empty() {
        return String::new();
    }
    let first = strings[0];
    let mut len = 0;
    for (i, c) in first.char_indices() {
        if strings.iter().all(|s| s[i..].starts_with(c)) {
            len = i + c.len_utf8();
        } else {
            break;
        }
    }
    first[..len].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_longest_common_prefix() {
        assert_eq!(
            longest_common_prefix(&["apple", "app", "application"]),
            "app"
        );
        assert_eq!(longest_common_prefix(&["apple", "banana"]), "");
        assert_eq!(longest_common_prefix(&["exact"]), "exact");
    }

    #[test]
    fn test_autocomplete_single_file_and_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let sub_dir = root.join("my_documents");
        std::fs::create_dir(&sub_dir).unwrap();

        let file = root.join("my_paper.pdf");
        std::fs::write(&file, b"test").unwrap();

        let prefix_dir = format!("{}/my_d", root.display());
        let res_dir = autocomplete_path(&prefix_dir);
        assert_eq!(res_dir, Some(format!("{}/my_documents/", root.display())));

        let prefix_file = format!("{}/my_p", root.display());
        let res_file = autocomplete_path(&prefix_file);
        assert_eq!(res_file, Some(format!("{}/my_paper.pdf", root.display())));
    }

    #[test]
    fn test_autocomplete_multiple_common_prefix() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        std::fs::write(root.join("paper_ai_1.pdf"), b"1").unwrap();
        std::fs::write(root.join("paper_ai_2.pdf"), b"2").unwrap();
        std::fs::write(root.join("paper_math.pdf"), b"3").unwrap();

        let prefix = format!("{}/paper_a", root.display());
        let res = autocomplete_path(&prefix);
        assert_eq!(res, Some(format!("{}/paper_ai_", root.display())));
    }

    #[test]
    fn test_autocomplete_dir_adds_trailing_slash() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let sub = root.join("subfolder");
        std::fs::create_dir(&sub).unwrap();

        let path_str = format!("{}", sub.display());
        let res = autocomplete_path(&path_str);
        assert_eq!(res, Some(format!("{}/", sub.display())));
    }
}
