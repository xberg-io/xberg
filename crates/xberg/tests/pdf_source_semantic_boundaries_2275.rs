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

#[test]
fn should_keep_the_source_remark_continuation_out_of_document_headings_2275() {
    let result = source_document(
        "arxiv-2403.01333v1-p4",
        "b6b67e0379852385254508c03e08e87d35b42348fa933e05e8bdb60c658ad215",
    );
    let document = result.document.as_ref().unwrap();
    let (_, remark) = unique_node(document, "Remark 3.");
    let (_, continuation) = unique_node(document, "feedback controller is able to achieve");
    let (_, ending) = unique_node(document, "performance in terms of the closed-loop systems");
    assert!(ending.content.text().unwrap().contains("norms."));
    let remark_text = remark.content.text().unwrap();
    assert!(
        remark.annotations.iter().any(|annotation| {
            matches!(annotation.kind, AnnotationKind::Bold | AnnotationKind::Italic)
                && remark_text
                    .get(annotation.start as usize..annotation.end as usize)
                    .is_some_and(|text| text.contains("Remark 3."))
        }),
        "actual nonempty styled source context, not an unmatched paragraph anchor"
    );
    let (_, previous_control) = unique_node(document, "We next present the result for");
    assert!(matches!(previous_control.content, NodeContent::Paragraph { .. }));
    assert!(
        matches!(continuation.content, NodeContent::Paragraph { .. }),
        "the continuous source Remark 3 sentence must not become a heading: {:?}",
        continuation.content
    );
}

#[test]
fn should_keep_the_source_algebraic_definition_continuation_in_one_paragraph_2275() {
    let result = source_document(
        "gutenberg-38769-p74",
        "8843c419ae6e36aa019af4b6a1155bf798fd589463df572b188f88bc402f6b77",
    );
    let document = result.document.as_ref().unwrap();
    let (definition_index, definition) = unique_node(document, "will be said to be an algebraical function");
    let (coefficient_index, coefficient) = unique_node(document, "coefficients are rational functions of x");
    assert!(definition.content.text().unwrap().contains("a function y = f(x)"));
    assert!(
        coefficient
            .content
            .text()
            .unwrap()
            .contains("There is plainly no loss of generality")
    );
    assert!(matches!(definition.content, NodeContent::Paragraph { .. }));
    assert!(matches!(coefficient.content, NodeContent::Paragraph { .. }));
    assert_eq!(
        definition_index, coefficient_index,
        "the source's italic definition continues through its coefficient clause without a new paragraph"
    );
}

#[test]
fn should_keep_the_source_t_score_explanation_continuation_in_one_paragraph_2275() {
    let result = source_document(
        "openstax-introductory-business-statistics-1e-p362",
        "dcf927a2900537d64df1b3484af3894d88c38fdbbc05aa82957d228c431016c4",
    );
    let document = result.document.as_ref().unwrap();
    let (formula_index, formula) = unique_node(document, "which measures");
    let (explanation_index, explanation) = unique_node(document, "how far away a measure is from the population mean");
    let (_, standard_deviation) = unique_node(document, "standard deviation, and");
    assert!(
        standard_deviation
            .content
            .text()
            .unwrap()
            .contains("is the sample size")
    );
    assert!(formula.content.text().unwrap().contains("is the formula for the"));
    assert!(explanation.content.text().unwrap().contains("Student's t-distribution"));
    assert!(matches!(formula.content, NodeContent::Paragraph { .. }));
    assert!(matches!(explanation.content, NodeContent::Paragraph { .. }));
    assert_eq!(
        formula_index, explanation_index,
        "how far away continues the printed t-score definition rather than starting a new paragraph"
    );
}

fn table_source_text(document: &DocumentStructure) -> String {
    document
        .nodes
        .iter()
        .flat_map(|node| match &node.content {
            NodeContent::Table { grid } => grid.cells.iter().map(|cell| cell.content.as_str()).collect(),
            content => content.text().into_iter().collect::<Vec<_>>(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn should_keep_source_sample_size_table_labels_out_of_document_headings_2275() {
    let result = source_document(
        "openstax-introductory-business-statistics-1e-p412",
        "56ea62a1dcc0be8bbfcf59597da6f68060524df68f083ebb30c550f0c53f7d5e",
    );
    let document = result.document.as_ref().unwrap();
    let (_, distribution) = unique_node(document, "9.3 Distribution Needed for Hypothesis Testing");
    let (_, hypotheses) = unique_node(document, "9.1 Null and Alternative Hypotheses");
    assert!(matches!(distribution.content, NodeContent::Heading { .. }));
    assert!(matches!(hypotheses.content, NodeContent::Heading { .. }));
    let text = table_source_text(document);
    let cells = text
        .lines()
        .filter(|line| !line.starts_with("Table 9.6 Test Statistics"))
        .collect::<Vec<_>>()
        .join("\n");
    for label in ["Sample Size", "Test Statistic"] {
        assert_eq!(
            cells.matches(label).count(),
            2,
            "both actual source tables retain {label}"
        );
    }
    assert_eq!(
        text.matches("Table 9.6 Test Statistics for Test").count(),
        2,
        "both actual source table captions are present before heading assertions"
    );
    let promoted_labels: Vec<_> = document
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Heading { text, .. } if text.contains("Sample Size") || text.contains("Test Statistic") => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        promoted_labels,
        Vec::<&str>::new(),
        "table column labels have no source document-heading role"
    );
}
