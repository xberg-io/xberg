use super::*;

const SOURCE: &str = "Reliable native source words remain available beside the missing character boundary. \u{fffd}";

fn boundary(page_number: u32, byte_start: usize, byte_end: usize) -> crate::types::PageBoundary {
    crate::types::PageBoundary {
        page_number,
        byte_start,
        byte_end,
    }
}

fn candidates(text: &str, boundaries: &[crate::types::PageBoundary]) -> ahash::AHashMap<u32, String> {
    let selected = boundaries.iter().map(|boundary| boundary.page_number).collect();
    marker_native_ink_layers(text, boundaries, &selected, 4, &OcrQualityThresholds::default())
}

#[test]
fn should_take_only_the_requested_marker_page_using_its_document_identity() {
    let text = SOURCE.repeat(2);
    let bounds = [boundary(4, SOURCE.len(), text.len()), boundary(2, 0, SOURCE.len())];
    let result = marker_native_ink_layers(
        &text,
        &bounds,
        &std::collections::HashSet::from([4]),
        4,
        &OcrQualityThresholds::default(),
    );
    assert_eq!(result, ahash::AHashMap::from_iter([(4, SOURCE.to_owned())]));
}

#[test]
fn should_not_offer_marker_free_empty_or_garbled_layers_for_marker_ink_proof() {
    let clean = SOURCE.replace(char::REPLACEMENT_CHARACTER, "");
    for source in [clean.as_str(), "\u{fffd}\u{fffd}", "q q \0 \u{fffd}"] {
        assert_eq!(
            candidates(source, &[boundary(1, 0, source.len())]),
            ahash::AHashMap::new()
        );
    }
}

#[test]
fn should_reject_overlapping_source_owners_and_duplicate_document_pages() {
    let text = SOURCE.repeat(2);
    for bounds in [
        [boundary(1, 0, SOURCE.len()), boundary(2, 0, SOURCE.len())],
        [boundary(1, 0, SOURCE.len()), boundary(1, SOURCE.len(), text.len())],
        [boundary(1, 0, text.len()), boundary(2, 10, 20)],
    ] {
        assert_eq!(candidates(&text, &bounds), ahash::AHashMap::new(), "{bounds:?}");
    }
}

#[test]
fn should_reject_invalid_utf8_ranges_and_out_of_range_page_numbers() {
    for source in [
        boundary(0, 0, SOURCE.len()),
        boundary(5, 0, SOURCE.len()),
        boundary(1, SOURCE.len(), 0),
        boundary(1, 0, SOURCE.len() + 1),
        boundary(1, SOURCE.len() - 2, SOURCE.len()),
    ] {
        assert_eq!(candidates(SOURCE, &[source]), ahash::AHashMap::new());
    }
}

#[test]
fn should_keep_an_independent_valid_page_when_other_source_owners_are_duplicated() {
    let text = SOURCE.repeat(3);
    let bounds = [
        boundary(1, 0, SOURCE.len()),
        boundary(1, SOURCE.len(), 2 * SOURCE.len()),
        boundary(3, 2 * SOURCE.len(), text.len()),
    ];
    assert_eq!(
        candidates(&text, &bounds),
        ahash::AHashMap::from_iter([(3, SOURCE.to_owned())])
    );
}
