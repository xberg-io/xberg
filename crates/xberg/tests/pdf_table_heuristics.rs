//! Regression tests for xberg-io/xberg#1223: the text-layer table heuristic must
//! accept a dense unruled ledger that its density guard wrongly rejected (row
//! count alone must not disqualify a real table), while still rejecting genuine
//! multi-column prose.
//!
//! An all-text roster is deliberately NOT recovered: a roster and a scanned
//! multi-column prose page (nougat pattern) are indistinguishable to the
//! row-coherence guard, so the conservative alpha-ratio check is kept and both
//! stay rejected — precision over recall for the ambiguous all-text case.
//!
//! These build unruled (text-only, no ruling lines) synthetic PDFs so they
//! exercise the heuristic fallback tier, not the ruled-table detectors.

#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use pdf_oxide::geometry::Rect;
use pdf_oxide::writer::{DocumentBuilder, TextAlign};
use xberg::core::config::ExtractionConfig;

fn text_pdf(rows: &[Vec<(f32, f32, String)>]) -> Vec<u8> {
    let mut doc = DocumentBuilder::new();
    let mut page = doc.a4_page();
    let (top, row_h) = (760.0_f32, 16.0_f32);
    for (i, row) in rows.iter().enumerate() {
        let y = top - row_h * i as f32;
        for (x, w, text) in row {
            page = page.text_in_rect(Rect::new(*x, y, *w, row_h), text, TextAlign::Left);
        }
    }
    page.done();
    doc.build().expect("build pdf")
}

fn table_count(bytes: &[u8]) -> usize {
    extract_bytes_document_blocking(bytes, "application/pdf", &ExtractionConfig::default())
        .expect("extraction must succeed")
        .tables
        .len()
}

/// A dense 3-column unruled ledger (Account | Amount | Note, 30 rows). Real
/// table; the density guard rejected it before.
#[test]
fn dense_unruled_ledger_is_detected() {
    let cols = [(50.0_f32, 150.0_f32), (210.0, 90.0), (310.0, 200.0)];
    let mut rows = vec![vec![
        (cols[0].0, cols[0].1, "Account".to_string()),
        (cols[1].0, cols[1].1, "Amount".to_string()),
        (cols[2].0, cols[2].1, "Note".to_string()),
    ]];
    for n in 1..=30 {
        rows.push(vec![
            (cols[0].0, cols[0].1, format!("Account {n:04}")),
            (cols[1].0, cols[1].1, format!("${}.00", n * 137)),
            (cols[2].0, cols[2].1, format!("ref {n}")),
        ]);
    }
    assert!(table_count(&text_pdf(&rows)) >= 1, "a 30-row 3-column ledger must be detected as a table");
}

/// An all-text roster (Name | City | Role). Ambiguous against scanned columned
/// prose, so it is conservatively NOT detected — the alpha-ratio guard that
/// protects against the nougat pattern also catches this. This pins that the
/// ledger relaxation does not accidentally reopen the all-text prose hole.
#[test]
fn all_text_roster_is_conservatively_rejected() {
    let cols = [(50.0_f32, 150.0_f32), (210.0, 120.0), (340.0, 160.0)];
    let people = [
        ("Alice Johnson", "New York", "Manager"),
        ("Bob Smith", "Chicago", "Analyst"),
        ("Carol White", "Boston", "Director"),
        ("David Brown", "Seattle", "Engineer"),
        ("Eve Davis", "Austin", "Designer"),
        ("Frank Moore", "Denver", "Recruiter"),
    ];
    let mut rows = vec![vec![
        (cols[0].0, cols[0].1, "Name".to_string()),
        (cols[1].0, cols[1].1, "City".to_string()),
        (cols[2].0, cols[2].1, "Role".to_string()),
    ]];
    for (name, city, role) in people {
        rows.push(vec![
            (cols[0].0, cols[0].1, name.to_string()),
            (cols[1].0, cols[1].1, city.to_string()),
            (cols[2].0, cols[2].1, role.to_string()),
        ]);
    }
    assert_eq!(
        table_count(&text_pdf(&rows)),
        0,
        "an all-text roster stays rejected — indistinguishable from columned prose"
    );
}

/// Two-column prose (an article laid out in columns). Must NOT be a table —
/// the guard against columned prose must still fire.
#[test]
fn columned_prose_is_not_a_table() {
    let left = [
        "The quick brown fox jumps over",
        "the lazy dog and then continues",
        "running across the wide green",
        "field toward the distant hills",
        "where the sun was slowly setting",
        "behind the ancient oak trees that",
    ];
    let right = [
        "In addition to that it should be",
        "noted that the weather was quite",
        "pleasant throughout the entire",
        "afternoon which made the long",
        "walk considerably more enjoyable",
        "for everyone who was present there",
    ];
    let rows: Vec<Vec<(f32, f32, String)>> = (0..left.len())
        .map(|i| {
            vec![
                (50.0_f32, 240.0_f32, left[i].to_string()),
                (300.0, 240.0, right[i].to_string()),
            ]
        })
        .collect();
    assert_eq!(table_count(&text_pdf(&rows)), 0, "columned prose must not be detected as a table");
}
