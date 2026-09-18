use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::db::Paper;
use crate::opener::error::OpenerError;

/// Resolves the actual PDF file path to open for the given paper based on application configuration.
///
/// If `config.toc.prefer_annotated_copy` is true and `paper.annotated_pdf_path` is Some,
/// and the annotated file exists on disk, the annotated path is returned.
/// Otherwise, `paper.file_path` is resolved.
///
/// Returns `OpenerError::FileNotFound` if the target file does not exist on disk.
pub fn resolve_paper_path(paper: &Paper, config: &Config) -> Result<PathBuf, OpenerError> {
    if config.toc.prefer_annotated_copy {
        if let Some(ref annotated) = paper.annotated_pdf_path {
            let path = PathBuf::from(annotated);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    let path = PathBuf::from(&paper.file_path);
    if path.exists() {
        Ok(path)
    } else {
        Err(OpenerError::FileNotFound(path))
    }
}

/// Tokenizes a command template into arguments, respecting single and double quotes.
pub fn tokenize_template(template: &str) -> Result<Vec<String>, OpenerError> {
    let trimmed = template.trim();
    if trimmed.is_empty() {
        return Err(OpenerError::InvalidTemplate(
            "command template cannot be empty".to_string(),
        ));
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = trimmed.chars().peekable();
    let mut in_quote: Option<char> = None;
    let mut has_token = false;

    while let Some(c) = chars.next() {
        if let Some(quote_char) = in_quote {
            if c == quote_char {
                in_quote = None;
            } else if c == '\\' && quote_char == '"' {
                if let Some(&next_c) = chars.peek() {
                    if next_c == '"' || next_c == '\\' {
                        current.push(next_c);
                        chars.next();
                    } else {
                        current.push('\\');
                    }
                } else {
                    current.push('\\');
                }
            } else {
                current.push(c);
            }
        } else {
            match c {
                '\'' | '"' => {
                    in_quote = Some(c);
                    has_token = true;
                }
                '\\' => {
                    if let Some(next_c) = chars.next() {
                        current.push(next_c);
                        has_token = true;
                    } else {
                        return Err(OpenerError::InvalidTemplate(
                            "trailing backslash in template".to_string(),
                        ));
                    }
                }
                c if c.is_whitespace() => {
                    if has_token {
                        tokens.push(std::mem::take(&mut current));
                        has_token = false;
                    }
                }
                _ => {
                    current.push(c);
                    has_token = true;
                }
            }
        }
    }

    if in_quote.is_some() {
        return Err(OpenerError::InvalidTemplate(
            "unclosed quote in template".to_string(),
        ));
    }

    if has_token {
        tokens.push(current);
    }

    if tokens.is_empty() {
        return Err(OpenerError::InvalidTemplate(
            "command template yielded no tokens".to_string(),
        ));
    }

    Ok(tokens)
}

/// Helper that replaces `{path}` in `token` with `path_str` if present.
pub fn substitute_path(token: &str, path_str: &str) -> (String, bool) {
    if token.contains("{path}") {
        (token.replace("{path}", path_str), true)
    } else {
        (token.to_string(), false)
    }
}

/// Safely builds the command program and arguments vector from a template string.
///
/// Replaces `{path}` with `file_path.display()`, and `{page}` with `page.to_string()`.
/// If `page` is None and `{page}` is present, gracefully drops the page flag / argument.
/// If `page` is Some(0), returns `OpenerError::InvalidPage(0)`.
/// If `{path}` was not explicitly specified in the template, appends `file_path` as the final argument.
pub fn build_open_command(
    template: &str,
    file_path: &Path,
    page: Option<u32>,
) -> Result<(String, Vec<String>), OpenerError> {
    if let Some(0) = page {
        return Err(OpenerError::InvalidPage(0));
    }

    let tokens = tokenize_template(template)?;
    let mut resolved_tokens = Vec::with_capacity(tokens.len());
    let path_str = file_path.display().to_string();
    let mut path_substituted = false;

    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];

        if token.contains("{page}") {
            match page {
                Some(p) => {
                    let substituted = token.replace("{page}", &p.to_string());
                    let (sub_path, used) = substitute_path(&substituted, &path_str);
                    if used {
                        path_substituted = true;
                    }
                    resolved_tokens.push(sub_path);
                }
                None => {
                    // Fallback when page is None:
                    // 1. If token is exactly "{page}", check if preceding token was a page flag like "--page", "-p", "-P",
                    // and drop the flag as well.
                    if token == "{page}" {
                        if let Some(last) = resolved_tokens.last() {
                            if last == "--page" || last == "-p" || last == "-P" {
                                resolved_tokens.pop();
                            }
                        }
                    } else if token.contains("{path}") {
                        // Token contains both {page} and {path}, fallback page to 1
                        let substituted = token.replace("{page}", "1");
                        let (sub_path, used) = substitute_path(&substituted, &path_str);
                        if used {
                            path_substituted = true;
                        }
                        resolved_tokens.push(sub_path);
                    }
                    // For tokens like "--page={page}", omitting this token completely removes the flag.
                }
            }
            i += 1;
            continue;
        }

        let (sub_path, used) = substitute_path(token, &path_str);
        if used {
            path_substituted = true;
        }
        resolved_tokens.push(sub_path);
        i += 1;
    }

    if resolved_tokens.is_empty() {
        return Err(OpenerError::InvalidTemplate(
            "command template produced no program executable".to_string(),
        ));
    }

    let program = resolved_tokens.remove(0);

    // If {path} was not in the template, append the path as the last argument
    if !path_substituted {
        resolved_tokens.push(path_str);
    }

    Ok((program, resolved_tokens))
}
