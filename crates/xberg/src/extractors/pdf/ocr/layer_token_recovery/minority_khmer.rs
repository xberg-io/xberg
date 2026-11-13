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
) -> Vec<AddedWord> {
    let Some((frame, candidates)) = painting_candidates(pdf, page, read) else {
        return Vec::new();
    };
    let Ok(Some(rasters)) = xberg_native_pdf::rendering::render_text_ink_rasters(
        pdf,
        page,
        INK_PROOF_DPI,
        limits.max_working_bytes,
        limits.cancelled,
    ) else {
        return Vec::new();
    };
    candidates
        .into_iter()
        .filter_map(|word| {
            if !glyph_ink_in(&rasters, &word.place, frame.media, INK_PROOF_DPI as f32 / 72.0) {
                return None;
            }
            Some((word.text, frame.to_raster(&word.place)?))
        })
        .collect()
}

pub(in crate::extractors::pdf::ocr) fn apply_minority_khmer(
    read: &mut ExtractedDocument,
    additions: &[AddedWord],
    cancelled: &dyn Fn() -> bool,
) {
    if !additions.is_empty() && !read.content.chars().any(is_khmer) {
        apply(read, &[], additions, cancelled);
    }
}

fn painting_candidates(pdf: &PdfDocument, page: usize, read: &ExtractedDocument) -> Option<(Frame, Vec<PlacedWord>)> {
    if read.content.chars().any(is_khmer) || !pdf.page_image_handles(page).ok()?.is_empty() {
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

    let ordered = xberg_native_pdf::pipeline::order_page_spans(pdf, page, spans).ok()?;
    let spans: Vec<_> = ordered.into_iter().map(|ordered| ordered.span).collect();
    let (words, _, _) = PdfDocument::assemble_positioned_words(&spans, media, None).ok()?;
    let candidates: Vec<_> = words
        .into_iter()
        .filter_map(|word| {
            let text: String = word.text.chars().filter(|ch| is_khmer(*ch)).collect();
            (!text.is_empty() && !word.text.chars().any(|ch| ch.is_ascii_alphanumeric())).then(|| PlacedWord {
                text,
                place: native_box(word.bbox),
                confidence: 1.0,
            })
        })
        .collect();
    if candidates.is_empty() || candidates.len() > MAX_ADDITIONS {
        return None;
    }
    if !candidates
        .iter()
        .flat_map(|word| word.text.chars())
        .eq(khmer.chars().filter(|ch| is_khmer(*ch)))
    {
        return None;
    }
    let frame = Frame { raster, media };
    (!region_contradicts_khmer(read, &frame, &candidates)?).then_some((frame, candidates))
}

fn region_contradicts_khmer(read: &ExtractedDocument, frame: &Frame, candidates: &[PlacedWord]) -> Option<bool> {
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
        if candidates.iter().any(|candidate| overlaps(&candidate.place, &place)) {
            if word.confidence.recognition >= MAX_CORRECTED_CONFIDENCE {
                local.push(word.text.as_str());
            }
        } else if word.text.chars().any(|ch| ch.is_ascii_alphabetic()) {
            corroborating += 1;
        }
    }
    if corroborating < MIN_LOCATED_WORDS {
        return None;
    }
    let khmer = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    Some(ink_probe::read_contradicts_layer(&khmer, &local.join(" ")))
}
