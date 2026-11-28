use super::chpx::{Script, ScriptIndex};
use crate::extractors::security::{SecurityBudget, SecurityLimits};

const PAIRS: usize = 16;

fn fixture(properties: &[&[u8]]) -> (Vec<u8>, Vec<u8>) {
    let mut word = vec![0; 2048];
    word[PAIRS - 2..PAIRS].copy_from_slice(&93u16.to_le_bytes());
    let pair = PAIRS + 12 * 8;
    word[pair..pair + 4].copy_from_slice(&0u32.to_le_bytes());
    word[pair + 4..pair + 8].copy_from_slice(&12u32.to_le_bytes());
    let mut table = Vec::new();
    for value in [100u32, 100 + 2 * properties.len() as u32, 3] {
        table.extend_from_slice(&value.to_le_bytes());
    }
    let fkp = &mut word[1536..2048];
    for index in 0..=properties.len() {
        fkp[index * 4..index * 4 + 4].copy_from_slice(&(100 + 2 * index as u32).to_le_bytes());
    }
    let mut position = 256;
    for (index, property) in properties.iter().enumerate() {
        fkp[4 * (properties.len() + 1) + index] = (position / 2) as u8;
        fkp[position] = property.len() as u8;
        fkp[position + 1..position + 1 + property.len()].copy_from_slice(property);
        position = (position + 2 + property.len()) & !1;
    }
    fkp[511] = properties.len() as u8;
    (word, table)
}

fn read(word: &[u8], table: &[u8]) -> (ScriptIndex, usize) {
    let mut budget = SecurityBudget::with_defaults();
    let mut warnings = Vec::new();
    let index = ScriptIndex::read(word, table, PAIRS, &mut budget, &mut warnings).expect("bounded CHPX");
    (index, warnings.len())
}

#[test]
fn should_reset_script_and_support_queries_in_physical_fc_reverse_order() {
    let (word, table) = fixture(&[&[0x48, 0x2a, 1], &[0x48, 0x2a, 0], &[0x48, 0x2a, 2]]);
    let (index, warnings) = read(&word, &table);
    assert_eq!(warnings, 0);
    assert_eq!(index.at(105), Script::Subscript);
    assert_eq!(index.at(103), Script::Normal);
    assert_eq!(index.at(101), Script::Superscript);
    assert_eq!(index.at(100), Script::Normal);
    assert_eq!(index.at(107), Script::Normal);
}

#[test]
fn should_limit_stale_fkp_runs_to_the_authoritative_bte_interval() {
    let (word, mut table) = fixture(&[&[0x48, 0x2a, 1][..]; 3]);
    table[..4].copy_from_slice(&102u32.to_le_bytes());
    table[4..8].copy_from_slice(&104u32.to_le_bytes());
    let (index, warnings) = read(&word, &table);
    assert_eq!(warnings, 0);
    assert_eq!(index.at(101), Script::Normal);
    assert_eq!(index.at(103), Script::Superscript);
    assert_eq!(index.at(105), Script::Normal);
}

#[test]
fn should_preserve_last_explicit_script_and_skip_unrelated_sprm_operands() {
    let (word, table) = fixture(&[&[0x03, 0x6a, 0x48, 0x2a, 1, 0, 0x48, 0x2a, 2, 0x48, 0x2a, 0]]);
    let (index, warnings) = read(&word, &table);
    assert_eq!(warnings, 0);
    assert_eq!(index.at(101), Script::Normal);
}

#[test]
fn should_ignore_script_opcode_bytes_inside_an_unrelated_operand() {
    let (word, table) = fixture(&[&[0x03, 0x6a, 0x48, 0x2a, 1, 0]]);
    let (index, warnings) = read(&word, &table);
    assert_eq!(warnings, 0);
    assert_eq!(index.at(101), Script::Normal);
}

#[test]
fn should_warn_and_drop_malformed_or_truncated_optional_chpx() {
    for case in 0..5 {
        let (mut word, mut table) = fixture(&[&[0x48, 0x2a, 1]]);
        match case {
            0 => table[4..8].copy_from_slice(&100u32.to_le_bytes()),
            1 => word.truncate(1800),
            2 => word[2047] = 255,
            3 => word[1792] = 2,
            _ => table.truncate(11),
        }
        let (index, warnings) = read(&word, &table);
        assert_eq!(warnings, 1, "malformed case {case}");
        assert_eq!(index.at(101), Script::Normal, "malformed case {case}");
    }
}

#[test]
fn should_ignore_undeclared_chpx_pair_without_reading_stale_fib_bytes() {
    let (mut word, table) = fixture(&[&[0x48, 0x2a, 1]]);
    word[PAIRS - 2..PAIRS].copy_from_slice(&12u16.to_le_bytes());
    let (index, warnings) = read(&word, &table);
    assert_eq!(warnings, 0);
    assert_eq!(index.at(101), Script::Normal);
}

#[test]
fn should_return_budget_exhaustion_instead_of_silently_discarding_properties() {
    let (word, table) = fixture(&[&[0x48, 0x2a, 1]]);
    let mut budget = SecurityBudget::from_limits(&SecurityLimits {
        max_iterations: 0,
        ..Default::default()
    });
    let error = ScriptIndex::read(&word, &table, PAIRS, &mut budget, &mut Vec::new())
        .err()
        .expect("iteration limit");
    assert!(error.to_string().contains("iteration"), "{error}");
}

#[test]
fn should_bound_normalized_text_without_images() {
    let mut budget = SecurityBudget::from_limits(&SecurityLimits {
        max_content_size: 1,
        ..Default::default()
    });
    let error = super::chpx::normalize("AB", &[Script::Normal; 2], &mut budget)
        .expect_err("normalized DOC text exceeds configured content cap");
    assert!(error.to_string().contains("Content too large"), "{error}");
}

#[test]
fn should_coalesce_and_trim_script_annotations_in_normalized_utf8_bytes() {
    let mut budget = SecurityBudget::with_defaults();
    let (text, annotations) = super::chpx::normalize("  é9\x0b\n\nH2O  ", &[Script::Superscript; 12], &mut budget)
        .expect("normalized scripts");
    assert_eq!(text, "é9\n\nH2O");
    assert_eq!(annotations, [crate::types::builder::superscript(0, 8)]);
}

#[cfg(feature = "tokio-runtime")]
#[tokio::test]
async fn should_apply_configured_doc_iteration_limit_when_images_are_disabled() {
    use crate::core::config::ExtractionConfig;
    use crate::plugins::InternalDocumentExtractor;
    let config = ExtractionConfig {
        use_cache: false,
        security_limits: Some(SecurityLimits {
            max_iterations: 0,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(!config.needs_image_data());
    let bytes = super::test_scripts::script_doc(&[("10", 0), ("9", 1), ("/l\r", 0)], false);
    let result = crate::extractors::doc::DocExtractor::new()
        .extract_content(&bytes, "application/msword", &config)
        .await;
    let error = result.expect_err("DOC iteration limit must reach parser without images");
    assert!(error.to_string().contains("iteration"), "{error}");
}

fn two_page_fixture() -> (Vec<u8>, Vec<u8>) {
    let (mut word, _) = fixture(&[&[0x48, 0x2a, 1]]);
    let first_page = word[1536..2048].to_vec();
    word.extend_from_slice(&first_page);
    word[2048..2052].copy_from_slice(&102u32.to_le_bytes());
    word[2052..2056].copy_from_slice(&104u32.to_le_bytes());
    word[2048 + 259] = 2;
    word[PAIRS + 12 * 8 + 4..PAIRS + 12 * 8 + 8].copy_from_slice(&20u32.to_le_bytes());
    let table = [100u32, 102, 104, 3, 4]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    (word, table)
}

#[test]
fn should_resolve_two_authoritative_bte_rows_across_distinct_fkp_pages() {
    let (word, table) = two_page_fixture();
    assert_eq!((table.len() - 4) / 8, 2);
    let (index, warnings) = read(&word, &table);
    assert_eq!(warnings, 0);
    assert_eq!(index.at(103), Script::Subscript);
    assert_eq!(index.at(101), Script::Superscript);
    assert_eq!(index.at(102), Script::Superscript);
    assert_eq!(index.at(104), Script::Subscript);
    assert_eq!(index.at(105), Script::Normal);
}

#[test]
fn should_discard_a_two_row_bte_with_a_descending_second_interval() {
    let (word, mut table) = two_page_fixture();
    table[4..8].copy_from_slice(&104u32.to_le_bytes());
    table[8..12].copy_from_slice(&102u32.to_le_bytes());
    assert_eq!((table.len() - 4) / 8, 2);
    let (index, warnings) = read(&word, &table);
    assert_eq!(warnings, 1);
    assert_eq!(index.at(101), Script::Normal);
    assert_eq!(index.at(103), Script::Normal);
}

#[test]
fn should_read_tracked_deletion_from_chpx_runs_and_piece_property_groups() {
    use super::chpx::piece_deleted_property;
    let (word, table) = fixture(&[
        &[0x00, 0x08, 1],
        &[0x00, 0x08, 0x81],
        &[0x00, 0x08, 0],
        &[0x48, 0x2a, 1],
    ]);
    let (index, warnings) = read(&word, &table);
    assert_eq!(warnings, 0);
    assert_eq!(index.is_deleted(101), Some(true));
    assert_eq!(index.is_deleted(103), Some(true));
    assert_eq!(index.is_deleted(105), Some(false));
    assert_eq!(index.is_deleted(107), None);
    assert_eq!(piece_deleted_property(&[0x00, 0x08, 1]), Some(true));
    assert_eq!(
        piece_deleted_property(&[0x48, 0x2a, 1, 0x00, 0x08, 1, 0x00, 0x08, 0]),
        Some(false)
    );
    assert_eq!(piece_deleted_property(&[0x48, 0x2a, 1]), None);
    assert_eq!(piece_deleted_property(&[0x00, 0x08]), None);
}
