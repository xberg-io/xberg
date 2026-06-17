use crate::error::Result;
use crate::parser;
use crate::types::{Language, Snippet, SnippetAnnotation};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Discover all snippet files from the given directories.
pub fn discover_snippets(dirs: &[PathBuf], language_filter: Option<&[Language]>) -> Result<Vec<Snippet>> {
    let mut snippets = Vec::new();

    for dir in dirs {
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            // Skip alef-generated API reference docs — code blocks in these are
            // language signature declarations (not runnable), so syntax-validating
            // them as standalone programs would always fail.
            if path
                .file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|n| n.starts_with("api-") && n.ends_with(".md"))
            {
                continue;
            }
            let file_snippets = extract_snippets_from_file(path, dir)?;

            for snippet in file_snippets {
                if let Some(filter) = language_filter
                    && !filter.contains(&snippet.language)
                {
                    continue;
                }
                snippets.push(snippet);
            }
        }
    }

    snippets.sort_by(|a, b| a.path.cmp(&b.path).then(a.block_index.cmp(&b.block_index)));
    Ok(snippets)
}

fn extract_snippets_from_file(path: &Path, base_dir: &Path) -> Result<Vec<Snippet>> {
    let blocks = parser::parse_code_blocks(path)?;
    let mut snippets = Vec::new();

    // Try to infer language from directory structure (e.g., docs/snippets/rust/...)
    let dir_language = infer_language_from_path(path, base_dir);

    for (idx, block) in blocks.into_iter().enumerate() {
        let language = {
            let from_fence = Language::from_fence_tag(&block.lang);
            if from_fence != Language::Unknown {
                from_fence
            } else {
                dir_language.unwrap_or(Language::Unknown)
            }
        };

        if language == Language::Unknown {
            continue;
        }

        let annotation = block.preceding_comment.as_deref().and_then(parse_annotation);

        snippets.push(Snippet {
            path: path.to_path_buf(),
            language,
            title: block.title,
            code: block.code,
            start_line: block.start_line,
            block_index: idx,
            annotation,
        });
    }

    Ok(snippets)
}

/// Infer language from path components.
/// e.g., docs/snippets/rust/api/foo.md -> Rust
fn infer_language_from_path(path: &Path, base_dir: &Path) -> Option<Language> {
    let relative = path.strip_prefix(base_dir).ok()?;
    let first_component = relative.components().next()?;
    let dir_name = first_component.as_os_str().to_str()?;
    let lang = Language::from_dir_name(dir_name);
    if lang != Language::Unknown { Some(lang) } else { None }
}

fn parse_annotation(comment: &str) -> Option<SnippetAnnotation> {
    let inner = comment.trim().strip_prefix("<!--")?.strip_suffix("-->")?.trim();

    match inner {
        "snippet:skip" => Some(SnippetAnnotation::Skip),
        "snippet:compile-only" => Some(SnippetAnnotation::CompileOnly),
        "snippet:syntax-only" => Some(SnippetAnnotation::SyntaxOnly),
        _ => None,
    }
}

/// Count snippets grouped by language.
pub fn count_by_language(snippets: &[Snippet]) -> Vec<(Language, usize)> {
    let mut counts: std::collections::HashMap<Language, usize> = std::collections::HashMap::new();
    for s in snippets {
        *counts.entry(s.language).or_default() += 1;
    }
    let mut result: Vec<_> = counts.into_iter().collect();
    result.sort_by_key(|(lang, _)| lang.to_string());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_annotation_skip() {
        assert_eq!(parse_annotation("<!-- snippet:skip -->"), Some(SnippetAnnotation::Skip));
    }

    #[test]
    fn test_parse_annotation_compile_only() {
        assert_eq!(
            parse_annotation("<!-- snippet:compile-only -->"),
            Some(SnippetAnnotation::CompileOnly)
        );
    }

    #[test]
    fn test_parse_annotation_syntax_only() {
        assert_eq!(
            parse_annotation("<!-- snippet:syntax-only -->"),
            Some(SnippetAnnotation::SyntaxOnly)
        );
    }

    #[test]
    fn test_parse_annotation_unrecognized() {
        assert_eq!(parse_annotation("<!-- some other comment -->"), None);
    }

    #[test]
    fn test_infer_language_from_path() {
        let base = Path::new("/docs/snippets");
        let path = Path::new("/docs/snippets/rust/api/example.md");
        assert_eq!(infer_language_from_path(path, base), Some(Language::Rust));

        let path = Path::new("/docs/snippets/python/getting-started/basic.md");
        assert_eq!(infer_language_from_path(path, base), Some(Language::Python));
    }
}
