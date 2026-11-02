use super::*;

fn cases() -> [(PiiCategory, &'static str); 7] {
    [
        (PiiCategory::Email, "User+tag@Example.COM"),
        (PiiCategory::Phone, "212-555-0100"),
        (PiiCategory::Ssn, "123-45-6789"),
        (PiiCategory::IpAddress, "192.0.2.1"),
        (PiiCategory::Iban, "de89 3704 0044 0532 0130 00"),
        (PiiCategory::SwiftBic, "DEUTDEFF500"),
        (PiiCategory::PostalCode, "sw1a 1aa"),
    ]
}

#[test]
fn category_candidate_error_stops_each_detector_and_preserves_error_identity() {
    for (category, text) in cases() {
        let source = format!("é {text}|{text}");
        assert_eq!(&source[3..3 + text.len()], text);
        let sentinel = String::from("candidate stop");
        let mut candidates = 0;
        let mut eligible = 0;
        let result = visit_category(
            &source,
            &category,
            &mut |matched| {
                candidates += 1;
                assert_eq!((matched.start, matched.end, matched.text), (3, 3 + text.len(), text));
                assert!(std::ptr::eq(matched.text, &source[matched.start..matched.end]));
                Err(&sentinel)
            },
            &mut |_| {
                eligible += 1;
                Ok(())
            },
        );
        assert!(std::ptr::eq(result.unwrap_err(), &sentinel));
        assert_eq!((candidates, eligible), (1, 0), "{category:?}");
    }
}

#[test]
fn category_eligible_error_stops_each_detector_and_preserves_error_identity() {
    for (category, text) in cases() {
        let source = format!("é {text}|{text}");
        assert_eq!(&source[3..3 + text.len()], text);
        let sentinel = String::from("eligible stop");
        let mut candidates = 0;
        let mut eligible = 0;
        let result = visit_category(
            &source,
            &category,
            &mut |_| {
                candidates += 1;
                Ok(())
            },
            &mut |matched| {
                eligible += 1;
                assert_eq!((matched.start, matched.end, matched.text), (3, 3 + text.len(), text));
                assert!(std::ptr::eq(matched.text, &source[matched.start..matched.end]));
                Err(&sentinel)
            },
        );
        assert!(std::ptr::eq(result.unwrap_err(), &sentinel));
        assert_eq!((candidates, eligible), (1, 1), "{category:?}");
    }
}

#[test]
fn category_success_borrows_original_utf8_slices_and_unsupported_categories_do_no_work() {
    for (category, text) in cases() {
        let source = format!("é {text}|{text}");
        let second = 4 + text.len();
        assert_eq!(&source[3..3 + text.len()], text);
        assert_eq!(&source[second..second + text.len()], text);
        let mut candidates = 0;
        let mut accepted = Vec::new();
        visit_category(
            &source,
            &category,
            &mut |matched| {
                candidates += 1;
                assert!(std::ptr::eq(matched.text, &source[matched.start..matched.end]));
                Ok::<_, Infallible>(())
            },
            &mut |matched| {
                assert!(std::ptr::eq(matched.text, &source[matched.start..matched.end]));
                accepted.push((matched.start, matched.end, matched.text));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(candidates, 2, "{category:?}");
        assert_eq!(
            accepted,
            vec![(3, 3 + text.len(), text), (second, second + text.len(), text)]
        );
        for empty in ["", "no match é"] {
            visit_category(empty, &category, &mut |_| panic!("unexpected candidate"), &mut |_| {
                panic!("unexpected eligible")
            })
            .unwrap_or_else(|never: Infallible| match never {});
        }
    }
    for category in [
        PiiCategory::Person,
        PiiCategory::Organization,
        PiiCategory::Location,
        PiiCategory::DateOfBirth,
        PiiCategory::Custom("email".to_string()),
    ] {
        visit_category(
            "User+tag@Example.COM",
            &category,
            &mut |_| panic!("unsupported category candidate"),
            &mut |_| panic!("unsupported category eligible"),
        )
        .unwrap_or_else(|never: Infallible| match never {});
    }
}
