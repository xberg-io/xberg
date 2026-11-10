use super::super::native_word_coverage::final_ocr_structure_can_yield_to_native;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};

const PAGE: u32 = 7;
const OTHER_PAGE: u32 = 3;
const READ: &str = "The orchard harvest includes apples | Crates | 42 |";
const BODY: &str = "The orchard harvest includes apples";

struct FinalPage {
    document: InternalDocument,
    words_covered: ahash::AHashSet<u32>,
    ink_owned: ahash::AHashSet<u32>,
    clean_admissions: std::collections::HashSet<u32>,
    layers: std::sync::Arc<ahash::AHashMap<u32, String>>,
}

impl FinalPage {
    fn omitted_body_word() -> Self {
        let mut document = super::super::document::flat_ocr_page_document("The orchard harvest includes");
        let table_index = document.push_table(crate::types::Table {
            cells: vec![vec!["Crates".into(), "42".into()]],
            markdown: "| Crates | 42 |".into(),
            page_number: PAGE,
            ..Default::default()
        });
        document
            .elements
            .push(InternalElement::text(ElementKind::Table { table_index }, "", 0));
        Self {
            document,
            words_covered: ahash::AHashSet::new(),
            ink_owned: ahash::AHashSet::from_iter([PAGE]),
            clean_admissions: std::collections::HashSet::from([OTHER_PAGE]),
            layers: std::sync::Arc::new(ahash::AHashMap::from_iter([(PAGE, READ.to_owned())])),
        }
    }

    fn can_yield(&self) -> bool {
        final_ocr_structure_can_yield_to_native(
            PAGE,
            &self.document,
            READ,
            &self.words_covered,
            &self.ink_owned,
            Some(&self.layers),
            &self.clean_admissions,
        )
    }
}

#[test]
fn should_keep_ocr_when_ink_owned_page_was_not_cleanly_admitted_and_structure_omits_body_word() {
    let page = FinalPage::omitted_body_word();
    assert_eq!(page.document.tables[0].cells, vec![vec!["Crates", "42"]]);
    assert_eq!(page.layers.get(&PAGE).map(String::as_str), Some(READ));
    assert!(!page.clean_admissions.contains(&PAGE));
    assert!(!page.can_yield());
}

#[test]
fn should_allow_body_word_omission_for_the_same_page_admitted_with_a_clean_layer() {
    let mut page = FinalPage::omitted_body_word();
    page.clean_admissions.insert(PAGE);
    assert!(page.can_yield());
}

#[test]
fn should_allow_exact_table_structure_without_clean_admission() {
    let mut page = FinalPage::omitted_body_word();
    page.document.elements[0].text = BODY.into();
    assert!(page.can_yield());
}

#[test]
fn should_keep_ocr_when_only_another_page_has_a_clean_layer() {
    let mut page = FinalPage::omitted_body_word();
    page.clean_admissions.insert(PAGE);
    page.layers = std::sync::Arc::new(ahash::AHashMap::from_iter([(OTHER_PAGE, READ.to_owned())]));
    assert!(!page.can_yield());
}

#[test]
fn should_keep_ocr_when_word_coverage_precedes_ink_ownership_for_a_table() {
    let mut page = FinalPage::omitted_body_word();
    page.clean_admissions.insert(PAGE);
    page.words_covered.insert(PAGE);
    assert!(!page.can_yield());
}

#[test]
fn should_keep_table_structure_without_native_ink_ownership() {
    let mut page = FinalPage::omitted_body_word();
    page.clean_admissions.insert(PAGE);
    page.ink_owned.clear();
    assert!(!page.can_yield());
}

#[test]
fn should_keep_ocr_metadata_even_with_clean_admission() {
    let mut page = FinalPage::omitted_body_word();
    page.clean_admissions.insert(PAGE);
    page.document
        .metadata
        .additional
        .insert("orchard_inspector".into(), serde_json::json!("Ada"));
    assert!(!page.can_yield());
}

#[test]
fn should_keep_an_unread_body_word_even_with_clean_admission() {
    let mut page = FinalPage::omitted_body_word();
    page.clean_admissions.insert(PAGE);
    page.document.elements[0].text = "Uncertified".into();
    assert!(!page.can_yield());
}

#[test]
fn should_allow_exact_plain_structure_before_ownership_checks() {
    let mut page = FinalPage::omitted_body_word();
    page.document = super::super::document::flat_ocr_page_document(READ);
    page.ink_owned.clear();
    assert!(page.can_yield());
}
