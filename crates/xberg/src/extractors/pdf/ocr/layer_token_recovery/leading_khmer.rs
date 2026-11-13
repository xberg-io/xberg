use super::*;

#[derive(Debug)]
pub(in crate::extractors::pdf::ocr) struct LeadingKhmerRows {
    source: String,
    rows: Vec<usize>,
    published: Vec<String>,
}

impl LeadingKhmerRows {
    fn bind(source: String, additions: &[AddedWord]) -> Option<Self> {
        let mut rows = Vec::new();
        let mut runs = additions.iter();
        for (index, row) in source.trim().lines().enumerate() {
            if !row.chars().any(is_khmer) {
                continue;
            }
            let (run, _) = runs.next()?;
            if row.chars().any(|ch| ch.is_alphanumeric() && !is_khmer(ch))
                || !row
                    .chars()
                    .filter(|&ch| is_khmer(ch))
                    .eq(run.chars().filter(|&ch| is_khmer(ch)))
            {
                return None;
            }
            rows.push(index);
        }
        (runs.next().is_none() && !rows.is_empty()).then(|| Self {
            source,
            rows,
            published: additions.iter().map(|(run, _)| run.clone()).collect(),
        })
    }

    pub(in crate::extractors::pdf::ocr) fn prepend_policy(
        &self,
        layer: &str,
        flat: &str,
    ) -> crate::extractors::pdf::ocr::document::NativePrepend {
        use crate::extractors::pdf::ocr::document::NativePrepend;
        match self.for_prepend(layer, flat) {
            Some((source, read)) => NativePrepend::Recovered(source, read),
            None => NativePrepend::Abstain,
        }
    }

    fn for_prepend(&self, layer: &str, flat: &str) -> Option<(String, String)> {
        if self.source.trim() != layer.trim() {
            return None;
        }
        let mut lines = flat.split_inclusive('\n');
        for row in &self.published {
            if lines.find(|line| !line.trim().is_empty())?.trim() != row {
                return None;
            }
        }
        // Blank the occupied lines so they cannot also cover an unclaimed native
        // repeat. Keep line indices for any OCR rows the prepend later replaces. ~keep
        let consumed = flat.split_inclusive('\n').count() - lines.clone().count();
        let mut read = "\n".repeat(consumed);
        for line in lines {
            read.push_str(line);
        }
        let source = layer
            .trim()
            .lines()
            .enumerate()
            .filter_map(|(index, line)| self.rows.binary_search(&index).is_err().then_some(line))
            .collect::<Vec<_>>()
            .join("\n");
        Some((source, read))
    }
}

pub(super) fn insert(
    read: &mut ExtractedDocument,
    additions: &[AddedWord],
    source: String,
    cancelled: &dyn Fn() -> bool,
) -> Option<LeadingKhmerRows> {
    let words = read.ocr_elements.as_ref()?;
    if cancelled()
        || read.content.chars().any(is_khmer)
        || !read.tables.is_empty()
        || !words_follow_block(words, additions)
        || read
            .ocr_internal_document
            .as_ref()
            .is_some_and(|doc| !doc.tables.is_empty())
    {
        return None;
    }
    let ownership = LeadingKhmerRows::bind(source, additions)?;
    let original_word_count = words.len();
    let page = words.first().map_or(1, |word| word.page_number);
    let mut text = PageText::of(read);
    for (token, place) in additions.iter().rev() {
        insert_recovered_text(
            read,
            &mut text,
            token,
            *place,
            page,
            (
                Insertion::Block {
                    offset: 0,
                    trailing_break: true,
                },
                Some(0),
            ),
        );
    }
    if let Some(words) = read.ocr_elements.as_mut() {
        words[original_word_count..].reverse();
        words.rotate_left(original_word_count);
    }
    Some(ownership)
}

fn words_follow_block(words: &[OcrElement], additions: &[AddedWord]) -> bool {
    let (Some((_, (_, top, _, _))), Some((_, (_, last_top, _, last_height)))) = (additions.first(), additions.last())
    else {
        return false;
    };
    let top = i64::from(*top);
    let bottom = i64::from(*last_top) + i64::from(*last_height);
    words
        .iter()
        .filter(|word| word.level == OcrElementLevel::Word)
        .all(|word| {
            let Some(place) = PixelBox::of(word) else { return false };
            place.top >= bottom
                || (place.top >= top && place.bottom <= bottom && is_khmer_misread(word, place, additions))
        })
}

// Misreads of the Khmer ink may remain in the OCR stream. An independent
// occurrence or one crossing either block edge makes the order ambiguous. ~keep
fn is_khmer_misread(word: &OcrElement, place: PixelBox, additions: &[AddedWord]) -> bool {
    if !is_iterator_word(word)
        || (word.confidence.recognition >= MAX_CORRECTED_CONFIDENCE
            && word.text.bytes().any(|byte| byte.is_ascii_alphabetic()))
    {
        return false;
    }
    let center_y = (place.top + place.bottom) / 2;
    additions
        .iter()
        .filter(|(_, (left, top, width, height))| {
            let (left, top, width, height) = (i64::from(*left), i64::from(*top), i64::from(*width), i64::from(*height));
            top <= center_y
                && center_y < top + height
                && (place.right.min(left + width) - place.left.max(left)) * 2 >= place.right - place.left
                && place.height() <= 2 * height
        })
        .take(2)
        .count()
        == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(text: &str, left: u32, top: u32, height: u32) -> OcrElement {
        OcrElement {
            text: text.into(),
            geometry: OcrBoundingGeometry::Rectangle {
                left,
                top,
                width: 12,
                height,
            },
            confidence: OcrConfidence {
                recognition: 0.4,
                detection: None,
            },
            level: OcrElementLevel::Word,
            rotation: None,
            page_number: 1,
            parent_id: None,
            backend_metadata: HashMap::from([("backend".into(), "tesseract-iterator".into())]),
        }
    }

    #[test]
    fn leading_recovery_rejects_ocr_crossing_or_interleaved_with_block() {
        let additions = [("ក".into(), (20, 20, 40, 20)), ("ខ".into(), (20, 60, 40, 20))];
        for ocr in [
            word("crosses end", 22, 65, 20),
            word("between rows", 22, 43, 10),
            word("beside row", 70, 63, 10),
            word("crosses start", 22, 9, 16),
        ] {
            assert!(!words_follow_block(&[ocr], &additions));
        }
        assert!(words_follow_block(
            &[word("misread", 22, 63, 10), word("below", 22, 80, 10)],
            &additions
        ));
    }

    #[test]
    fn prepend_spends_exact_source_rows_and_keeps_unclaimed_repeats() {
        let source = "ក� ខ!\n\nក� ខ!\nក� ខ!\nGarden club";
        let ownership = LeadingKhmerRows {
            source: source.into(),
            rows: vec![0, 3],
            published: vec!["កខ".into(), "កខ".into()],
        };
        let flat = "កខ\n\nកខ\n\nGarden club";
        assert_eq!(
            ownership.for_prepend(source, flat).map(|pair| pair.0).as_deref(),
            Some("\nក� ខ!\nGarden club")
        );
        let (remaining, read) = ownership.for_prepend(source, flat).unwrap();
        let first = crate::extractors::pdf::ocr::document::layer_lines_to_put_first(
            &remaining,
            &read,
            true,
            &Default::default(),
            None,
            crate::extractors::pdf::ocr::document::PlainOcrRows::every(),
        )
        .expect("the unclaimed occurrence still reaches prepend");
        assert_eq!(first.lines.chars().filter(|&ch| is_khmer(ch)).collect::<String>(), "កខ");
        assert_eq!(ownership.for_prepend(source, "កខ\nGarden club"), None);
        assert_eq!(ownership.for_prepend(&source.replace('!', "?"), flat), None);
    }
    #[test]
    fn mismatched_recovered_rows_do_not_prepend_to_final_output() {
        use crate::extractors::pdf::ocr::{document, scoring::OcrPageNoiseVerdict};
        let ownership = LeadingKhmerRows::bind("កខ!\nGarden club".into(), &[("កខ".into(), (20, 20, 40, 20))]).unwrap();
        for (source, flat) in [
            ("កខ?\nGarden club", "កខ\nGarden club"),
            ("កខ!\nGarden club", "Heading\nកខ\nGarden club"),
        ] {
            let layers = ahash::AHashMap::from([(1, source.into())]);
            let config = Default::default();
            let verdicts = [OcrPageNoiseVerdict {
                page_index: 0,
                discarded: false,
                low_confidence: true,
                fragmented_noise: false,
                dictionary_noise: false,
                fragmented_word_ratio: 0.0,
                word_count: 3,
                mean_confidence: None,
                dict_invalid_word_ratio: None,
            }];
            let mut text = flat.to_owned();
            let mut pages = [text.clone()];
            let mut doc = document::flat_ocr_page_document(flat);
            document::put_layers_first_in_run_with_recovered_rows(
                &mut text,
                &mut pages,
                Some(&mut doc),
                document::PageLayers {
                    lines: &layers,
                    boxes: &Default::default(),
                },
                &verdicts,
                0,
                &config,
                |_, source, flat| ownership.prepend_policy(source, flat),
            );
            assert_eq!(text, flat);
            assert_eq!(pages[0], flat);
            assert_eq!(
                doc.elements
                    .iter()
                    .map(|element| element.text.as_str())
                    .collect::<Vec<_>>(),
                [flat]
            );
            let mut control = flat.to_owned();
            let mut control_pages = [control.clone()];
            document::put_layers_first_in_run(
                &mut control,
                &mut control_pages,
                None,
                document::PageLayers {
                    lines: &layers,
                    boxes: &Default::default(),
                },
                &verdicts,
                0,
                &config,
            );
            assert_ne!(control, flat, "ordinary prepend must exercise the conflicting row");
        }
    }
}
