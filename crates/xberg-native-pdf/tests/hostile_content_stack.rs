//! A content stream of unmatched `q` operators must not grow the graphics-state
//! stack without bound, and text after it must still be extracted.

mod common;

#[test]
fn many_unmatched_save_operators_still_extract_text() {
    let mut content = "q\n".repeat(300_000);
    content.push_str("BT /Helvetica 12 Tf 100 700 Td (hello) Tj ET\n");
    let text = common::build_and_extract_page0(&content);
    assert!(text.contains("hello"), "got {text:?}");
}
