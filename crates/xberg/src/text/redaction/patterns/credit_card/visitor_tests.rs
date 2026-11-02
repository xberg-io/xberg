use super::*;

fn spans(text: &str) -> Vec<(usize, usize, PiiCategory, String)> {
    find_all(text)
        .into_iter()
        .map(|matched| (matched.start, matched.end, matched.category, matched.text))
        .collect()
}

#[test]
fn owned_matches_keep_exact_utf8_offsets_and_original_separator_bytes() {
    let source = "é 4111 1111 1111 1111; 4111-1111-1111-1111.";
    assert_eq!(&source[3..22], "4111 1111 1111 1111");
    assert_eq!(&source[24..43], "4111-1111-1111-1111");
    assert_eq!(
        spans(source),
        vec![
            (3, 22, PiiCategory::CreditCard, "4111 1111 1111 1111".to_string()),
            (24, 43, PiiCategory::CreditCard, "4111-1111-1111-1111".to_string()),
        ]
    );
}

#[test]
fn owned_matches_reject_invalid_luhn_and_keep_later_valid_order() {
    let source = "bad 4111111111111112; good 4111-1111-1111-1111; again 4111 1111 1111 1111";
    assert_eq!(&source[4..20], "4111111111111112");
    assert_eq!(&source[27..46], "4111-1111-1111-1111");
    assert_eq!(&source[54..73], "4111 1111 1111 1111");
    assert_eq!(
        RE_CC.find_iter(source).count(),
        3,
        "all three are actual regex candidates"
    );
    assert!(!luhn_check("4111111111111112"));
    assert!(luhn_check("4111111111111111"));
    assert_eq!(
        spans(source),
        vec![
            (27, 46, PiiCategory::CreditCard, "4111-1111-1111-1111".to_string()),
            (54, 73, PiiCategory::CreditCard, "4111 1111 1111 1111".to_string()),
        ]
    );
}

#[test]
fn owned_detector_preserves_length_boundaries_and_ascii_digit_validation() {
    let valid = "4222222222222";
    let short = "411111111111";
    let long = "41111111111111111111";
    let unicode = "４１１１１１１１１１１１１１１１";
    assert_eq!(
        (valid.len(), short.len(), long.len(), unicode.chars().count()),
        (13, 12, 20, 16)
    );
    assert!(luhn_check(valid));
    assert_eq!(RE_CC.find_iter(valid).count(), 1);
    assert_eq!(RE_CC.find_iter(short).count(), 0);
    assert_eq!(RE_CC.find_iter(long).count(), 0);
    assert_eq!(
        RE_CC.find_iter(unicode).count(),
        1,
        "Unicode digits reach the existing ASCII validator"
    );
    assert_eq!(unicode.chars().filter(char::is_ascii_digit).count(), 0);
    assert_eq!(
        spans(valid),
        vec![(0, 13, PiiCategory::CreditCard, "4222222222222".to_string())]
    );
    assert_eq!(spans(short), vec![]);
    assert_eq!(spans(long), vec![]);
    assert_eq!(spans(unicode), vec![]);
}

#[test]
fn candidate_error_stops_before_eligibility_and_preserves_error_identity() {
    let source = "4111111111111112; 4111-1111-1111-1111";
    assert_eq!(RE_CC.find_iter(source).count(), 2);
    let sentinel = String::from("candidate rejected");
    let mut candidates = 0;
    let mut eligible = 0;
    let result = visit_matches(
        source,
        &mut |matched| {
            candidates += 1;
            assert_eq!((matched.start, matched.end, matched.text), (0, 16, &source[..16]));
            Err(&sentinel)
        },
        &mut |_| {
            eligible += 1;
            Ok(())
        },
    );
    assert!(std::ptr::eq(result.unwrap_err(), &sentinel));
    assert_eq!((candidates, eligible), (1, 0));
}

#[test]
fn eligible_error_stops_after_actual_luhn_rejection_and_preserves_error_identity() {
    let source = "4111111111111112; 4111-1111-1111-1111; 4111 1111 1111 1111";
    assert_eq!(RE_CC.find_iter(source).count(), 3);
    assert!(!luhn_check(&source[..16]));
    let sentinel = String::from("eligible rejected");
    let mut candidates = 0;
    let mut eligible = 0;
    let result = visit_matches(
        source,
        &mut |_| {
            candidates += 1;
            Ok(())
        },
        &mut |matched| {
            eligible += 1;
            assert_eq!((matched.start, matched.end, matched.text), (18, 37, &source[18..37]));
            Err(&sentinel)
        },
    );
    assert!(std::ptr::eq(result.unwrap_err(), &sentinel));
    assert_eq!((candidates, eligible), (2, 1));
}

#[test]
fn successful_visitor_borrows_exact_source_slices_and_empty_inputs_do_no_work() {
    let source = "é 4111 1111 1111 1111; 4111-1111-1111-1111.";
    let mut candidates = 0;
    let mut accepted = Vec::new();
    visit_matches(
        source,
        &mut |_| {
            candidates += 1;
            Ok::<_, Infallible>(())
        },
        &mut |matched| {
            let original = &source[matched.start..matched.end];
            assert!(std::ptr::eq(matched.text, original));
            accepted.push((matched.start, matched.end, matched.text));
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(candidates, 2);
    assert_eq!(
        accepted,
        vec![(3, 22, "4111 1111 1111 1111"), (24, 43, "4111-1111-1111-1111")]
    );
    for empty in ["", "no card é"] {
        let mut candidates = 0;
        let mut eligible = 0;
        visit_matches(
            empty,
            &mut |_| {
                candidates += 1;
                Ok::<_, Infallible>(())
            },
            &mut |_| {
                eligible += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!((candidates, eligible), (0, 0));
    }
}
