//! Quality scoring: score extracted content against ground truth (text F1 + structural
//! breakdown), and read pre-computed vendored pipeline output from disk.

use crate::Result;
use crate::quality::{compute_f1, structural_sidecar, tokenize};
use std::collections::HashMap;
use std::path::Path;

/// Structural scoring outputs for one document: the SF1 rollup, reading-order
/// score, and the per-dimension F1 / precision / recall maps used by triage.
/// Precision/recall are report-only diagnostics (not part of SF1).
#[derive(Debug, Clone, Default)]
pub struct StructuralBreakdown {
    pub sf1: f64,
    pub order_score: f64,
    pub per_type_sf1: HashMap<String, f64>,
    pub per_type_precision: HashMap<String, f64>,
    pub per_type_recall: HashMap<String, f64>,
}

impl StructuralBreakdown {
    fn unavailable() -> Self {
        Self {
            sf1: f64::NAN,
            order_score: f64::NAN,
            ..Default::default()
        }
    }

    /// Explode a canonical [`StructuralScore`] into per-dimension maps.
    pub fn from_score(score: &structural_sidecar::StructuralScore) -> Self {
        let mut per_type_sf1 = HashMap::new();
        let mut per_type_precision = HashMap::new();
        let mut per_type_recall = HashMap::new();
        for (name, bd) in score.dimensions_pr() {
            per_type_sf1.insert(name.to_string(), bd.f1);
            per_type_precision.insert(name.to_string(), bd.precision);
            per_type_recall.insert(name.to_string(), bd.recall);
        }
        Self {
            sf1: score.sf1,
            order_score: score.d5_order,
            per_type_sf1,
            per_type_precision,
            per_type_recall,
        }
    }
}

/// Score extracted content against ground truth, returning the text F1 and the
/// structural breakdown (SF1, reading order, per-dimension F1/precision/recall).
pub fn score_document(content: &str, gt_text: &str, gt_markdown: Option<&str>) -> (f64, StructuralBreakdown) {
    let tf1 = {
        // Leading front matter is metadata, not body text, as in `compute_quality_with_structure`.
        let ext_tokens = tokenize(crate::markdown_quality::strip_leading_front_matter(content));
        let gt_tokens = tokenize(gt_text);
        compute_f1(&ext_tokens, &gt_tokens)
    };
    let breakdown = match gt_markdown {
        Some(md) => StructuralBreakdown::from_score(&structural_sidecar::score_markdown(content, md)),
        None => StructuralBreakdown::unavailable(),
    };
    (tf1, breakdown)
}

fn resolve_vendored_dir(fixtures_path: &Path, vendored_name: &str) -> std::path::PathBuf {
    let search_start = if fixtures_path.is_file() {
        fixtures_path.parent().unwrap_or(fixtures_path)
    } else {
        fixtures_path
    };
    let fixtures_root = search_start
        .ancestors()
        .find(|ancestor| ancestor.file_name().is_some_and(|name| name == "fixtures"))
        .unwrap_or(search_start);

    fixtures_root
        .parent()
        .unwrap_or(fixtures_root)
        .join("vendored")
        .join(vendored_name)
}

/// Read vendored markdown + cached timing for a single document.
///
/// Returns an error when the cached markdown is missing or empty. A missing
/// timing remains valid for quality-only comparisons and is represented by
/// `NaN`.
pub fn read_vendored_cached(doc_name: &str, fixtures_path: &Path, vendored_name: &str) -> Result<(String, f64)> {
    let vendored_dir = resolve_vendored_dir(fixtures_path, vendored_name);
    let md_path = vendored_dir.join("md").join(format!("{}.md", doc_name));
    let timing_path = vendored_dir.join("timing").join(format!("{}.ms", doc_name));
    let md = std::fs::read_to_string(&md_path).map_err(|error| {
        crate::Error::Benchmark(format!(
            "Failed to read vendored {vendored_name} output for {doc_name} at {}: {error}",
            md_path.display()
        ))
    })?;
    if md.trim().is_empty() {
        return Err(crate::Error::Benchmark(format!(
            "Vendored {vendored_name} output for {doc_name} is empty at {}",
            md_path.display()
        )));
    }
    let cached_ms = std::fs::read_to_string(&timing_path)
        .ok()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(f64::NAN);
    Ok((md, cached_ms))
}
