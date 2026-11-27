//! Corpus discovery and filtering for benchmark documents.
//!
//! Builds on the existing [`FixtureManager`] to provide structured corpus access
//! with filtering by file type, ground truth availability, and name patterns.

use crate::Result;
use crate::fixture::FixtureManager;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A document in the benchmark corpus with resolved paths.
#[derive(Debug, Clone)]
pub struct CorpusDocument {
    /// Human-readable name (fixture stem, e.g. "nougat_001")
    pub name: String,
    /// Absolute path to the source document
    pub document_path: PathBuf,
    /// File type (e.g. "pdf", "docx")
    pub file_type: String,
    /// File size in bytes
    pub file_size: u64,
    /// Absolute path to text ground truth (if available)
    pub ground_truth_text: Option<PathBuf>,
    /// Absolute path to markdown ground truth (if available)
    pub ground_truth_markdown: Option<PathBuf>,
    /// Fixture metadata used by maintained benchmark groups.
    pub metadata: HashMap<String, serde_json::Value>,
    /// Fixture descriptor path, retained for reproducibility hashing.
    pub fixture_path: PathBuf,
}

/// Filter criteria for corpus discovery.
#[derive(Debug, Clone, Default)]
pub struct CorpusFilter {
    /// Only include these file types (None = all)
    pub file_types: Option<Vec<String>>,
    /// Require text ground truth
    pub require_ground_truth: bool,
    /// Require markdown ground truth
    pub require_markdown_ground_truth: bool,
    /// Maximum file size in bytes (None = no limit)
    pub max_file_size: Option<u64>,
    /// Only include fixtures whose name contains one of these strings
    pub name_patterns: Vec<String>,
    /// Exact fixture stems or fixture-root-relative JSON descriptor paths.
    /// Combined with `name_patterns` using OR semantics. ~keep
    pub exact_names: Vec<String>,
    /// Only include fixtures whose `metadata.category` is exactly this string.
    pub category: Option<String>,
}

fn corpus_document(fixture_path: &Path, fixture: &crate::fixture::Fixture) -> Option<CorpusDocument> {
    let fixture_dir = fixture_path.parent()?;
    let name = fixture_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("")
        .to_string();

    Some(CorpusDocument {
        name,
        document_path: fixture.resolve_document_path(fixture_dir),
        file_type: fixture.file_type.clone(),
        file_size: fixture.file_size,
        ground_truth_text: fixture.resolve_ground_truth_path(fixture_dir),
        ground_truth_markdown: fixture.resolve_ground_truth_markdown_path(fixture_dir),
        metadata: fixture.metadata.clone(),
        fixture_path: fixture_path.to_path_buf(),
    })
}

/// Convert already-loaded fixtures to corpus documents without changing their order.
pub fn corpus_from_fixture_manager(manager: &FixtureManager) -> Vec<CorpusDocument> {
    manager
        .fixtures()
        .iter()
        .filter_map(|(fixture_path, fixture)| corpus_document(fixture_path, fixture))
        .collect()
}

/// True when `requested` names the same format as `actual`, either literally or because both
/// are extensions xberg's MIME table treats as aliases of one format (e.g. `typst` and `typ`).
///
/// Fixture `file_type` values are deliberately diverse per-document — the corpus keeps real
/// documents under several legal extension aliases of the same format on purpose (see
/// [`crate::fixture::canonical_extension_family`]) — so a literal filter match alone would
/// silently return only a fraction of a format's fixtures to a caller who filtered by one
/// spelling (e.g. `--file-types typst` missing every `typ`-labeled fixture).
fn file_type_matches(requested: &str, actual: &str) -> bool {
    if requested.eq_ignore_ascii_case(actual) {
        return true;
    }
    crate::fixture::canonical_extension_family(requested)
        .is_some_and(|family| family.iter().any(|extension| extension.eq_ignore_ascii_case(actual)))
}

fn matches_filter(doc: &CorpusDocument, relative_fixture_path: &Path, filter: &CorpusFilter) -> bool {
    let has_name_filter = !filter.name_patterns.is_empty() || !filter.exact_names.is_empty();
    let matches_name = filter
        .name_patterns
        .iter()
        .any(|pattern| doc.name.contains(pattern.as_str()))
        || filter
            .exact_names
            .iter()
            .any(|exact| exact == &doc.name || Path::new(exact) == relative_fixture_path);

    (!has_name_filter || matches_name)
        && filter.file_types.as_ref().is_none_or(|types| {
            types
                .iter()
                .any(|requested| file_type_matches(requested, &doc.file_type))
        })
        && filter.max_file_size.is_none_or(|max_size| doc.file_size <= max_size)
        && (!filter.require_ground_truth || doc.ground_truth_text.is_some())
        && (!filter.require_markdown_ground_truth || doc.ground_truth_markdown.is_some())
        && filter
            .category
            .as_deref()
            .is_none_or(|category| doc.metadata.get("category").and_then(serde_json::Value::as_str) == Some(category))
}

fn sort_corpus_by_selection(docs: &mut [CorpusDocument], fixtures_dir: &Path, exact_names: &[String]) {
    docs.sort_by(|a, b| {
        let exact_position = |doc: &CorpusDocument| {
            let relative = doc.fixture_path.strip_prefix(fixtures_dir).unwrap_or(&doc.fixture_path);
            exact_names.iter().position(|exact| Path::new(exact) == relative)
        };

        match (exact_position(a), exact_position(b)) {
            (Some(a_position), Some(b_position)) => a_position.cmp(&b_position),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.name.cmp(&b.name),
        }
    });
}

/// Build a filtered corpus from the fixture directory.
pub fn build_corpus(fixtures_dir: &Path, filter: &CorpusFilter) -> Result<Vec<CorpusDocument>> {
    let mut manager = FixtureManager::new();
    if fixtures_dir.is_dir() {
        manager.load_fixtures_from_dir(fixtures_dir)?;
    } else {
        manager.load_fixture(fixtures_dir)?;
    }

    let mut docs = Vec::new();

    for (fixture_path, fixture) in manager.fixtures() {
        let Some(doc) = corpus_document(fixture_path, fixture) else {
            continue;
        };
        let relative_fixture_path = fixture_path.strip_prefix(fixtures_dir).unwrap_or(fixture_path);
        if matches_filter(&doc, relative_fixture_path, filter) {
            docs.push(doc);
        }
    }

    sort_corpus_by_selection(&mut docs, fixtures_dir, &filter.exact_names);
    Ok(docs)
}

/// Convenience: all PDFs with text ground truth.
pub fn pdf_corpus(fixtures_dir: &Path) -> Result<Vec<CorpusDocument>> {
    build_corpus(
        fixtures_dir,
        &CorpusFilter {
            file_types: Some(vec!["pdf".to_string()]),
            require_ground_truth: true,
            ..Default::default()
        },
    )
}

/// Convenience: all PDFs with markdown ground truth.
pub fn pdf_markdown_corpus(fixtures_dir: &Path) -> Result<Vec<CorpusDocument>> {
    build_corpus(
        fixtures_dir,
        &CorpusFilter {
            file_types: Some(vec!["pdf".to_string()]),
            require_ground_truth: true,
            require_markdown_ground_truth: true,
            ..Default::default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_default_filter_is_permissive() {
        let filter = CorpusFilter::default();
        assert!(filter.file_types.is_none());
        assert!(!filter.require_ground_truth);
        assert!(!filter.require_markdown_ground_truth);
        assert!(filter.max_file_size.is_none());
        assert!(filter.name_patterns.is_empty());
        assert!(filter.exact_names.is_empty());
        assert!(filter.category.is_none());
    }

    #[test]
    fn category_filter_matches_metadata_category_exactly() {
        let mut doc = CorpusDocument {
            name: "sample".to_string(),
            document_path: PathBuf::from("sample.png"),
            file_type: "png".to_string(),
            file_size: 1,
            ground_truth_text: Some(PathBuf::from("sample.txt")),
            ground_truth_markdown: None,
            metadata: HashMap::from([("category".to_string(), serde_json::json!("image-ocr-realgt"))]),
            fixture_path: PathBuf::from("sample.json"),
        };
        let relative_path = Path::new("sample.json");
        let filter = CorpusFilter {
            category: Some("image-ocr-realgt".to_string()),
            ..Default::default()
        };

        assert!(matches_filter(&doc, relative_path, &filter));

        doc.metadata
            .insert("category".to_string(), serde_json::json!("image-ocr-realgt-extra"));
        assert!(!matches_filter(&doc, relative_path, &filter));

        doc.metadata.clear();
        assert!(!matches_filter(&doc, relative_path, &filter));
    }

    #[test]
    fn category_and_name_filters_are_combined() {
        let doc = CorpusDocument {
            name: "sample".to_string(),
            document_path: PathBuf::from("sample.png"),
            file_type: "png".to_string(),
            file_size: 1,
            ground_truth_text: Some(PathBuf::from("sample.txt")),
            ground_truth_markdown: None,
            metadata: HashMap::from([("category".to_string(), serde_json::json!("image-ocr-realgt"))]),
            fixture_path: PathBuf::from("sample.json"),
        };
        let filter = CorpusFilter {
            name_patterns: vec!["other".to_string()],
            category: Some("image-ocr-realgt".to_string()),
            ..Default::default()
        };

        assert!(!matches_filter(&doc, Path::new("sample.json"), &filter));
    }

    #[test]
    fn exact_fixture_paths_are_selected_in_declared_order() {
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixtures_dir = crate_root.join("fixtures");
        let exact_names = vec![
            "xlsx/stanley_cups.json".to_string(),
            "csv/stanley_cups.json".to_string(),
        ];
        let docs = build_corpus(
            &fixtures_dir,
            &CorpusFilter {
                exact_names,
                ..Default::default()
            },
        )
        .unwrap();

        let fixture_paths: Vec<_> = docs
            .iter()
            .map(|doc| doc.fixture_path.strip_prefix(&fixtures_dir).unwrap())
            .collect();
        assert_eq!(
            fixture_paths,
            [Path::new("xlsx/stanley_cups.json"), Path::new("csv/stanley_cups.json")]
        );
    }

    #[test]
    fn file_type_matches_accepts_the_literal_spelling() {
        assert!(file_type_matches("pdf", "pdf"));
        assert!(!file_type_matches("pdf", "docx"));
    }

    #[test]
    fn file_type_matches_accepts_a_recognized_alias_in_either_direction() {
        // typst/typ, dbk/docbook, wp/wpd, jpg/jpeg, tiff/tif and yaml/yml are all legal alias
        // pairs of one format per crates/xberg/src/core/mime.rs — a filter for either spelling
        // must match a fixture labeled with the other.
        assert!(file_type_matches("typst", "typ"));
        assert!(file_type_matches("typ", "typst"));
        assert!(file_type_matches("docbook", "dbk"));
        assert!(file_type_matches("wpd", "wp"));
        assert!(file_type_matches("jpg", "jpeg"));
        assert!(file_type_matches("tiff", "tif"));
        assert!(file_type_matches("yaml", "yml"));
        assert!(!file_type_matches("typst", "docbook"));
    }

    /// Regression test for the fixture corpus's split file_type spelling: before
    /// `matches_filter` became alias-aware, filtering by one spelling of a format silently
    /// returned only the fixtures using that exact literal spelling. The typst family is the
    /// starkest example — 8 of the corpus's 9 typst fixtures spell `file_type` as `"typ"`, so a
    /// literal `"typst"` filter used to return just 1 document. It must now return all 9.
    #[test]
    fn typst_filter_returns_every_typ_and_typst_fixture() {
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixtures_dir = crate_root.join("fixtures");

        let docs = build_corpus(
            &fixtures_dir,
            &CorpusFilter {
                file_types: Some(vec!["typst".to_string()]),
                ..Default::default()
            },
        )
        .unwrap();

        let file_types: HashSet<_> = docs.iter().map(|doc| doc.file_type.as_str()).collect();
        assert_eq!(file_types, HashSet::from(["typ", "typst"]));
        assert_eq!(docs.len(), 9, "expected the full typst family, got {docs:?}");
    }

    /// Same regression as above, checked for every format the corpus keeps under a split
    /// spelling: wordperfect (wp/wpd), docbook (dbk/docbook), jpeg (jpg/jpeg), tiff (tiff/tif)
    /// and yaml (yaml/yml). Counts reflect the fixture corpus as of this fix and will need
    /// updating if fixtures for these formats are added or removed.
    #[test]
    fn family_aware_filters_recover_the_full_bucket_for_every_split_format() {
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixtures_dir = crate_root.join("fixtures");

        let expected_counts: &[(&str, usize)] = &[("wpd", 7), ("docbook", 7), ("jpeg", 54), ("tiff", 2), ("yaml", 6)];

        for (requested, expected_count) in expected_counts {
            let docs = build_corpus(
                &fixtures_dir,
                &CorpusFilter {
                    file_types: Some(vec![(*requested).to_string()]),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(
                docs.len(),
                *expected_count,
                "file_types=[{requested}] returned {} documents, expected {expected_count}",
                docs.len()
            );
        }
    }
}
