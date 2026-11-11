//! Pins for the refusal guards of the two-table sideways recovery (xberg-io/xberg#2896). Each
//! test constructs the minimal read a single guard refuses, so removing only that guard turns
//! the test red.

use super::tests::mixed_two_table_candidate;
use super::*;
use crate::types::OcrBoundingGeometry;

/// The flattened page the accepted mixed read proves, matching the existing mixed test.
const MIXED_PAGE: &str = "Overview 7 Cash 100 31,290 Rent 107 77,078 Fuel 114 71,433 Subtotal 8 \
                          Cash 100 31,290 Rent 107 77,078 Fuel 114 71,433 Balance 9";

/// Only the prose work budget refuses this read: the reported count, the band geometry and the
/// occurrence claims stay exactly as in the accepted mixed read, and the budget left over is
/// one work unit short of the prose work.
#[test]
fn the_prose_work_budget_alone_refuses_the_mixed_two_table_read() {
    let (candidate, primary) = mixed_two_table_candidate();
    let read = candidate.ocr_elements.as_deref().unwrap();
    let primary_words = primary.ocr_elements.as_deref().unwrap();
    let dims = (200, 400);
    let boxes = canonical_table_boxes([&candidate.tables[0], &candidate.tables[1]], dims).unwrap();
    let (_, bands) = partition_words(read, boxes, dims).unwrap();
    let prose_words = bands.iter().flatten().copied().collect::<Vec<_>>();
    let work = prose_work(&prose_words, primary_words);
    assert!(work > 0, "the mixed read must carry prose words for the budget to bind");
    let limits = SecurityLimits::default();
    let remaining = remaining_work(
        [&candidate.tables[0], &candidate.tables[1]],
        [&candidate, &primary],
        read,
        primary_words,
        &limits,
    )
    .unwrap();
    assert!(work <= remaining, "the default limits must accept the mixed read");
    let tight = SecurityLimits {
        max_iterations: MAX_WORK - (remaining - (work - 1)),
        ..Default::default()
    };
    let prove = |limits: &SecurityLimits| complete_text(&candidate, &primary, &Default::default(), limits, 0.5);
    assert_eq!(
        prove(&limits).as_deref(),
        Some(MIXED_PAGE),
        "the mixed read passes under the default limits"
    );
    assert_eq!(
        prove(&tight),
        None,
        "one work unit of prose work short must refuse the read"
    );
}

/// Only the band-crossing branch refuses this read: the extra word carries no tokens, so the
/// word count, the prose order, the work budget and the occurrence claims all stay as in the
/// accepted mixed read no matter which band a weakened branch would put it in.
#[test]
fn a_word_whose_box_crosses_a_table_edge_refuses_even_without_tokens() {
    let (mut candidate, primary) = mixed_two_table_candidate();
    let mut straddler = candidate.ocr_elements.as_ref().unwrap()[0].clone();
    straddler.text = String::new();
    straddler.geometry = OcrBoundingGeometry::Rectangle {
        left: 100,
        top: 155,
        width: 10,
        height: 10,
    };
    candidate.ocr_elements.as_mut().unwrap().push(straddler);
    assert_eq!(
        complete_text(&candidate, &primary, &Default::default(), &Default::default(), 0.5),
        None,
        "a box crossing the first table's bottom edge is in no prose band"
    );
}

/// Only the mutual centre test refuses these reads: the read occurrence's box undoes to exactly
/// the primary word's box (10, 160)-(20, 170), so each one-directional half of the centre test
/// must be unable to claim on its own.
#[test]
fn primary_prose_is_claimed_only_when_the_centre_test_holds_in_both_directions() {
    let (candidate, mut primary) = mixed_two_table_candidate();
    let prove =
        |primary: &ExtractedDocument| complete_text(&candidate, primary, &Default::default(), &Default::default(), 0.5);
    assert!(
        prove(&primary).is_some(),
        "the mixed read must reach the occurrence claim"
    );
    // The primary box holds the read occurrence's centre, but its own centre is outside.
    primary.ocr_elements.as_mut().unwrap()[1].geometry = OcrBoundingGeometry::Rectangle {
        left: 0,
        top: 150,
        width: 100,
        height: 25,
    };
    assert_eq!(prove(&primary), None, "one direction of the centre test cannot claim");
    // The primary box's centre is inside the read occurrence, but the read occurrence's centre
    // is outside the primary box.
    let (candidate, mut primary) = mixed_two_table_candidate();
    primary.ocr_elements.as_mut().unwrap()[1].geometry = OcrBoundingGeometry::Rectangle {
        left: 16,
        top: 160,
        width: 4,
        height: 4,
    };
    assert_eq!(
        complete_text(&candidate, &primary, &Default::default(), &Default::default(), 0.5),
        None,
        "the mirrored one-directional centre test cannot claim either"
    );
}

/// Only the reported word count refuses this read: the count feeds `prose_bands` alone, and
/// every other guard sees the same accepted read.
#[test]
fn the_reported_word_count_alone_refuses_the_mixed_two_table_read() {
    let (mut candidate, primary) = mixed_two_table_candidate();
    let prove = |candidate: &ExtractedDocument| {
        complete_text(candidate, &primary, &Default::default(), &Default::default(), 0.5)
    };
    assert_eq!(prove(&candidate).as_deref(), Some(MIXED_PAGE));
    candidate.metadata.additional.insert("word_count".into(), 25.into());
    assert_eq!(
        prove(&candidate),
        None,
        "the read's reported count must decide the prose bands"
    );
}
