use super::*;

type ExpectedSpan = (usize, usize, PiiCategory, &'static str);

fn assert_owned(source: &str, actual: impl FnOnce() -> Vec<PatternMatch>, expected: &[ExpectedSpan]) {
    for (start, end, _, text) in expected {
        assert_eq!(&source[*start..*end], *text);
    }
    let actual: Vec<_> = actual()
        .into_iter()
        .map(|m| (m.start, m.end, m.category, m.text))
        .collect();
    let expected: Vec<_> = expected
        .iter()
        .map(|(start, end, category, text)| (*start, *end, category.clone(), text.to_string()))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn email_owned_output_preserves_original_case_and_utf8_byte_offsets() {
    let source = "é User+tag@Example.COM; no@host";
    assert_eq!(&source[25..32], "no@host");
    assert_owned(
        source,
        || email::find_all(source),
        &[(3, 23, PiiCategory::Email, "User+tag@Example.COM")],
    );
}

#[test]
fn phone_owned_output_retains_separators_and_rejects_short_number() {
    let source = "é 212-555-0100; 1234";
    assert_eq!(&source[17..21], "1234");
    assert_owned(
        source,
        || phone::find_all(source),
        &[(3, 15, PiiCategory::Phone, "212-555-0100")],
    );
}

#[test]
fn ssn_owned_output_rejects_reserved_area_zero_group_and_zero_serial() {
    let source = "é 000-12-3456|666-12-3456|900-12-3456|123-00-6789|123-45-0000|123-45-6789";
    for (start, end, text) in [
        (3, 14, "000-12-3456"),
        (15, 26, "666-12-3456"),
        (27, 38, "900-12-3456"),
        (39, 50, "123-00-6789"),
        (51, 62, "123-45-0000"),
    ] {
        assert_eq!(&source[start..end], text);
    }
    assert_owned(
        source,
        || ssn::find_all(source),
        &[(63, 74, PiiCategory::Ssn, "123-45-6789")],
    );
}

#[test]
fn ip_owned_output_keeps_ipv4_before_earlier_ipv6_source_span() {
    let source = "é 2001:db8:0:0:0:0:0:1|192.0.2.1";
    assert_owned(
        source,
        || ip_address::find_all(source),
        &[
            (24, 33, PiiCategory::IpAddress, "192.0.2.1"),
            (3, 23, PiiCategory::IpAddress, "2001:db8:0:0:0:0:0:1"),
        ],
    );
}

#[test]
fn iban_owned_output_preserves_original_case_after_ascii_normalization() {
    let source = "é zz12 1234 5678 9012|de89 3704 0044 0532 0130 00";
    assert_eq!(&source[3..22], "zz12 1234 5678 9012");
    assert_eq!(source.len(), source.to_ascii_uppercase().len());
    assert_owned(
        source,
        || iban::find_all(source),
        &[(23, 50, PiiCategory::Iban, "de89 3704 0044 0532 0130 00")],
    );
}

#[test]
fn swift_owned_output_preserves_uppercase_only_eight_and_eleven_byte_codes() {
    let source = "é deutdeff|DEUTDEFF|DEUTDEFF500";
    assert_eq!(&source[3..11], "deutdeff");
    assert_owned(
        source,
        || swift_bic::find_all(source),
        &[
            (12, 20, PiiCategory::SwiftBic, "DEUTDEFF"),
            (21, 32, PiiCategory::SwiftBic, "DEUTDEFF500"),
        ],
    );
}

#[test]
fn postal_uk_owned_output_preserves_lowercase_original() {
    let source = "é sw1a 1aa";
    assert_owned(
        source,
        || postal_code::find_all(source),
        &[(3, 11, PiiCategory::PostalCode, "sw1a 1aa")],
    );
}

#[test]
fn postal_canadian_owned_output_preserves_lowercase_original() {
    let source = "é k1a 0b1";
    assert_owned(
        source,
        || postal_code::find_all(source),
        &[(3, 10, PiiCategory::PostalCode, "k1a 0b1")],
    );
}

#[test]
fn postal_us_owned_output_keeps_zip_plus_four() {
    let source = "é 90210-1234";
    assert_owned(
        source,
        || postal_code::find_all(source),
        &[(3, 13, PiiCategory::PostalCode, "90210-1234")],
    );
}

#[test]
fn postal_owned_output_uses_locale_order_instead_of_source_order() {
    let source = "é 90210|k1a 0b1|sw1a 1aa";
    assert_owned(
        source,
        || postal_code::find_all(source),
        &[
            (17, 25, PiiCategory::PostalCode, "sw1a 1aa"),
            (9, 16, PiiCategory::PostalCode, "k1a 0b1"),
            (3, 8, PiiCategory::PostalCode, "90210"),
        ],
    );
}

#[test]
fn dispatcher_sorts_selected_and_default_categories_by_source_offsets() {
    let source = "é a@b.co; DEUTDEFF";
    let expected = [
        (3, 9, PiiCategory::Email, "a@b.co"),
        (11, 19, PiiCategory::SwiftBic, "DEUTDEFF"),
    ];
    assert_owned(
        source,
        || scan_text(source, &[PiiCategory::SwiftBic, PiiCategory::Email]),
        &expected,
    );
    assert_owned(source, || scan_text(source, &[]), &expected);
    assert_owned(source, || scan_text(source, &[PiiCategory::Email]), &expected[..1]);
    assert_owned("", || scan_text("", &[]), &[]);
}
