//! A fixed-arity operator is defined by its **trailing** operands.
//!
//! Real content streams carry leaked operands ahead of an operator: pdfTeX
//! emits `bcolor [0]` and `pagesize width 597.5pt height 845pt`, and the
//! unknown operator is skipped while its operand stays on the stack. Reading
//! the operands from the front then shifts every element of the next operator
//! (`[0] 1 0 0 1 -72 -779 cm` -> a=1, b=1, ..., f=-72) or drops it entirely,
//! and a page whose text is translated by such a `cm` extracts empty.
//!
//! Poppler's `Gfx::execOp` applies the last N operands. These tests pin the
//! same rule on every parser the text extractor routes through, including the
//! >256 KiB prescan path (xberg-io/xberg#3383).

use xberg_native_pdf::content::{Operator, parse_and_execute_text_only, parse_content_stream};

fn cms(stream: &[u8]) -> Vec<[f32; 6]> {
    parse_content_stream(stream)
        .expect("the stream parses")
        .into_iter()
        .filter_map(|operator| match operator {
            Operator::Cm { a, b, c, d, e, f } => Some([a, b, c, d, e, f]),
            _ => None,
        })
        .collect()
}

fn executed_cms(stream: &[u8]) -> Vec<[f32; 6]> {
    let mut seen = Vec::new();
    parse_and_execute_text_only(stream, |operator| {
        if let Operator::Cm { a, b, c, d, e, f } = operator {
            seen.push([a, b, c, d, e, f]);
        }
        Ok(())
    })
    .expect("the stream parses");
    seen
}

#[test]
fn a_surplus_array_operand_does_not_shift_the_matrix() {
    // pdfTeX leak: the array is the abandoned operand of an unknown operator.
    let stream = b"[0] 1 0 0 1 -300 0 cm";

    assert_eq!(cms(stream), vec![[1.0, 0.0, 0.0, 1.0, -300.0, 0.0]]);
    assert_eq!(executed_cms(stream), vec![[1.0, 0.0, 0.0, 1.0, -300.0, 0.0]]);
}

#[test]
fn a_surplus_number_operand_is_discarded_not_read_as_the_matrix() {
    let stream = b"7 1 0 0 1 -300 0 cm";

    assert_eq!(cms(stream), vec![[1.0, 0.0, 0.0, 1.0, -300.0, 0.0]]);
    assert_eq!(executed_cms(stream), vec![[1.0, 0.0, 0.0, 1.0, -300.0, 0.0]]);
}

#[test]
fn an_unknown_operator_does_not_leave_its_operands_for_the_next_one() {
    let stream = b"bcolor [0]\n1 0 0 1 -300 0 cm";

    assert_eq!(cms(stream), vec![[1.0, 0.0, 0.0, 1.0, -300.0, 0.0]]);
    assert_eq!(executed_cms(stream), vec![[1.0, 0.0, 0.0, 1.0, -300.0, 0.0]]);
}

#[test]
fn a_surplus_name_operand_does_not_drop_a_rectangle() {
    let stream = b"/X 10 20 30 40 re";
    let rectangles: Vec<[f32; 4]> = parse_content_stream(stream)
        .expect("the stream parses")
        .into_iter()
        .filter_map(|operator| match operator {
            Operator::Rectangle { x, y, width, height } => Some([x, y, width, height]),
            _ => None,
        })
        .collect();

    assert_eq!(rectangles, vec![[10.0, 20.0, 30.0, 40.0]]);
}

#[test]
fn the_large_stream_prescan_path_applies_the_same_matrix() {
    // Control: > 256 KiB routes `parse_and_execute_text_only` through the
    // prescan, which reconstructs the CTM from its own forward scan and injects
    // it around each region. That scan already took the trailing six operands;
    // the assertion keeps it that way while the sequential readers change.
    let mut stream = Vec::with_capacity(300 * 1024);
    while stream.len() < 280 * 1024 {
        stream.extend_from_slice(b"q 1 0 0 1 0 0 cm Q\n");
    }
    stream.extend_from_slice(b"[0] 1 0 0 1 -300 0 cm\nBT /F1 12 Tf 72 720 Td (Hello) Tj ET\n");

    assert!(
        stream.len() > 256 * 1024,
        "the fixture must exceed the prescan threshold, got {}",
        stream.len()
    );
    assert_eq!(executed_cms(&stream), vec![[1.0, 0.0, 0.0, 1.0, -300.0, 0.0]]);
}
