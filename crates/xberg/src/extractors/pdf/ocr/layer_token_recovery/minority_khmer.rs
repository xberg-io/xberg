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

pub(in crate::extractors::pdf::ocr) fn contains_khmer(text: &str) -> bool {
    text.chars().any(is_khmer)
}

pub(in crate::extractors::pdf::ocr) fn minority_khmer_supplement(
    pdf: &PdfDocument,
    page: usize,
    read: &ExtractedDocument,
    limits: &InkProofLimits<'_>,
) -> Option<String> {
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
    candidates
        .iter()
        .all(|run| !(limits.cancelled)() && glyph_ink_in(&rasters, &run.place, frame.media, scale))
        .then(|| {
            candidates
                .into_iter()
                .map(|run| run.text)
                .collect::<Vec<_>>()
                .join("\n")
        })
}

fn painting_candidates(pdf: &PdfDocument, page: usize, read: &ExtractedDocument) -> Option<(Frame, Vec<PlacedWord>)> {
    if contains_khmer(&read.content) || !pdf.page_image_handles(page).ok()?.is_empty() {
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
    // Position clustering can rearrange glyphs within a source span when vowel signs
    // and base letters have overlapping advances. Keep each painting span intact.
    let candidates: Vec<_> = ordered
        .into_iter()
        .filter_map(|ordered| {
            let span = ordered.span;
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
    (!region_contradicts_khmer(read, &frame, &candidates)?).then_some((frame, candidates))
}

fn khmer_run(text: &str) -> String {
    text.split_whitespace()
        .map(|word| word.chars().filter(|ch| is_khmer(*ch)).collect::<String>())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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
