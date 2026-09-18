//! Regression coverage for xberg-io/xberg#1649.
//!
//! The Enterprise-bundled bank statement sample (`bank_statement_example.png`, SHA256
//! `266b343934e7d55839b4ea37ff92a45b7f5613a3861c6f520411ae2f8336bc66`), extracted as a
//! standalone image with the default config, lost the statement period and the
//! transaction table:
//!
//! - The "STATEMENT PERIOD" summary card's two-word header split across two table
//!   columns, dragging its value ("Jan 1 - Jan 31, 2026") apart into two cells with it
//!   (a stray hyphen glyph's bounding box left a gap just over the cell-merge
//!   threshold, minting a spurious column between "STATEMENT" and "PERIOD").
//! - The transaction table's WITHDRAWAL amounts are right-aligned, so their left edges
//!   vary by digit count (`$1,250.00` vs `$87.32`); that variance split WITHDRAWAL into
//!   two OCR table columns, one carrying the header and the other carrying most of the
//!   values, corrupting every row that had a withdrawal.
//! - A "TRANSACTIONS" section caption sat close enough above the real header row to
//!   join the same table region, so the reconstructed table's first row was the
//!   caption, not the header.
//!
//! This fixture is the public bank-statement-preset sample shipped from
//! `xberg-io/xberg-enterprise` (`crates/presets/library/bank_statement/samples/`), not
//! a real customer document.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: org logging policy exempts tests
#![cfg(feature = "ocr")]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::core::config::ExtractionConfig;

const BANK_STATEMENT_SAMPLE: &[u8] = include_bytes!("fixtures/ocr/bank_statement_example.png");
const BANK_STATEMENT_SAMPLE_SHA256: &str = "266b343934e7d55839b4ea37ff92a45b7f5613a3861c6f520411ae2f8336bc66";

const EXPECTED_STATEMENT_PERIOD: &str = "Jan 1 - Jan 31, 2026";
const EXPECTED_HEADERS: [&str; 5] = ["DATE", "DESCRIPTION", "WITHDRAWAL", "DEPOSIT", "BALANCE"];
const EXPECTED_TRANSACTION_ROWS: [[&str; 5]; 9] = [
    ["2026-01-02", "Opening Balance", "", "", "$10,500.00"],
    [
        "2026-01-05",
        "ACH Deposit - EMPLOYER INC",
        "",
        "$2,500.00",
        "$13,000.00",
    ],
    ["2026-01-08", "Check #1042", "$1,250.00", "", "$11,750.00"],
    [
        "2026-01-12",
        "Debit Card Purchase - GROCERY",
        "$87.32",
        "",
        "$11,662.68",
    ],
    ["2026-01-15", "Wire Transfer (Outgoing)", "$3,000.00", "", "$8,662.68"],
    ["2026-01-18", "ATM Withdrawal", "$500.00", "", "$8,162.68"],
    ["2026-01-22", "ACH Deposit - CONSULTING", "", "$5,000.00", "$13,162.68"],
    ["2026-01-25", "Monthly Service Fee", "$15.00", "", "$13,147.68"],
    [
        "2026-01-28",
        "Debit Card Purchase - UTILITIES",
        "$300.00",
        "",
        "$12,847.68",
    ],
];

fn assert_sample_bytes_are_unchanged() {
    let digest = <sha2::Sha256 as sha2::Digest>::digest(BANK_STATEMENT_SAMPLE);
    let hex_digest = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    assert_eq!(
        hex_digest, BANK_STATEMENT_SAMPLE_SHA256,
        "the regression must exercise the exact sample reported in issue 1649"
    );
}

#[test]
fn bank_statement_image_keeps_period_and_transaction_rows() {
    assert_sample_bytes_are_unchanged();

    let config = ExtractionConfig::default();
    let document = extract_bytes_document_blocking(BANK_STATEMENT_SAMPLE, "image/png", &config)
        .expect("the bundled bank statement sample must extract with the default config");

    let period_is_one_cell = document
        .tables
        .iter()
        .flat_map(|table| table.cells.iter().flatten())
        .any(|cell| cell.contains(EXPECTED_STATEMENT_PERIOD));
    assert!(
        period_is_one_cell,
        "the statement period {EXPECTED_STATEMENT_PERIOD:?} must land contiguously in one table \
         cell, not split across cells; tables were: {:#?}",
        document.tables
    );

    let transaction_table = document
        .tables
        .iter()
        .find(|table| {
            table
                .cells
                .first()
                .is_some_and(|row| row.iter().map(String::as_str).eq(EXPECTED_HEADERS.iter().copied()))
        })
        .unwrap_or_else(|| {
            panic!(
                "the bank statement sample must produce a five-column transaction table with \
                 headers {EXPECTED_HEADERS:?}; tables were: {:#?}",
                document.tables
            )
        });

    let actual_rows = transaction_table
        .cells
        .iter()
        .skip(1)
        .map(|row| row.iter().map(String::as_str).collect::<Vec<_>>())
        .collect::<Vec<_>>();

    assert_eq!(
        actual_rows, EXPECTED_TRANSACTION_ROWS,
        "the extractor must retain all nine transaction rows and their amounts in the right columns"
    );
}
