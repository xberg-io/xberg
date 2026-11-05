//! Recognition-time OSD retry; the caller's raster and ONNX rotation policy are unchanged.

use super::*;
use crate::ocr::processor::validation::{TessdataEnv, existing_tessdata_dir_for};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use std::collections::{HashMap, HashSet};

const MIN_UNIT_LETTERS: usize = 2;
const MAX_CERTIFICATE_WORDS: usize = 1024;
const MAX_CERTIFICATE_TEXT_BYTES: usize = 512 * 1024;

pub(super) fn eligible(config: &OcrConfig) -> bool {
    config.enabled
        && config.backend == "tesseract"
        && !config.auto_rotate
        && !language_selects_vertical_psm(config)
        && config.pipeline.is_none()
        && config.backend_options.is_none()
        && config.output_format.is_none()
        && config.tesseract_config.as_ref().is_none_or(|tess| {
            tess.psm.is_none()
                && tess.preprocessing.is_none()
                && tess.output_format == "markdown"
                && tess.enable_table_detection
        })
}

fn language_selects_vertical_psm(config: &OcrConfig) -> bool {
    // A *_vert language selects PSM5 even when the caller leaves psm unset. ~keep
    config
        .effective_tesseract_language()
        .iter()
        .flat_map(|language| language.split('+'))
        .any(|language| language.trim().to_ascii_lowercase().ends_with("_vert"))
}

pub(super) fn capture_words(config: &OcrConfig) -> OcrConfig {
    let mut config = config.clone();
    config.element_config = Some(OcrElementConfig {
        include_elements: true,
        min_level: OcrElementLevel::Word,
        ..Default::default()
    });
    config
}

pub(super) fn restore_elements(mut result: ExtractedDocument, requested: &OcrConfig) -> ExtractedDocument {
    result.ocr_elements = requested.select_public_elements(result.ocr_elements.take());
    result
}

pub(super) fn existing_osd_directory(config: &OcrConfig) -> Option<String> {
    existing_osd_directory_in(config, &TessdataEnv::from_process())
}

fn existing_osd_directory_in(config: &OcrConfig, environment: &TessdataEnv) -> Option<String> {
    let directory = existing_tessdata_dir_for(
        &config.effective_tesseract_language(),
        config.tessdata_path.as_deref(),
        environment,
    )?;
    // OSD must accompany the primary language models; do not silently switch traineddata roots.
    std::path::Path::new(&directory)
        .join("osd.traineddata")
        .is_file()
        .then_some(directory)
}

pub(super) fn primary_supported(result: &ExtractedDocument) -> bool {
    !result.content.trim().is_empty()
        && result.tables.is_empty()
        && result.formulas.is_empty()
        && result.document.is_none()
        && result.djot_content.is_none()
        && result.elements.as_ref().is_none_or(Vec::is_empty)
        && result.annotations.as_ref().is_none_or(Vec::is_empty)
        && result.uris.as_ref().is_none_or(Vec::is_empty)
        && result.form_fields.is_empty()
        && result.images.as_ref().is_none_or(Vec::is_empty)
        && result
            .ocr_internal_document
            .as_ref()
            .is_none_or(plain_internal_document)
        && result.internal_document.as_ref().is_none_or(plain_internal_document)
        && checked_words(result).is_some()
}

fn plain_internal_document(document: &InternalDocument) -> bool {
    document.images.is_empty()
        && document.tables.is_empty()
        && document.diagrams.is_empty()
        && document.relationships.is_empty()
        && document.uris.is_empty()
        && document.formulas.is_empty()
        && document.recorded_formulas.is_empty()
        && document.form_fields.is_empty()
        && document.children.as_ref().is_none_or(Vec::is_empty)
        && document.annotations.as_ref().is_none_or(Vec::is_empty)
        && document.revisions.as_ref().is_none_or(Vec::is_empty)
        && document.pre_rendered_content.is_none()
        && document.elements.iter().all(plain_internal_element)
}

fn plain_internal_element(element: &InternalElement) -> bool {
    matches!(
        element.kind,
        ElementKind::Paragraph | ElementKind::OcrText { .. } | ElementKind::PageBreak
    ) && element.depth == 0
        && element.layer == crate::types::ContentLayer::Body
        && element.annotations.is_empty()
        && element.anchor.is_none()
        && element
            .attributes
            .as_ref()
            .is_none_or(|attributes| attributes.iter().all(|(name, value)| plain_ocr_attribute(name, value)))
}

fn plain_ocr_attribute(name: &str, value: &str) -> bool {
    use crate::ocr::hocr_parser::*;
    match name {
        HOCR_BOLD_FRACTION_ATTRIBUTE | HOCR_ITALIC_FRACTION_ATTRIBUTE => value.parse::<f64>().ok() == Some(0.0),
        HOCR_BLOCK_ID_ATTRIBUTE
        | HOCR_FONT_SIZE_ATTRIBUTE
        | HOCR_TEXT_ANGLE_ATTRIBUTE
        | HOCR_X_HEIGHT_ATTRIBUTE
        | HOCR_X_ASCENDERS_ATTRIBUTE
        | HOCR_X_DESCENDERS_ATTRIBUTE
        | HOCR_BASELINE_SLOPE_ATTRIBUTE
        | HOCR_BASELINE_CONST_ATTRIBUTE
        | HOCR_LINE_FONT_SIZES_ATTRIBUTE
        | HOCR_LINE_X_HEIGHTS_ATTRIBUTE
        | HOCR_FONT_NAME_ATTRIBUTE => true,
        _ => false,
    }
}

fn checked_words(result: &ExtractedDocument) -> Option<Vec<&OcrElement>> {
    if result.content.len() > MAX_CERTIFICATE_TEXT_BYTES {
        return None;
    }
    let (width, height) = processed_ocr_layout_dimensions(&result.metadata)?;
    let words = result
        .ocr_elements
        .as_deref()?
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .take(MAX_CERTIFICATE_WORDS + 1)
        .collect::<Vec<_>>();
    if words.is_empty()
        || words.len() > MAX_CERTIFICATE_WORDS
        || usize::try_from(word_count_of(&result.metadata.additional)?).ok()? != words.len()
        || words.iter().any(|word| {
            word.backend_metadata.get("backend").and_then(serde_json::Value::as_str) != Some("tesseract-iterator")
                || !word.confidence.recognition.is_finite()
                || !(0.0..=1.0).contains(&word.confidence.recognition)
                || !within_image(word, width, height)
        })
    {
        return None;
    }
    Some(words)
}

fn within_image(element: &OcrElement, width: u32, height: u32) -> bool {
    let (left, top, word_width, word_height) = element.geometry.to_aabb();
    word_width > 0
        && word_height > 0
        && left.checked_add(word_width).is_some_and(|right| right <= width)
        && top.checked_add(word_height).is_some_and(|bottom| bottom <= height)
}

pub(super) fn select(
    original: ExtractedDocument,
    mut candidate: ExtractedDocument,
    thresholds: &OcrQualityThresholds,
    confidence: ConfidenceSemantics,
    config: &OcrConfig,
) -> ExtractedDocument {
    let row_threshold = config.tesseract_config.as_ref().map_or_else(
        || crate::types::TesseractConfig::default().table_row_threshold_ratio,
        |tess| tess.table_row_threshold_ratio,
    );
    if !accepts(&original, &candidate, thresholds, confidence, row_threshold) {
        return original;
    }
    crate::core::diagnostics::dedup_extend_warnings(&mut candidate.processing_warnings, original.processing_warnings);
    candidate
}

fn accepts(
    original: &ExtractedDocument,
    candidate: &ExtractedDocument,
    thresholds: &OcrQualityThresholds,
    confidence: ConfidenceSemantics,
    row_threshold: f64,
) -> bool {
    if !primary_supported(original)
        || !candidate.formulas.is_empty()
        || checked_words(candidate).is_none()
        || processed_ocr_layout_dimensions(&original.metadata) != processed_ocr_layout_dimensions(&candidate.metadata)
    {
        return false;
    }
    let Some(dimensions) = processed_ocr_layout_dimensions(&candidate.metadata) else {
        return false;
    };
    let Some(text) = complete_table_text(candidate, row_threshold, |cells, elements, bbox, threshold| {
        matches_original_frame(cells, elements, bbox, threshold, dimensions)
    }) else {
        return false;
    };
    let mean = mean_text_conf_of(&candidate.metadata.additional);
    let noise = ocr_recognition_noise_decision(&text, thresholds, confidence, mean);
    !noise.low_confidence
        && !noise.fragmented_noise
        && mean.is_some()
        && retains_primary(original, candidate, &text, thresholds)
}

fn retains_primary(
    original: &ExtractedDocument,
    candidate: &ExtractedDocument,
    text: &str,
    thresholds: &OcrQualityThresholds,
) -> bool {
    let Some(words) = checked_words(original) else {
        return false;
    };
    let Some(candidate_words) = checked_words(candidate) else {
        return false;
    };
    let Some(bbox) = candidate.tables.first().and_then(|table| table.bounding_box) else {
        return false;
    };
    let mut claimed = HashSet::new();
    let minimum_length = thresholds.min_meaningful_word_len.min(MIN_UNIT_LETTERS);
    let mut inventory = HashMap::new();
    for token in text.split_whitespace() {
        *inventory.entry(token).or_insert(0usize) += 1;
    }
    for word in words
        .into_iter()
        .filter(|word| protected(word, thresholds, minimum_length))
    {
        if !claim_occurrence(word, &candidate_words, bbox, &mut claimed) {
            return false;
        }
        for token in word.text.split_whitespace() {
            let Some(count) = inventory.get_mut(token).filter(|count| **count > 0) else {
                return false;
            };
            *count -= 1;
        }
    }
    true
}

fn protected(word: &OcrElement, thresholds: &OcrQualityThresholds, minimum_length: usize) -> bool {
    word.confidence.recognition * 100.0 >= thresholds.min_ocr_mean_confidence
        && word.text.split_whitespace().any(|token| {
            token.chars().any(|character| character.is_numeric())
                || token.chars().filter(|character| character.is_alphabetic()).count() >= minimum_length
        })
}

fn claim_occurrence(
    primary: &OcrElement,
    candidate: &[&OcrElement],
    bbox: BoundingBox,
    claimed: &mut HashSet<usize>,
) -> bool {
    let (left, top, width, height) = primary.geometry.to_aabb();
    if f64::from(left) < bbox.x0
        || f64::from(top) < bbox.y0
        || f64::from(left) + f64::from(width) > bbox.x1
        || f64::from(top) + f64::from(height) > bbox.y1
    {
        return false;
    }
    let mut matches = candidate.iter().enumerate().filter(|(_, word)| {
        primary.page_number == word.page_number
            && primary.text.split_whitespace().eq(word.text.split_whitespace())
            && same_occurrence(primary, word)
    });
    let Some((index, _)) = matches.next() else {
        return false;
    };
    matches.next().is_none() && claimed.insert(index)
}

fn same_occurrence(left: &OcrElement, right: &OcrElement) -> bool {
    let rectangle = |element: &OcrElement| {
        let (left, top, width, height) = element.geometry.to_aabb();
        BoundingBox {
            x0: f64::from(left),
            y0: f64::from(top),
            x1: f64::from(left) + f64::from(width),
            y1: f64::from(top) + f64::from(height),
        }
    };
    let contains_center = |outer: BoundingBox, inner: BoundingBox| {
        (outer.x0..=outer.x1).contains(&((inner.x0 + inner.x1) / 2.0))
            && (outer.y0..=outer.y1).contains(&((inner.y0 + inner.y1) / 2.0))
    };
    let (left, right) = (rectangle(left), rectangle(right));
    contains_center(left, right) && contains_center(right, left)
}

fn matches_original_frame(
    cells: &[Vec<String>],
    elements: &[OcrElement],
    bbox: BoundingBox,
    threshold: f64,
    dimensions: (u32, u32),
) -> bool {
    let Some(words) = elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .take(MAX_CERTIFICATE_WORDS + 1)
        .map(|element| iterator_word(element, bbox))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    if words.len() > MAX_CERTIFICATE_WORDS {
        return false;
    }
    let matches = [90, 180, 270]
        .into_iter()
        .filter(|angle| {
            let projected = words
                .iter()
                .map(|word| project_word(word, dimensions, *angle))
                .collect::<Option<Vec<_>>>();
            projected.is_some_and(|words| table_matches_words(cells, &words, threshold))
        })
        .count();
    matches == 1
}

fn project_word(word: &HocrWord, dimensions: (u32, u32), angle: u16) -> Option<HocrWord> {
    let (width, height) = dimensions;
    let right = word.left.checked_add(word.width)?;
    let bottom = word.top.checked_add(word.height)?;
    if word.width == 0 || word.height == 0 || right > width || bottom > height {
        return None;
    }
    let (left, top, width, height) = match angle {
        90 => (height - bottom, word.left, word.height, word.width),
        180 => (width - right, height - bottom, word.width, word.height),
        270 => (word.top, width - right, word.height, word.width),
        _ => return None,
    };
    Some(HocrWord {
        left,
        top,
        width,
        height,
        ..word.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn primary(tokens: &[&str]) -> ExtractedDocument {
        let mut result = super::super::tests::table_candidate();
        result.tables.clear();
        result.content = tokens.join(" ");
        let geometry = sideways_candidate().ocr_elements.unwrap();
        let template = geometry[0].clone();
        result.ocr_elements = Some(
            tokens
                .iter()
                .map(|token| OcrElement {
                    text: (*token).into(),
                    confidence: crate::types::OcrConfidence {
                        recognition: 0.99,
                        detection: None,
                    },
                    ..geometry
                        .iter()
                        .find(|word| word.text == *token)
                        .unwrap_or(&template)
                        .clone()
                })
                .collect(),
        );
        result
            .metadata
            .additional
            .insert("word_count".into(), tokens.len().into());
        result
    }

    fn sideways_candidate() -> ExtractedDocument {
        let mut candidate = super::super::tests::table_candidate();
        candidate
            .metadata
            .additional
            .remove(crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY);
        candidate
            .metadata
            .additional
            .remove(crate::ocr_metadata_keys::OCR_ORIENTATION_DEGREES_METADATA_KEY);
        for word in candidate.ocr_elements.as_mut().unwrap() {
            let (left, top, width, height) = word.geometry.to_aabb();
            word.geometry = crate::types::OcrBoundingGeometry::Rectangle {
                left: top,
                top: 200 - left - width,
                width: height,
                height: width,
            };
        }
        candidate.tables[0].bounding_box = Some(BoundingBox {
            x0: 40.0,
            y0: 20.0,
            x1: 160.0,
            y1: 180.0,
        });
        candidate
    }

    fn accepted(primary: &ExtractedDocument, candidate: &ExtractedDocument) -> bool {
        accepts(
            primary,
            candidate,
            &OcrQualityThresholds::default(),
            ConfidenceSemantics::Legibility { scale_max: 100.0 },
            0.5,
        )
    }

    #[test]
    fn unrotated_pixels_require_exact_non_upright_rows_and_complete_table_content() {
        let original = primary(&["Cash", "100", "31,290"]);
        let candidate = sideways_candidate();
        assert!(accepted(&original, &candidate));
        assert!(!accepted(&original, &super::super::tests::table_candidate()));
        let mut changed = candidate.clone();
        changed.tables[0].cells[0][1] = "31,299".into();
        changed.content = changed.content.replace("31,290", "31,299");
        assert!(!accepted(&original, &changed));
        let mut changed = candidate.clone();
        changed.content.push_str(" Caption outside the table");
        assert!(!accepted(&original, &changed));
        let mut changed = candidate.clone();
        changed.tables.push(candidate.tables[0].clone());
        assert!(!accepted(&original, &changed));
    }

    #[test]
    fn selector_preserves_high_confidence_short_units_amounts_and_duplicate_occurrences() {
        for missing in ["Caption", "ID99", "10", "kg", "mA", "12%", "-4.17", "+42", "0.5"] {
            let original = primary(&["Cash", missing]);
            let candidate = sideways_candidate();
            assert!(
                !accepted(&original, &candidate),
                "lost literal primary token: {missing}"
            );
            let selected = select(
                original,
                candidate,
                &OcrQualityThresholds::default(),
                ConfidenceSemantics::Legibility { scale_max: 100.0 },
                &OcrConfig::default(),
            );
            assert_eq!(selected.content, format!("Cash {missing}"));
        }
        assert!(!accepted(&primary(&["Cash", "Cash"]), &sideways_candidate()));
        let mut original = primary(&["N", "+", "©"]);
        assert!(accepted(&original, &sideways_candidate()));
        original.ocr_elements.as_mut().unwrap()[0].confidence.recognition = f64::NAN;
        assert!(!accepted(&original, &sideways_candidate()));
    }

    #[test]
    fn missing_or_incomplete_primary_evidence_and_rich_payload_are_not_replaced() {
        let mut original = primary(&["Cash"]);
        original.ocr_elements = None;
        assert!(!accepted(&original, &sideways_candidate()));
        let mut original = primary(&["Cash"]);
        original.metadata.additional.insert("word_count".into(), 2.into());
        assert!(!accepted(&original, &sideways_candidate()));
        let mut original = primary(&["Cash"]);
        original.tables.push(crate::types::Table::default());
        assert!(!accepted(&original, &sideways_candidate()));
        let mut original = primary(&["Cash"]);
        original.ocr_elements.as_mut().unwrap()[0].geometry = crate::types::OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        };
        assert!(!accepted(&original, &sideways_candidate()));
        let mut candidate = sideways_candidate();
        candidate.ocr_elements.as_mut().unwrap()[0].backend_metadata.clear();
        assert!(!accepted(&primary(&["Cash"]), &candidate));
    }

    #[test]
    fn explicit_psm_preprocessing_payload_modes_and_disabled_ocr_are_preserved() {
        assert!(eligible(&OcrConfig::default()));
        let mut config = OcrConfig::default();
        config.tesseract_config.get_or_insert_default().psm = Some(11);
        assert!(!eligible(&config));
        config.tesseract_config.as_mut().unwrap().psm = None;
        config.tesseract_config.as_mut().unwrap().preprocessing = Some(Default::default());
        assert!(!eligible(&config));
        config.tesseract_config.as_mut().unwrap().preprocessing = None;
        config.tesseract_config.as_mut().unwrap().output_format = "hocr".into();
        assert!(!eligible(&config));
        config.tesseract_config = None;
        config.enabled = false;
        assert!(!eligible(&config));
        config.enabled = true;
        config.auto_rotate = true;
        assert!(!eligible(&config));
    }

    #[test]
    fn internal_evidence_does_not_change_caller_element_selection() {
        let mut requested = OcrConfig::default();
        let captured = capture_words(&requested);
        assert_eq!(
            captured.element_config.as_ref().unwrap().min_level,
            OcrElementLevel::Word
        );
        assert!(
            restore_elements(sideways_candidate(), &requested)
                .ocr_elements
                .is_none()
        );
        requested.element_config = Some(OcrElementConfig {
            include_elements: true,
            min_level: OcrElementLevel::Page,
            ..Default::default()
        });
        assert!(
            restore_elements(sideways_candidate(), &requested)
                .ocr_elements
                .is_none()
        );
        requested.element_config.as_mut().unwrap().min_level = OcrElementLevel::Word;
        assert_eq!(
            restore_elements(sideways_candidate(), &requested)
                .ocr_elements
                .unwrap()
                .len(),
            9
        );
    }
    #[test]
    fn cold_osd_probe_does_not_download_or_switch_the_primary_model_directory() {
        let root = tempfile::tempdir().unwrap();
        let requested = root.path().join("requested");
        let fallback = root.path().join("fallback");
        std::fs::create_dir(&requested).unwrap();
        std::fs::create_dir(&fallback).unwrap();
        std::fs::write(requested.join("eng.traineddata"), b"primary fixture").unwrap();
        for language in ["eng", "osd"] {
            std::fs::write(fallback.join(format!("{language}.traineddata")), b"other fixture").unwrap();
        }
        let config = OcrConfig {
            tessdata_path: Some(requested.clone()),
            ..Default::default()
        };
        let environment = TessdataEnv {
            tessdata_prefix: Some(fallback.to_str().unwrap().into()),
            cache_dir: None,
        };
        assert!(existing_osd_directory_in(&config, &environment).is_none());
        assert_eq!(
            std::fs::read_dir(&requested).unwrap().count(),
            1,
            "probe must not materialize OSD"
        );
        std::fs::write(requested.join("osd.traineddata"), b"existing fixture").unwrap();
        assert_eq!(
            existing_osd_directory_in(&config, &environment).as_deref(),
            requested.to_str()
        );
    }
    #[test]
    fn rich_internal_primary_content_is_retained_even_if_its_words_appear_in_the_table() {
        use crate::types::internal::{
            ElementKind, InternalDocument, InternalElement, Relationship, RelationshipTarget,
        };
        for case in 0..5 {
            let mut original = primary(&["Cash"]);
            let mut element = InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Block,
                },
                "Cash",
                0,
            );
            let mut document = InternalDocument::default();
            match case {
                0 => element.kind = ElementKind::Heading { level: 1 },
                1 => element.anchor = Some("cash-caption".into()),
                2 => element.annotations.push(crate::types::TextAnnotation {
                    start: 0,
                    end: 4,
                    kind: crate::types::AnnotationKind::Link {
                        url: "https://example.org/receipt".into(),
                        title: None,
                    },
                }),
                3 => document.relationships.push(Relationship {
                    source: 0,
                    target: RelationshipTarget::Key("cash-caption".into()),
                    kind: crate::types::RelationshipKind::Caption,
                }),
                4 => document.images.push(crate::types::ExtractedImage::default()),
                _ => unreachable!(),
            }
            document.elements.push(element);
            original.ocr_internal_document = Some(document);
            assert!(
                !accepted(&original, &sideways_candidate()),
                "lost rich internal content: {case}"
            );
        }
    }

    #[test]
    fn protected_primary_occurrences_cannot_be_borrowed_from_another_location_or_raster_frame() {
        let candidate = sideways_candidate();
        let mut original = primary(&["Cash"]);
        assert!(accepted(&original, &candidate));
        original.ocr_elements.as_mut().unwrap()[0].geometry = crate::types::OcrBoundingGeometry::Rectangle {
            left: 0,
            top: 0,
            width: 10,
            height: 10,
        };
        assert!(
            !accepted(&original, &candidate),
            "an outside caption is not a matching table occurrence"
        );
        let mut original = primary(&["Cash"]);
        original.metadata.additional.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
            201.into(),
        );
        assert!(
            !accepted(&original, &candidate),
            "different processed rasters cannot share literal geometry"
        );
        let mut original = primary(&["Cash"]);
        original.ocr_elements.as_mut().unwrap()[0].geometry = crate::types::OcrBoundingGeometry::Rectangle {
            left: 120,
            top: 25,
            width: 10,
            height: 10,
        };
        assert!(
            !accepted(&original, &candidate),
            "same token at another in-table location is not owned"
        );
    }
    #[test]
    fn plain_native_hocr_block_identity_is_not_a_semantic_anchor() {
        let mut original = primary(&["Cash"]);
        let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document(
            r#"<div class="ocr_page" title="bbox 0 0 200 200">
            <div class="ocr_carea" id="block_1"><p class="ocr_par">
            <span class="ocr_line" title="bbox 50 160 60 170">
            <span class="ocrx_word" title="bbox 50 160 60 170; x_wconf 99">Cash</span>
            </span></p></div></div>"#,
        );
        assert_eq!(document.elements.len(), 1);
        assert_eq!(
            document.elements[0]
                .attributes
                .as_ref()
                .unwrap()
                .get(crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE)
                .map(String::as_str),
            Some("block_1")
        );
        original.ocr_internal_document = Some(document);
        assert!(accepted(&original, &sideways_candidate()));
    }
    #[test]
    fn language_selected_vertical_modes_are_not_replaced_by_automatic_layout_osd() {
        let configurations = [
            OcrConfig {
                language: vec!["jpn_vert".into()],
                ..Default::default()
            },
            OcrConfig {
                language: vec!["eng".into(), "kor_vert".into()],
                ..Default::default()
            },
            OcrConfig {
                language: vec![" eng + JPN_VERT ".into()],
                ..Default::default()
            },
            OcrConfig {
                tesseract_config: Some(crate::types::TesseractConfig {
                    language: vec!["jpn_vert".into()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        ];
        for config in configurations {
            assert!(
                !eligible(&config),
                "preserve the existing language-selected vertical PSM policy"
            );
        }
        assert!(eligible(&OcrConfig {
            language: vec!["eng".into(), "jpn".into()],
            ..Default::default()
        }));
        assert!(eligible(&OcrConfig::default()));
    }
}
