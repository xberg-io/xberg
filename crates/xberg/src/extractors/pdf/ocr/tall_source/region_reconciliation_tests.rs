use super::*;

fn rect([left, top, right, bottom]: [u32; 4]) -> RegionBox {
    RegionBox {
        left,
        top,
        right,
        bottom,
    }
}

fn stock(text: &str, bounds: [u32; 4], start: usize) -> StockWord<'_> {
    StockWord {
        transcript_bytes: Some(start..start + text.len()),
        text,
        bounds: rect(bounds),
    }
}

fn source(text: &str, bounds: [u32; 4], confidence: f64) -> SourceWord<'_> {
    SourceWord {
        text,
        bounds: rect(bounds),
        confidence,
    }
}

fn materialize(transcript: &str, edits: &[OccurrenceEdit<'_>]) -> String {
    let mut result = transcript.to_owned();
    let mut end = transcript.len();
    for edit in edits.iter().rev() {
        assert!(edit.bytes.end <= end);
        assert_eq!(transcript.get(edit.bytes.clone()), Some(edit.expected));
        result.replace_range(edit.bytes.clone(), edit.replacement);
        end = edit.bytes.start;
    }
    result
}

fn heading_stock(start: usize) -> [StockWord<'static>; 2] {
    [
        stock("POP", [205, 155, 251, 173], start),
        stock("WAcES:", [262, 155, 354, 173], start + 4),
    ]
}

fn heading_source() -> [SourceWord<'static>; 2] {
    [
        source("PDP", [206, 157, 251, 171], 95.872864),
        source("IMAGES", [265, 157, 353, 171], 96.422661),
    ]
}

#[test]
fn frozen_page_eight_corrects_only_the_owned_heading() {
    let transcript = "—\n\nPOP WAcES:\n\n@ Hero\n\nEXAMPLE";
    let stock = heading_stock("—\n\n".len());
    // The accepted right crop and separate tight heading crop map to the same source boxes.
    let reads = [heading_source(), heading_source()];
    let protected = [
        rect([205, 131, 254, 140]),
        rect([71, 203, 151, 280]),
        rect([210, 229, 390, 283]),
    ];
    let edits = plan_line_edits(
        transcript,
        rect([203, 154, 355, 174]),
        rect([175, 132, 2328, 283]),
        &stock,
        [&reads[0], &reads[1]],
        &protected,
    )
    .expect("each observed heading word has one stock owner");
    assert_eq!(edits.len(), 2);
    assert_eq!(materialize(transcript, &edits), "—\n\nPDP IMAGES\n\n@ Hero\n\nEXAMPLE");
}

#[test]
fn frozen_page_thirteen_corrects_one_token_and_preserves_tables_and_annotation_text() {
    let prefix = "table one\n\ntable two\n\ntable three\n\nRecommended content\n";
    let suffix = "\ncommany\nGet the Report\ncounts\n\nAnnotations:\noriginal note";
    let transcript = format!("{prefix}wi\n\nthe blog{suffix}");
    let mut stock = [
        stock("wi", [320, 5455, 337, 5469], prefix.len()),
        stock("the", [368, 5457, 387, 5469], prefix.len() + 4),
        stock("blog", [402, 5457, 439, 5474], prefix.len() + 8),
    ];
    // The PSM 3 and PSM 11 observations agree on all three words and their boxes.
    let read = || {
        [
            source("View", [321, 5456, 364, 5469], 95.202385),
            source("the", [369, 5457, 397, 5469], 95.202385),
            source("blog", [402, 5457, 438, 5472], 94.816429),
        ]
    };
    let reads = [read(), read()];
    let protected = [
        rect([306, 358, 1229, 388]),
        rect([306, 945, 1229, 975]),
        rect([306, 1229, 1392, 1405]),
        rect([906, 5435, 943, 5443]),
        rect([947, 5435, 984, 5443]),
    ];
    let edits = plan_line_edits(
        &transcript,
        rect([319, 5454, 440, 5475]),
        rect([0, 5287, 742, 5773]),
        &stock,
        [&reads[0], &reads[1]],
        &protected,
    )
    .expect("the two adjacent stock anchors are unchanged");
    assert_eq!(
        edits,
        [OccurrenceEdit {
            bytes: prefix.len()..prefix.len() + 2,
            expected: "wi",
            replacement: "View",
        }]
    );
    assert_eq!(
        materialize(&transcript, &edits),
        format!("{prefix}View\n\nthe blog{suffix}")
    );
    let interrupted = format!("{prefix}wi\n\nretained paragraph\n\nthe blog{suffix}");
    let extra = "retained paragraph\n\n".len();
    stock[1].transcript_bytes = Some(prefix.len() + 4 + extra..prefix.len() + 7 + extra);
    stock[2].transcript_bytes = Some(prefix.len() + 8 + extra..prefix.len() + 12 + extra);
    assert!(
        plan_line_edits(
            &interrupted,
            rect([319, 5454, 440, 5475]),
            rect([0, 5287, 742, 5773]),
            &stock,
            [&reads[0], &reads[1]],
            &protected,
        )
        .is_none()
    );
}

#[test]
fn verified_ranges_select_the_owned_occurrence_among_lookalikes() {
    let transcript = "POP WAcES:\n\n@ Hero";
    let region = rect([203, 154, 355, 174]);
    let stock = heading_stock(0);
    let source = heading_source();
    let positive = || {
        plan_line_edits(
            transcript,
            region,
            rect([175, 132, 2328, 283]),
            &stock,
            [&source, &source],
            &[],
        )
    };
    assert!(positive().is_some());
    let repeated = "POP WAcES:\nPOP WAcES:\n";
    let second = heading_stock("POP WAcES:\n".len());
    let edits = plan_line_edits(repeated, region, region, &second, [&source, &source], &[]).unwrap();
    assert_eq!(materialize(repeated, &edits), "POP WAcES:\nPDP IMAGES\n");
    let protected = [rect([265, 157, 353, 171])];
    assert!(
        plan_line_edits(
            transcript,
            region,
            rect([175, 132, 2328, 283]),
            &stock,
            [&source, &source],
            &protected
        )
        .is_none()
    );
    let mut cross_owner = heading_source();
    cross_owner[0].bounds.right = 266;
    cross_owner[1].bounds.left = 268;
    assert!(
        plan_line_edits(
            transcript,
            region,
            rect([175, 132, 2328, 283]),
            &stock,
            [&cross_owner, &source],
            &[]
        )
        .is_none()
    );
}

#[test]
fn refuses_partial_disagreeing_low_confidence_and_out_of_order_reads() {
    let transcript = "POP WAcES:\n";
    let region = rect([203, 154, 355, 174]);
    let stock = heading_stock(0);
    let good = heading_source();
    let reject = |other: &[SourceWord<'_>]| {
        plan_line_edits(
            transcript,
            region,
            rect([175, 132, 2328, 283]),
            &stock,
            [&good, other],
            &[],
        )
        .is_none()
    };
    assert!(reject(&good[..1]));
    let mut other = heading_source();
    other[0].text = "POP";
    assert!(reject(&other));
    other[0].text = "PDP";
    other[0].confidence = f64::NAN;
    assert!(reject(&other));
    other[0].confidence = 17.08375;
    assert!(reject(&other));
    other[0].confidence = 95.872864;
    other.swap(0, 1);
    assert!(reject(&other));
}

#[test]
fn refuses_to_project_unmatched_regions_and_preserves_nonedited_bytes() {
    let transcript = "table cells unchanged\ncomer nest\n\nAnnotations:\noriginal note";
    let stock = [
        stock("comer", [906, 5287, 940, 5296], "table cells unchanged\n".len()),
        stock("nest", [943, 5287, 978, 5296], "table cells unchanged\ncomer ".len()),
    ];
    let read = [
        source("FIRST", [907, 5288, 940, 5295], 93.289673),
        source("NAME", [943, 5288, 978, 5295], 57.503773),
        source("John", [940, 5321, 965, 5328], 85.409805),
    ];
    assert!(
        plan_line_edits(
            transcript,
            rect([905, 5287, 983, 5329]),
            rect([905, 5287, 2328, 5773]),
            &stock,
            [&read, &read],
            &[],
        )
        .is_none()
    );
    assert!(
        plan_line_edits(
            transcript,
            rect([905, 5287, 983, 5296]),
            rect([905, 5287, 2328, 5773]),
            &stock,
            [&read[..2], &read[..2]],
            &[],
        )
        .is_none(),
        "the equal-count right-column line still lacks a confident confirmation"
    );
    let stock = heading_stock("é\r\n  ".len());
    let read = heading_source();
    let transcript = "é\r\n  POP\tWAcES:  \r\n@ Hero\r\n";
    let edits = plan_line_edits(
        transcript,
        rect([203, 154, 355, 174]),
        rect([175, 132, 2328, 283]),
        &stock,
        [&read, &read],
        &[],
    )
    .expect("project tokens without changing whitespace or Unicode outside their ranges");
    assert_eq!(materialize(transcript, &edits), "é\r\n  PDP\tIMAGES  \r\n@ Hero\r\n");
}

#[test]
fn omitted_projection_never_rewrites_a_unique_unrelated_lookalike() {
    let transcript = "unrelated\nPOP WAcES:\n@ Hero";
    assert_eq!(transcript.matches("POP WAcES:").count(), 1);
    let region = rect([203, 154, 355, 174]);
    let read = heading_source();
    let mut omitted = heading_stock(0);
    for word in &mut omitted {
        word.transcript_bytes = None;
    }
    assert!(plan_line_edits(transcript, region, region, &omitted, [&read, &read], &[]).is_none());
    let stale = heading_stock(0);
    assert!(plan_line_edits(transcript, region, region, &stale, [&read, &read], &[]).is_none());
}

#[test]
fn refuses_a_region_crossing_the_column_gutter_even_when_all_words_fit() {
    let transcript = "POP WAcES:\n";
    let stock = heading_stock(0);
    let read = heading_source();
    let column = rect([0, 132, 742, 283]);
    let allowed = rect([203, 154, 355, 174]);
    let crossing = rect([203, 154, 906, 174]);
    assert!(plan_line_edits(transcript, allowed, column, &stock, [&read, &read], &[]).is_some());
    assert!(plan_line_edits(transcript, crossing, column, &stock, [&read, &read], &[]).is_none());
}
