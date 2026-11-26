use super::*;

use crate::pdf::table_reconstruct::HocrWord;

const WORD_HEIGHT: u32 = 8;
const HEADER_TOP: u32 = 90;
const BODY_TOP: u32 = 108;
const BODY_PITCH: u32 = 10;
const BODY_ROWS: u32 = 8;
const RULE_Y: f32 = 100.0;

fn word(text: &str, left: u32, top: u32, width: u32) -> HocrWord {
    HocrWord {
        text: text.to_string(),
        left,
        top,
        width,
        height: WORD_HEIGHT,
        confidence: 95.0,
    }
}

/// Two side-by-side holdings schedules under one `Security | Shares | Value` label row whose centre
/// sits 18 points above the first body row's at an 8-point word height, beyond the 15.4-point region
/// split gap (pb_fqr-retail-blackrock-global-allocation-fund-inc_page8).
fn holdings_body() -> Vec<HocrWord> {
    let mut words = Vec::new();
    for row in 0..BODY_ROWS {
        let top = BODY_TOP + row * BODY_PITCH;
        for offset in [0, 291] {
            // A security name and its dot leader are several text words, so the rows are not
            // numeric-dense and `attach_aligned_numeric_headers` leaves the label row alone.
            words.push(word(&format!("Holding{row}"), 18 + offset, top, 40));
            words.push(word("Corp.", 60 + offset, top, 20));
            words.push(word("..........", 82 + offset, top, 36));
            words.push(word(&format!("{},800", 100 + row), 196 + offset, top, 28));
            words.push(word(&format!("3,{},671", 200 + row), 253 + offset, top, 36));
        }
    }
    words
}

fn label_row() -> Vec<HocrWord> {
    [0, 291]
        .into_iter()
        .flat_map(|offset| {
            [
                word("Security", 18 + offset, HEADER_TOP, 30),
                word("Shares", 199 + offset, HEADER_TOP, 22),
                word("Value", 269 + offset, HEADER_TOP, 20),
            ]
        })
        .collect()
}

fn rule_under_labels() -> Vec<HeaderRule> {
    [18.0, 309.0]
        .into_iter()
        .map(|left| HeaderRule {
            y: RULE_Y,
            left,
            right: left + 276.0,
        })
        .collect()
}

fn regions_holding(regions: &[Vec<HocrWord>], text: &str) -> Vec<usize> {
    regions
        .iter()
        .enumerate()
        .filter(|(_, region)| region.iter().any(|word| word.text == text))
        .map(|(index, _)| index)
        .collect()
}

#[test]
fn a_ruled_label_row_joins_the_table_regions_it_heads() {
    let mut words = holdings_body();
    words.extend(label_row());

    let regions = cluster_words_into_vertical_regions_with_rules(&words, &rule_under_labels());

    for label in ["Security", "Shares", "Value"] {
        let owners = regions_holding(&regions, label);
        assert!(!owners.is_empty(), "{label} must join a table region");
        for owner in owners {
            assert!(
                regions[owner].iter().any(|word| word.text.starts_with("Holding")),
                "{label} must join a region holding the body rows"
            );
        }
    }
}

#[test]
fn a_label_row_without_a_rule_stays_out_of_the_table() {
    let mut words = holdings_body();
    words.extend(label_row());

    let regions = cluster_words_into_vertical_regions_with_rules(&words, &[]);

    assert!(regions_holding(&regions, "Security").is_empty());
    assert!(!regions_holding(&regions, "Holding0").is_empty());
}

#[test]
fn a_ruled_heading_over_one_column_is_not_a_header_row() {
    let mut words = holdings_body();
    words.extend([word("Common", 18, HEADER_TOP, 30), word("Stocks", 50, HEADER_TOP, 26)]);

    let regions = cluster_words_into_vertical_regions_with_rules(&words, &rule_under_labels());

    assert!(regions_holding(&regions, "Common").is_empty());
}
