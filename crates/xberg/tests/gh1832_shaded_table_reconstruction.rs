//! Ground-truth measurement for the scanned-table reconstruction cluster: GH#1832 (a two-word
//! header splits into two columns), GH#1833 (several values glue into one cell across the
//! shading's underscore-like marks) and GH#1834 (the tail of a multi-word row label becomes a
//! row of its own).
//!
//! All three were reported against `shaded_table_scan.pdf` and all three are properties of the
//! same reconstruction, so they are measured together against one ground truth rather than
//! asserted one symptom at a time. The fixture is synthetic -- a generated six-year summary with
//! shaded subtotal rows -- so every cell's correct value is known exactly and the metric can be
//! an absolute count of correct values recovered, never a ratio over output whose length these
//! changes alter.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: org logging policy exempts tests
#![cfg(all(feature = "ocr", feature = "pdf"))]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::types::TesseractConfig;

const SCANNED_TABLE: &[u8] = include_bytes!("fixtures/ocr/shaded_table_scan.pdf");

/// The fixture's 23 data rows, label first then Year 1 through Year 6, read off the rendered
/// page. Every value is printed in full on the page, so a miss is a reconstruction defect and
/// never an ambiguity in the source. ~keep
const GROUND_TRUTH: &[[&str; 7]] = &[
    ["APPLES", "48,210", "49,850", "51,344", "52,885", "54,471", "56,105"],
    ["PEARS", "21,406", "21,977", "22,636", "23,315", "24,015", "24,735"],
    ["PLUMS AND FIGS", "7,812", "7,968", "8,127", "8,290", "8,456", "8,625"],
    ["GRAPES", "3,104", "3,197", "3,293", "3,391", "3,493", "3,598"],
    [
        "SUBTOTAL FRUIT",
        "80,532",
        "82,992",
        "85,400",
        "87,881",
        "90,435",
        "93,063",
    ],
    [
        "MELONS AND BERRIES",
        "5,103",
        "5,256",
        "5,413",
        "5,576",
        "5,743",
        "5,915",
    ],
    ["TRANSFERS IN", "3,250", "3,250", "3,250", "3,250", "3,250", "3,250"],
    [
        "TOTAL PRODUCE",
        "88,885",
        "91,498",
        "94,063",
        "96,707",
        "99,428",
        "102,228",
    ],
    ["BREAD", "41,920", "43,177", "44,472", "45,806", "47,180", "48,595"],
    ["CHEESE", "18,402", "19,138", "19,903", "20,699", "21,527", "22,388"],
    [
        "SPOILAGE", "(2,100)", "(2,163)", "(2,227)", "(2,294)", "(2,363)", "(2,434)",
    ],
    [
        "SUBTOTAL DAIRY",
        "58,222",
        "60,152",
        "62,148",
        "64,211",
        "66,344",
        "68,549",
    ],
    ["SALT", "4,871", "5,017", "5,167", "5,322", "5,481", "5,645"],
    ["PEPPER", "19,334", "19,914", "20,511", "21,126", "21,759", "22,411"],
    ["TRANSFERS OUT", "6,102", "6,285", "6,473", "6,667", "6,867", "7,073"],
    [
        "SUBTOTAL OTHER GOODS",
        "30,307",
        "31,216",
        "32,151",
        "33,115",
        "34,107",
        "35,129",
    ],
    [
        "TOTAL GOODS",
        "88,529",
        "91,368",
        "94,299",
        "97,326",
        "100,451",
        "103,678",
    ],
    ["TIMBER", "1,500", "1,545", "1,591", "1,639", "1,688", "1,739"],
    [
        "TOTAL NON GOODS",
        "(1,200)",
        "(1,236)",
        "(1,273)",
        "(1,311)",
        "(1,350)",
        "(1,391)",
    ],
    [
        "OPENING STOCK BALANCE",
        "30,118",
        "30,474",
        "30,604",
        "30,368",
        "29,749",
        "28,726",
    ],
    [
        "NET CHANGE (DEFICIT)",
        "356",
        "130",
        "(236)",
        "(619)",
        "(1,023)",
        "(1,450)",
    ],
    ["EXPECTED SURPLUS", "1,012", "1,042", "1,073", "1,105", "1,138", "1,172"],
    [
        "CLOSING STOCK BALANCE",
        "30,474",
        "30,604",
        "30,368",
        "29,749",
        "28,726",
        "27,276",
    ],
];

fn config_with_psm(psm: i32, shaded: bool) -> ExtractionConfig {
    let mut preprocessing = xberg::types::ImagePreprocessingConfig {
        normalize_shaded_rows: shaded,
        ..Default::default()
    };
    preprocessing.normalize_shaded_rows = shaded;
    ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            tesseract_config: Some(TesseractConfig {
                psm: Some(psm),
                use_cache: false,
                enable_table_detection: true,
                preprocessing: Some(preprocessing),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// How many of the 138 ground-truth values land in the correct row, in the correct column.
/// Position matters: a value recovered into the wrong column is a reconstruction failure, and a
/// metric that only asked "does this string appear somewhere" would score GH#1833's glued cell
/// as a success.
fn correct_values_in_place(table: &xberg::types::Table) -> (usize, Vec<String>) {
    let mut correct = 0;
    let mut misses = Vec::new();
    for truth in GROUND_TRUTH {
        let label = truth[0];
        let Some(row) = table
            .cells
            .iter()
            .find(|row| row.first().is_some_and(|cell| cell.trim() == label))
        else {
            misses.push(format!("{label}: row absent"));
            continue;
        };
        for (offset, expected) in truth[1..].iter().enumerate() {
            match row.get(offset + 1) {
                Some(actual) if actual.trim() == *expected => correct += 1,
                Some(actual) => misses.push(format!("{label} col{}: want {expected:?} got {actual:?}", offset + 1)),
                None => misses.push(format!("{label} col{}: want {expected:?} got <short row>", offset + 1)),
            }
        }
    }
    (correct, misses)
}

/// Extract once and return the first reconstructed table, or `None` if the page produced none.
fn first_table(psm: i32, shaded: bool) -> Option<xberg::types::Table> {
    let document = extract_bytes_document_blocking(SCANNED_TABLE, "application/pdf", &config_with_psm(psm, shaded))
        .expect("forced OCR of the scanned table must succeed");
    document.tables.into_iter().next()
}

/// The page must come back with a table at all.
///
/// Before the header-less sparse-column fold in `pdf::table_reconstruct`, it did not -- at any
/// segmentation mode, with or without shaded-row normalisation. The grid was reconstructed
/// correctly (23 rows) and then discarded wholesale by the `column_sparsity` gate, because a
/// misread shaded row contributed three stray glyphs that minted a phantom header-less column
/// which was 19/22 empty. Three stray cells cost the entire table (xberg-io/xberg#1797).
#[test]
fn the_scanned_table_is_not_discarded_over_a_phantom_column() {
    for psm in [3, 11] {
        let table = first_table(psm, false)
            .unwrap_or_else(|| panic!("PSM {psm} produced no table at all; the sparsity gate discarded it"));
        assert!(
            table.cells.len() >= 20,
            "PSM {psm}: the 23-row grid must survive, got {} rows",
            table.cells.len()
        );
    }
}

/// An absolute count of ground-truth values recovered into the correct row and column, which is
/// the metric that decides whether a reconstruction change helped. Never a ratio: these changes
/// alter the grid's width, and a ratio over a narrower grid rises when values are lost.
///
/// The floor is deliberately well under the measured value so ordinary OCR jitter does not fail
/// the build; what it pins is the order of magnitude, against the 0 this fixture produced when
/// no table was built at all. Measured: 102 at PSM 11 and 98 at PSM 3, from 36 and 18 before the
/// split-header column was folded away (GH#1832). Every one of the 36 remaining misses is a row
/// whose *label* OCR mangled (`[TOTAL GOODS`, `EE NET CHANGE (DEFICIT)`), so the harness cannot
/// match the row at all -- no correctly-labelled row has a value in the wrong cell.
#[test]
fn enough_ground_truth_values_land_in_the_right_cell() {
    let table = first_table(11, false).expect("PSM 11 must produce a table");
    let (correct, misses) = correct_values_in_place(&table);
    assert!(
        correct >= 80,
        "only {correct} of {} ground-truth values landed in the right cell (floor 80);          first misses: {:?}",
        GROUND_TRUTH.len() * 6,
        misses.iter().take(8).collect::<Vec<_>>()
    );
}

/// GH#1832 specifically: each "Year N" header must be one column, not two.
///
/// This is the defect the value count above is dominated by rather than a separate symptom. OCR
/// splits the header across two x-tracks, each mints a column, and the right-hand one holds a
/// header fragment and no data -- so every value in the table sits one or more places left of the
/// column it belongs to while the OCR itself read it correctly.
#[test]
fn each_year_header_occupies_exactly_one_column() {
    for psm in [3, 11] {
        let table = first_table(psm, false).unwrap_or_else(|| panic!("PSM {psm} must produce a table"));
        let header = table.cells.first().expect("the table must have a header row");
        let years: Vec<&str> = header.iter().skip(1).take(6).map(String::as_str).collect();
        assert_eq!(
            years,
            ["Year 1", "Year 2", "Year 3", "Year 4", "Year 5", "Year 6"],
            "PSM {psm}: the six year headers must each occupy one column; whole header: {header:?}"
        );
    }
}

/// Measurement harness for the rest of the cluster -- GH#1833 (values glue across the shading's
/// underscore marks) and GH#1834 (a label's tail becomes its own row); GH#1832's split header is
/// fixed and gated above. This prints the full grid and the per-cell misses at four
/// configurations so a change to the remaining two can be scored against ground truth.
/// Ignored because it is a report, not a gate.
#[test]
#[ignore = "measurement report for GH#1832/1833/1834; run with --ignored --nocapture"]
fn measure_shaded_table_reconstruction() {
    for psm in [3, 11] {
        for shaded in [false, true] {
            let Some(table) = first_table(psm, shaded) else {
                println!("=== PSM {psm} shaded={shaded}: NO TABLE ===");
                continue;
            };
            let (correct, misses) = correct_values_in_place(&table);
            println!(
                "=== PSM {psm} shaded={shaded}: {correct} of {} values in place, {} rows x {} cols ===",
                GROUND_TRUTH.len() * 6,
                table.cells.len(),
                table.cells.first().map_or(0, Vec::len)
            );
            for row in &table.cells {
                println!("  {row:?}");
            }
            for miss in misses.iter().take(25) {
                println!("  MISS {miss}");
            }
        }
    }
}
