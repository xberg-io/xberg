//! Glyphs and spans of one font share one font-name allocation.

use super::super::*;
use super::extraction_and_rotation::create_test_font;

#[test]
fn glyphs_of_one_font_share_one_name_allocation() {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), create_test_font());
    let chars = extractor
        .extract_owned(b"BT /F1 12 Tf 72 700 Td (Hi) Tj /F1 10 Tf (Ho) Tj ET")
        .unwrap();
    assert!(chars.len() >= 2, "both runs yield glyphs: {chars:?}");
    let first = chars[0].font_name.as_str().as_ptr();
    for ch in &chars {
        assert_eq!(ch.font_name, "Times-Roman");
        assert!(
            std::ptr::eq(ch.font_name.as_str().as_ptr(), first),
            "{ch:?} copies its font name"
        );
    }
}

#[test]
fn reselecting_a_font_reuses_its_interned_name() {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), create_test_font());
    let font = extractor.fonts.get("F1").cloned();
    extractor.set_cached_current_font(font.clone());
    let first = extractor.glyph_font_name.clone();
    extractor.set_cached_current_font(None);
    assert_eq!(extractor.glyph_font_name, "");
    extractor.set_cached_current_font(font);
    assert_eq!(first, "Times-Roman");
    assert!(std::ptr::eq(
        first.as_str().as_ptr(),
        extractor.glyph_font_name.as_str().as_ptr()
    ));
}

fn select_font(extractor: &mut TextExtractor<'_>, font: &str, size: f32) {
    extractor
        .execute_operator(Operator::Tf {
            font: font.to_string(),
            size,
        })
        .unwrap();
}

#[test]
fn restoring_the_graphics_state_restores_the_font_it_names() {
    let mut extractor = TextExtractor::new();
    extractor.add_font("F1".to_string(), create_test_font());
    let mut other = create_test_font();
    other.base_font = "Helvetica".to_string();
    extractor.add_font("F2".to_string(), other);

    select_font(&mut extractor, "F1", 12.0);
    extractor.execute_operator(Operator::SaveState).unwrap();
    select_font(&mut extractor, "F2", 10.0);
    assert_eq!(extractor.glyph_font_name, "Helvetica");
    extractor.execute_operator(Operator::RestoreState).unwrap();
    assert_eq!(extractor.glyph_font_name, "Times-Roman");
    assert!(std::sync::Arc::ptr_eq(
        extractor.cached_current_font.as_ref().unwrap(),
        extractor.fonts.get("F1").unwrap()
    ));

    let name = extractor.glyph_font_name.clone();
    extractor.execute_operator(Operator::SaveState).unwrap();
    extractor.execute_operator(Operator::RestoreState).unwrap();
    assert!(std::ptr::eq(
        name.as_str().as_ptr(),
        extractor.glyph_font_name.as_str().as_ptr()
    ));

    // The same resource name can come to mean another font object while a scope is open.
    let mut replaced = create_test_font();
    replaced.base_font = "Courier".to_string();
    extractor.execute_operator(Operator::SaveState).unwrap();
    extractor.add_font("F1".to_string(), replaced);
    extractor.execute_operator(Operator::RestoreState).unwrap();
    assert_eq!(extractor.glyph_font_name, "Courier");
    assert!(std::sync::Arc::ptr_eq(
        extractor.cached_current_font.as_ref().unwrap(),
        extractor.fonts.get("F1").unwrap()
    ));
}
