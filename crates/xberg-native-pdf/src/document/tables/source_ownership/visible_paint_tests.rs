use super::*;

fn page_with_fill(fill: &str) -> Vec<u8> {
    let rules = "0 0 m 10 0 l S\n".repeat(MIN_RULES);
    format!("{rules}BT {fill} /F1 10 Tf (x) Tj ET\n").into_bytes()
}

fn audited(fill: &str) -> bool {
    audit_operators(&page_with_fill(fill), |font| font == "F1")
}

#[test]
fn dark_text_fill_is_painted() {
    assert!(audited("0 g"));
    assert!(audited("0.1 0.2 0.3 rg"));
}

#[test]
fn light_text_fills_are_not_painted() {
    for fill in ["1 g", "0.9 g", "1 1 0 rg", "0 1 0 rg"] {
        assert!(!audited(fill), "{fill} fill unexpectedly audited as visible");
    }
}
