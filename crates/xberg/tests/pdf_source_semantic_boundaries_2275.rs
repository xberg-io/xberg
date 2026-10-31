#![cfg(all(feature = "full", feature = "formula-recognition", feature = "bundle-tessdata-eng"))]

mod helpers;

use helpers::{extract_uri_document_blocking, get_test_file_path};
use regex::Regex;
use sha2::{Digest, Sha256};
use xberg::core::config::{ExtractionConfig, OutputFormat, PdfBackend};
use xberg::types::document_structure::{AnnotationKind, DocumentNode, DocumentStructure, NodeContent};

fn source_document(name: &str, sha256: &str) -> xberg::ExtractedDocument {
    let path = get_test_file_path(&format!("pdf/regression_pages/{name}.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("mandatory source {}: {error}", path.display()));
    assert!(bytes.starts_with(b"%PDF-"));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        sha256,
        "visually inspected source identity"
    );
    // Markdown already selects structured native extraction; expose those nodes for semantic assertions. ~keep
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        ..Default::default()
    };
    assert!(config.pdf_options.is_none(), "retain canonical native PDF defaults");
    assert!(matches!(PdfBackend::default(), PdfBackend::Native));
    let result = extract_uri_document_blocking(&path, Some("application/pdf"), &config)
        .expect("canonical public native source extraction");
    assert!(!result.content.is_empty(), "actual nonempty rendered source");
    assert!(
        result
            .document
            .as_ref()
            .is_some_and(|document| !document.nodes.is_empty()),
        "actual nonzero source elements"
    );
    result
}

fn unique_node<'a>(document: &'a DocumentStructure, anchor: &str) -> (usize, &'a DocumentNode) {
    let nodes: Vec<_> = document
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.content.text().is_some_and(|text| text.contains(anchor)))
        .collect();
    assert_eq!(nodes.len(), 1, "unique nonempty source context {anchor:?}");
    nodes[0]
}

fn source_text(document: &DocumentStructure) -> String {
    document
        .nodes
        .iter()
        .filter_map(|node| node.content.text())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn should_keep_exact_source_decimals_in_the_time_exercise_2275() {
    let result = source_document(
        "gutenberg-33283-p74",
        "958bae6a83c56fe3777ead14cb592df44c79e574effc3bc03c774256375db40f",
    );
    let document = result.document.as_ref().unwrap();
    unique_node(document, "A body falling freely");
    let text = source_text(document);
    let compact: String = text.chars().filter(|character| !character.is_whitespace()).collect();
    for decimal in ["4.6", "0.01"] {
        assert_eq!(
            compact.matches(decimal).count(),
            1,
            "all source decimal digits and punctuation present"
        );
    }
    assert!(
        compact.contains("t=2seconds"),
        "source exercise also retains its first time sample"
    );
    for decimal in ["4.6", "0.01"] {
        assert_eq!(
            text.matches(decimal).count(),
            1,
            "retain contiguous source decimal {decimal}, not spaced digits: {text}"
        );
    }
}

#[test]
fn should_keep_the_source_closed_loop_transition_as_body_text_2275() {
    let result = source_document(
        "arxiv-2403.01333v1-p4",
        "b6b67e0379852385254508c03e08e87d35b42348fa933e05e8bdb60c658ad215",
    );
    let document = result.document.as_ref().unwrap();
    let (_, transition) = unique_node(document, "We next present the result for");
    assert!(transition.content.text().unwrap().contains("optimal closed-loop"));
    assert!(
        source_text(document).contains("performance."),
        "the source sentence continuation is present"
    );
    assert!(
        matches!(&transition.content, NodeContent::Paragraph { .. }),
        "the visually ordinary source sentence must remain body text, got {:?}",
        transition.content
    );
}

#[test]
fn should_keep_source_paragraph_starts_separate_while_healing_following_2275() {
    let result = source_document(
        "gutenberg-33283-p27",
        "4d7fe54cedd4daf177144e6b8a7d863b7e071e637d259b4bb4006835c07cf2d3",
    );
    let document = result.document.as_ref().unwrap();
    let anchors = [
        "It will never do",
        "In case the reader",
        "Second differential coefficients",
    ];
    let nodes: Vec<_> = anchors.iter().map(|anchor| unique_node(document, anchor)).collect();
    let explanation = nodes[1].1.content.text().unwrap();
    assert!(
        explanation.contains("in the following way."),
        "preserve the proven ordinary line-wrap healing"
    );
    let derivative = nodes[2].1;
    let text = derivative.content.text().unwrap();
    let powers = derivative
        .annotations
        .iter()
        .filter(|annotation| {
            annotation.kind == AnnotationKind::Superscript
                && text.get(annotation.start as usize..annotation.end as usize) == Some("2")
        })
        .count();
    assert_eq!(powers, 2, "both source d-squared and dx-squared powers remain attached");
    let mut indices: Vec<_> = nodes.iter().map(|(index, _)| *index).collect();
    indices.sort_unstable();
    indices.dedup();
    assert_eq!(
        indices.len(),
        3,
        "three visually separate indented source paragraphs must not be merged"
    );
}

const CONTRIBUTION_ROWS: [[&str; 5]; 8] = [
    ["$3,600", "$1,243,900", "$10,900", "$385,200", "$581,500"],
    ["$7,400", "$2,900", "$400", "$3,714,500", "$632,500"],
    ["$391,000", "$467,400", "$56,800", "$5,800", "$405,200"],
    ["$733,200", "$8,000", "$468,700", "$75,200", "$41,000"],
    ["$13,300", "$9,500", "$953,800", "$1,113,500", "$1,109,300"],
    ["$353,900", "$986,100", "$88,600", "$378,200", "$13,200"],
    ["$3,800", "$745,100", "$5,800", "$3,072,100", "$1,626,700"],
    ["$512,900", "$2,309,200", "$6,600", "$202,400", "$15,800"],
];

#[test]
fn should_keep_eight_source_contribution_rows_with_five_values_each_2275() {
    let result = source_document(
        "openstax-introductory-business-statistics-1e-p377",
        "228e9bd948913791b9270f72e80d5d470e1181d93275877bab61fa156aa01567",
    );
    assert!(result.content.contains("Table 8.5"), "actual source table context");
    let start = result
        .content
        .find(CONTRIBUTION_ROWS[0][0])
        .expect("first source table value");
    let last = CONTRIBUTION_ROWS[7][4];
    let end = result.content.find(last).expect("last source table value") + last.len();
    let table = &result.content[start..end];
    let currency = Regex::new(r"\$[0-9][0-9,]*").unwrap();
    let values: Vec<_> = currency.find_iter(table).map(|value| value.as_str()).collect();
    let expected: Vec<_> = CONTRIBUTION_ROWS.iter().flatten().copied().collect();
    assert_eq!(values.len(), 40, "all forty source values actually extracted");
    assert_eq!(
        values, expected,
        "source value order and multiplicity before row assertions"
    );
    let rows: Vec<Vec<_>> = table
        .lines()
        .map(|line| currency.find_iter(line).map(|value| value.as_str()).collect::<Vec<_>>())
        .filter(|row| !row.is_empty())
        .collect();
    assert!(!rows.is_empty(), "actual numeric source rows observed");
    assert_eq!(
        rows.len(),
        8,
        "preserve the eight ruled source rows instead of one forty-value paragraph"
    );
    for (actual, expected) in rows.iter().zip(CONTRIBUTION_ROWS) {
        assert_eq!(
            actual.as_slice(),
            &expected[..],
            "five source values per numeric row; no table markup required"
        );
    }
}
