use super::*;
use crate::extractors::pdf::{ink_probe, unreadable_script};

pub(in crate::extractors::pdf::ocr) fn is_english_read(read: &ExtractedDocument) -> bool {
    matches!(
        &read.metadata.format,
        Some(crate::types::FormatMetadata::Ocr(metadata)) if metadata.language == "eng"
    )
}

fn is_khmer(ch: char) -> bool {
    ('\u{1780}'..='\u{17ff}').contains(&ch)
}

pub(in crate::extractors::pdf::ocr) fn plan_minority_khmer(
    pdf: &PdfDocument,
    page: usize,
    read: &ExtractedDocument,
    limits: &InkProofLimits<'_>,
) -> Option<Vec<AddedWord>> {
    let (frame, candidates) = painting_candidates(pdf, page, read)?;
    let rasters = xberg_native_pdf::rendering::render_text_ink_rasters(
        pdf,
        page,
        INK_PROOF_DPI,
        limits.max_working_bytes,
        limits.cancelled,
    )
    .ok()??;
    let scale = INK_PROOF_DPI as f32 / 72.0;
    Some(
        candidates
            .into_iter()
            .filter(|run| !(limits.cancelled)() && glyph_ink_in(&rasters, &run.place, frame.media, scale))
            .filter_map(|run| Some((run.text, frame.to_raster(&run.place)?)))
            .collect(),
    )
}

pub(in crate::extractors::pdf::ocr) fn apply_minority_khmer(
    read: &mut ExtractedDocument,
    additions: &[AddedWord],
    cancelled: &dyn Fn() -> bool,
) {
    let mut text = PageText::of(read);
    let page = read
        .ocr_elements
        .as_deref()
        .and_then(|words| words.first())
        .map_or(1, |word| word.page_number);
    for (run, place) in additions {
        if cancelled() {
            break;
        }
        let (left, top, width, height) = *place;
        let area = PixelBox {
            left: i64::from(left),
            top: i64::from(top),
            right: i64::from(left) + i64::from(width),
            bottom: i64::from(top) + i64::from(height),
        };
        if outside_tables(read, &area)
            && let Some(insertion) = line_insertion(&read.content, &text, &area)
        {
            insert_recovered_text(read, &mut text, run, *place, page, insertion);
        }
    }
}

fn line_insertion(content: &str, text: &PageText, area: &PixelBox) -> Option<Insertion> {
    let mut offset = 0;
    let mut above = None;
    let mut below = None;
    for line in content.split_inclusive('\n') {
        let place = line
            .split_whitespace()
            .filter_map(|word| text.once(word))
            .reduce(|left, right| left.union(&right));
        if let Some(place) = place {
            if place.bottom <= area.bottom && below.is_none() {
                above = Some(offset + line.trim_end_matches('\n').len());
            } else if place.top >= area.top {
                below.get_or_insert(offset);
            }
        }
        offset += line.len();
    }
    match (above, below) {
        (_, Some(offset)) => Some(Insertion::Block {
            offset,
            trailing_break: true,
        }),
        (Some(offset), None) => Some(Insertion::Block {
            offset,
            trailing_break: false,
        }),
        _ => None,
    }
}

fn painting_candidates(pdf: &PdfDocument, page: usize, read: &ExtractedDocument) -> Option<(Frame, Vec<PlacedWord>)> {
    if !pdf.page_image_handles(page).ok()?.is_empty() {
        return None;
    }
    let raster = super::super::document::processed_ocr_layout_dimensions(&read.metadata)?;
    let media = upright_media_box(pdf, page)?;
    if ((raster.0 as f32 / raster.1 as f32) / (media.width / media.height) - 1.0).abs() > MAX_ASPECT_DRIFT {
        return None;
    }
    let (painting, spans) = pdf.extract_painting_text_and_spans(page).ok()?;
    let letters = unreadable_script::script_letters(&painting, &["eng".into()])?;
    let [(unreadable_script::Script::Khmer, count)] = letters.uncovered.as_slice() else {
        return None;
    };
    if *count >= letters.covered || *count < 8 {
        return None;
    }
    let khmer: String = painting
        .chars()
        .filter(|ch| is_khmer(*ch) || ch.is_whitespace())
        .collect();
    if !unreadable_script::is_complete_khmer_retry_witness(&khmer) {
        return None;
    }

    if spans.len() > MAX_LAYER_WORDS {
        return None;
    }
    let ordered = xberg_native_pdf::pipeline::order_page_spans(pdf, page, spans).ok()?;
    // Position clustering can rearrange glyphs within a source span when vowel signs
    // and base letters have overlapping advances. Keep each painting span intact.
    let candidates: Vec<_> = ordered
        .iter()
        .filter_map(|ordered| {
            let span = &ordered.span;
            let text = khmer_run(&span.text);
            (!text.is_empty()
                && span.rotation_degrees == 0.0
                && !span.text.chars().any(|ch| ch.is_ascii_alphanumeric()))
            .then(|| PlacedWord {
                text,
                place: native_box(span.bbox),
                confidence: 1.0,
            })
        })
        .collect();
    if candidates.is_empty() || candidates.len() > MAX_ADDITIONS {
        return None;
    }
    if !candidates
        .iter()
        .flat_map(|run| run.text.chars().filter(|ch| is_khmer(*ch)))
        .eq(khmer.chars().filter(|ch| is_khmer(*ch)))
    {
        return None;
    }
    let frame = Frame { raster, media };
    let places: Vec<_> = ordered.iter().map(|item| native_box(item.span.bbox)).collect();
    let candidates = candidates
        .into_iter()
        .filter(|run| {
            places
                .iter()
                .filter(|place| overlaps(place, &run.place))
                .take(2)
                .count()
                == 1
        })
        .filter_map(|run| {
            region_allows_recovery(read, &frame, &run)
                .filter(|allowed| *allowed)
                .map(|_| run)
        })
        .collect();
    Some((frame, candidates))
}

fn khmer_run(text: &str) -> String {
    text.split_whitespace()
        .map(|word| word.chars().filter(|ch| is_khmer(*ch)).collect::<String>())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn region_allows_recovery(read: &ExtractedDocument, frame: &Frame, run: &PlacedWord) -> Option<bool> {
    let words = read.ocr_elements.as_deref()?;
    if words.len() > MAX_READ_WORDS {
        return None;
    }
    let mut local = Vec::new();
    let mut corroborating = 0;
    for word in words
        .iter()
        .filter(|word| word.level == OcrElementLevel::Word && is_iterator_word(word))
    {
        let place = ocr_word_in_page_points(word, frame.raster, frame.media)?;
        if overlaps(&run.place, &place) {
            if word.text.chars().any(is_khmer) {
                return Some(false);
            }
            if word.confidence.recognition >= MAX_CORRECTED_CONFIDENCE {
                local.push(word.text.as_str());
            }
        } else if word.text.chars().any(|ch| ch.is_ascii_alphabetic()) {
            corroborating += 1;
        }
    }
    let plain_words = local.iter().filter(|word| {
        let word = word.trim_matches(|ch: char| ch.is_ascii_punctuation());
        word.len() >= 4 && word.bytes().all(|byte| byte.is_ascii_alphabetic())
    });
    let english_conflict =
        plain_words.take(3).count() == 3 || ink_probe::read_contradicts_layer(&run.text, &local.join(" "));
    Some(corroborating >= MIN_LOCATED_WORDS && !english_conflict)
}
