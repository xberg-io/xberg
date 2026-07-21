//! Fast benchmark groups: curated document subsets for targeted iteration.

use std::path::Path;

use crate::Result;
use crate::corpus::{CorpusDocument, CorpusFilter, build_corpus};

/// A named benchmark group backed by exact document IDs and/or fixture metadata.
pub struct BenchmarkGroup {
    pub name: &'static str,
    pub description: &'static str,
    /// Exact fixture stems. These are intentionally not substring patterns.
    pub docs: &'static [&'static str],
    /// Match any fixture whose `metadata.size_tier` is one of these values.
    pub size_tiers: &'static [&'static str],
    /// Match any fixture whose `metadata.role` is one of these values.
    pub roles: &'static [&'static str],
    /// Reject fixtures whose `metadata.role` is one of these values.
    /// Exclusions take precedence over every positive selector.
    pub excluded_roles: &'static [&'static str],
    /// Match any fixture containing one of these `metadata.cohorts` values.
    pub cohorts: &'static [&'static str],
}

const PROMOTION_VALIDATION_DOCS: &[&str] = &[
    "0903.1810",
    "10075815",
    "113012366",
    "167647146",
    "2001.09113",
    "2026-13845",
    "2026-14578",
    "2211.13451",
    "26614980",
    concat!("ft_B", "AX_2012_page_100_t1"),
    concat!("ft_C", "B_2008_page_102_t0"),
    concat!("pb_1948bb0c-01f1-aa67-a1bd-", "4d54323f4f0d_page2"),
    concat!("pb_203924_", "fb04e77929bf4bbc93dbd659653a4f01_page26"),
    concat!("pb_B", "RWS-134565917_page1171"),
    concat!("pb_S", "ERFF_TX_random_pages_1_page650"),
    concat!("pb_f", "qr-retail-blackrock-global-allocation-fund-inc_page8"),
];

pub const GROUPS: &[BenchmarkGroup] = &[
    BenchmarkGroup {
        name: "hotspot",
        description: "Maintained fast loop for current PDF quality hotspots",
        docs: &[
            "160428551",
            "2309.17020",
            "24231810",
            "681693",
            concat!("ft_A", "CN_2009_page_102_t0"),
            "pb_FBLB-134215544_page147",
            "pb_fqr-retail-blackrock-global-allocation-fund-inc_page4",
            "pb_sample_page_16_page1",
        ],
        size_tiers: &[],
        roles: &[],
        excluded_roles: &[],
        cohorts: &[],
    },
    BenchmarkGroup {
        name: "smoke",
        description: "Corpus-maintained smoke/tune tier, excluding evaluation holdouts",
        docs: &[],
        size_tiers: &["smoke"],
        roles: &[],
        excluded_roles: &["eval"],
        cohorts: &[],
    },
    BenchmarkGroup {
        name: "promotion",
        description: "Smoke/tune tier plus the frozen exact validation gate, excluding evaluation holdouts",
        docs: PROMOTION_VALIDATION_DOCS,
        size_tiers: &["smoke"],
        roles: &[],
        excluded_roles: &["eval"],
        cohorts: &[],
    },
    BenchmarkGroup {
        name: "holdout",
        description: "Final-only held-out evaluation fixtures (metadata.role=eval)",
        docs: &[],
        size_tiers: &[],
        roles: &["eval"],
        excluded_roles: &[],
        cohorts: &[],
    },
    BenchmarkGroup {
        name: "tables",
        description: "All fixtures tagged with the tables cohort",
        docs: &[],
        size_tiers: &[],
        roles: &[],
        excluded_roles: &[],
        cohorts: &["tables"],
    },
    BenchmarkGroup {
        name: "structure",
        description: "All fixtures tagged with nested heading structure",
        docs: &[],
        size_tiers: &[],
        roles: &[],
        excluded_roles: &[],
        cohorts: &["nested-heading"],
    },
    BenchmarkGroup {
        name: "lists",
        description: "All fixtures tagged with nested lists",
        docs: &[],
        size_tiers: &[],
        roles: &[],
        excluded_roles: &[],
        cohorts: &["nested-list"],
    },
];

impl BenchmarkGroup {
    pub fn matches(&self, doc: &CorpusDocument) -> bool {
        let included = self.docs.contains(&doc.name.as_str())
            || metadata_string_matches(&doc.metadata, "size_tier", self.size_tiers)
            || metadata_string_matches(&doc.metadata, "role", self.roles)
            || metadata_array_matches(&doc.metadata, "cohorts", self.cohorts);
        included && !metadata_string_matches(&doc.metadata, "role", self.excluded_roles)
    }
}

fn metadata_string_matches(
    metadata: &std::collections::HashMap<String, serde_json::Value>,
    key: &str,
    expected: &[&str],
) -> bool {
    !expected.is_empty()
        && metadata
            .get(key)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| expected.contains(&value))
}

fn metadata_array_matches(
    metadata: &std::collections::HashMap<String, serde_json::Value>,
    key: &str,
    expected: &[&str],
) -> bool {
    !expected.is_empty()
        && metadata
            .get(key)
            .and_then(serde_json::Value::as_array)
            .is_some_and(|values| {
                values
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .any(|value| expected.contains(&value))
            })
}

/// Resolve a group to exact fixture stems using current corpus metadata.
pub fn resolve_group_docs(fixtures_dir: &Path, group: &BenchmarkGroup) -> Result<Vec<String>> {
    let pdf_dir = fixtures_dir.join("pdf");
    let corpus_root = if pdf_dir.is_dir() {
        pdf_dir.as_path()
    } else {
        fixtures_dir
    };
    let docs = build_corpus(
        corpus_root,
        &CorpusFilter {
            file_types: Some(vec!["pdf".to_string()]),
            require_ground_truth: true,
            ..Default::default()
        },
    )?;
    let matches: Vec<String> = docs
        .into_iter()
        .filter(|doc| group.matches(doc))
        .map(|doc| doc.name)
        .collect();
    if matches.is_empty() {
        return Err(crate::Error::Config(format!(
            "benchmark group '{}' matched zero documents in {}",
            group.name,
            corpus_root.display()
        )));
    }
    Ok(matches)
}

/// Find a group by name, case-insensitive.
pub fn find_group(name: &str) -> Option<&'static BenchmarkGroup> {
    GROUPS.iter().find(|g| g.name.eq_ignore_ascii_case(name))
}

/// List all available group names.
pub fn group_names() -> Vec<&'static str> {
    GROUPS.iter().map(|g| g.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashMap};
    use std::fs;
    use std::path::PathBuf;

    fn doc(name: &str, metadata: serde_json::Value) -> CorpusDocument {
        CorpusDocument {
            name: name.to_string(),
            document_path: PathBuf::new(),
            file_type: "pdf".to_string(),
            file_size: 0,
            ground_truth_text: None,
            ground_truth_markdown: None,
            metadata: serde_json::from_value::<HashMap<String, serde_json::Value>>(metadata).unwrap(),
            fixture_path: PathBuf::new(),
        }
    }

    fn write_fixture(root: &Path, name: &str, file_type: &str, metadata: serde_json::Value) {
        let document = format!("{name}.{file_type}");
        let ground_truth = format!("{name}.txt");
        fs::write(root.join(&document), "source").unwrap();
        fs::write(root.join(&ground_truth), "ground truth").unwrap();
        fs::write(
            root.join(format!("{name}.json")),
            serde_json::to_vec(&serde_json::json!({
                "document": document,
                "file_type": file_type,
                "file_size": 6,
                "metadata": metadata,
                "ground_truth": {
                    "text_file": ground_truth,
                    "source": "manual"
                }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn resolve_maintained_group(fixtures: &Path, name: &str) -> Vec<String> {
        let group = find_group(name).unwrap();
        resolve_group_docs(fixtures, group).unwrap_or_else(|error| {
            panic!(
                "failed to resolve maintained '{name}' gate; ensure benchmark fixtures and LFS documents are available: {error}"
            )
        })
    }

    fn assert_maintained_group_size(fixtures: &Path, name: &str, expected: usize) {
        let docs = resolve_maintained_group(fixtures, name);
        assert_eq!(
            docs.len(),
            expected,
            "benchmark corpus rebuild changed maintained '{name}' gate size; review its membership before updating this assertion"
        );
    }

    #[test]
    fn fast_gates_exclude_evaluation_holdouts() {
        let smoke = find_group("smoke").unwrap();
        let promotion = find_group("promotion").unwrap();
        let holdout = find_group("holdout").unwrap();
        let tune_smoke = doc("a", serde_json::json!({"size_tier": "smoke", "role": "tune"}));
        let eval_smoke = doc("b", serde_json::json!({"size_tier": "smoke", "role": "eval"}));
        assert!(smoke.matches(&tune_smoke));
        assert!(!smoke.matches(&eval_smoke));
        assert!(promotion.matches(&tune_smoke));
        assert!(!promotion.matches(&eval_smoke));
        assert!(holdout.matches(&eval_smoke));
        assert!(!holdout.matches(&tune_smoke));
    }

    #[test]
    fn excluded_role_overrides_exact_document_match() {
        let promotion = find_group("promotion").unwrap();
        let validation_doc = promotion.docs[0];
        let eval = doc(validation_doc, serde_json::json!({"role": "eval"}));
        assert!(!promotion.matches(&eval));
    }

    #[test]
    fn thematic_group_matches_cohort() {
        let tables = find_group("tables").unwrap();
        let table_doc = doc("a", serde_json::json!({"cohorts": ["native-clean", "tables"]}));
        assert!(tables.matches(&table_doc));
    }

    #[test]
    fn group_resolution_is_limited_to_pdf_fixtures() {
        let fixtures = tempfile::tempdir().unwrap();
        let metadata = serde_json::json!({"size_tier": "smoke", "role": "tune"});
        write_fixture(fixtures.path(), "pdf_doc", "pdf", metadata.clone());
        write_fixture(fixtures.path(), "text_doc", "txt", metadata);

        let docs = resolve_group_docs(fixtures.path(), find_group("smoke").unwrap()).unwrap();
        assert_eq!(docs, ["pdf_doc"]);
    }

    #[test]
    fn zero_match_group_is_an_error() {
        let fixtures = tempfile::tempdir().unwrap();
        let error = resolve_group_docs(fixtures.path(), find_group("smoke").unwrap()).unwrap_err();
        assert!(matches!(error, crate::Error::Config(_)));
    }

    #[test]
    fn maintained_pdf_corpus_resolves_frozen_fast_gates() {
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        assert_maintained_group_size(&fixtures, "smoke", 10);
        assert_maintained_group_size(&fixtures, "promotion", 26);
        assert_maintained_group_size(&fixtures, "holdout", 1);

        let smoke: BTreeSet<String> = resolve_maintained_group(&fixtures, "smoke").into_iter().collect();
        let promotion: BTreeSet<String> = resolve_maintained_group(&fixtures, "promotion").into_iter().collect();
        let expected: BTreeSet<String> = PROMOTION_VALIDATION_DOCS
            .iter()
            .map(|name| (*name).to_string())
            .collect();

        assert_eq!(
            expected.len(),
            PROMOTION_VALIDATION_DOCS.len(),
            "frozen promotion validation fixture IDs must be unique"
        );
        assert_eq!(
            promotion.difference(&smoke).cloned().collect::<BTreeSet<_>>(),
            expected,
            "promotion-minus-smoke must remain the frozen validation set"
        );
    }
}
