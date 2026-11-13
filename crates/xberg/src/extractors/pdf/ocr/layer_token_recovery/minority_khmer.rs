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
    let (frame, font, candidates) = painting_candidates(pdf, page, read)?;
    let rasters = xberg_native_pdf::rendering::render_font_ink_rasters(
        pdf,
        page,
        INK_PROOF_DPI,
        limits.max_working_bytes,
        &font,
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
            && let Some(insertion) = line_insertion(read, &text, &area)
        {
            insert_recovered_text(read, &mut text, run, *place, page, insertion);
        }
    }
}

fn line_insertion(read: &ExtractedDocument, text: &PageText, area: &PixelBox) -> Option<(Insertion, Option<usize>)> {
    let mut offset = 0;
    let mut above = None;
    let mut below = None;
    for line in read.content.split_inclusive('\n') {
        let words = line
            .split_whitespace()
            .filter_map(|word| Some((word, text.once(word)?)));
        let mut place = None::<PixelBox>;
        let mut anchor = None;
        for (word, bbox) in words {
            place = Some(place.map_or(bbox, |current| current.union(&bbox)));
            anchor.get_or_insert(word);
        }
        if let (Some(place), Some(anchor)) = (place, anchor) {
            if place.bottom <= area.bottom {
                if below.is_some() {
                    return None;
                }
                above = Some((offset + line.trim_end_matches('\n').len(), anchor));
            } else if place.top >= area.top {
                below.get_or_insert((offset, anchor));
            }
        }
        offset += line.len();
    }
    let (offset, anchor, trailing_break) = match (above, below) {
        (_, Some((offset, anchor))) => (offset, anchor, true),
        (Some((offset, anchor)), None) => (offset, anchor, false),
        _ => return None,
    };
    let position = match &read.ocr_internal_document {
        None => None,
        Some(doc) => {
            let position = reading_position(&doc.elements, area);
            let owner = if trailing_break {
                position
            } else {
                position.checked_sub(1)?
            };
            let element = doc.elements.get(owner)?;
            if !element.text.split_whitespace().any(|word| word == anchor) {
                return None;
            }
            let bbox = element.bbox?;
            if (trailing_break && bbox.y0 < area.bottom as f64) || (!trailing_break && bbox.y1 > area.top as f64) {
                return None;
            }
            Some(position)
        }
    };
    Some((Insertion::Block { offset, trailing_break }, position))
}

fn painting_candidates(
    pdf: &PdfDocument,
    page: usize,
    read: &ExtractedDocument,
) -> Option<(Frame, String, Vec<PlacedWord>)> {
    if !pdf.page_image_handles(page).ok()?.is_empty() {
        return None;
    }
    let raster = super::super::document::processed_ocr_layout_dimensions(&read.metadata)?;
    let media = upright_media_box(pdf, page)?;
    if ((raster.0 as f32 / raster.1 as f32) / (media.width / media.height) - 1.0).abs() > MAX_ASPECT_DRIFT {
        return None;
    }
    let (painting, spans) = pdf.extract_painting_text_and_spans(page).ok()?;
    let khmer = minority_transcript(&painting)?;
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
    let mut font = None;
    let mut admitted = Vec::new();
    for run in candidates {
        let mut owners = ordered
            .iter()
            .zip(&places)
            .filter(|(_, place)| overlaps(place, &run.place));
        let Some((owner, _)) = owners.next() else { continue };
        if owners.next().is_some() {
            continue;
        }
        let remaining = unmatched_runs(read, &frame, run, &owner.span)?;
        if remaining.is_empty() {
            continue;
        }
        if font.as_ref().is_some_and(|font| font != &owner.span.font_name) {
            return None;
        }
        font.get_or_insert_with(|| owner.span.font_name.clone());
        admitted.extend(remaining);
    }
    let font = font?;
    if !isolated_outline_font(pdf, page, &font) {
        return None;
    }
    Some((frame, font, admitted))
}

fn minority_transcript(painting: &str) -> Option<String> {
    let letters = unreadable_script::script_letters(painting, &["eng".into()])?;
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
    unreadable_script::is_complete_khmer_retry_witness(&khmer).then_some(khmer)
}

fn isolated_outline_font(pdf: &PdfDocument, page: usize, font: &str) -> bool {
    use xberg_native_pdf::content::{Operator, parse_content_stream};
    let check = || {
        let resources = pdf.get_page_resources(page).ok()?;
        let resources = resources.as_dict()?;
        if ["XObject", "Pattern", "ExtGState"]
            .iter()
            .any(|key| resources.contains_key(*key))
        {
            return Some(false);
        }
        let fonts = pdf.resolve_object(resources.get("Font")?).ok()?;
        let selected = pdf.resolve_object(fonts.as_dict()?.get(font)?).ok()?;
        if selected.as_dict()?.get("Subtype")?.as_name()? == "Type3" {
            return Some(false);
        }
        let data = pdf.get_page_content_data(page).ok()?;
        let operators = parse_content_stream(&data).ok()?;
        Some(!operators.iter().any(|operator| matches!(operator, Operator::Do { .. })))
    };
    check() == Some(true)
}

fn khmer_run(text: &str) -> String {
    text.split_whitespace()
        .map(|word| word.chars().filter(|ch| is_khmer(*ch)).collect::<String>())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn unmatched_runs(
    read: &ExtractedDocument,
    frame: &Frame,
    run: PlacedWord,
    span: &xberg_native_pdf::layout::TextSpan,
) -> Option<Vec<PlacedWord>> {
    let words = read.ocr_elements.as_deref()?;
    if words.len() > MAX_READ_WORDS {
        return None;
    }
    let mut local = Vec::new();
    let mut matched = Vec::new();
    let mut corroborating = 0;
    for word in words
        .iter()
        .filter(|word| word.level == OcrElementLevel::Word && is_iterator_word(word))
    {
        let place = ocr_word_in_page_points(word, frame.raster, frame.media)?;
        if overlaps(&run.place, &place) {
            if word.text.chars().any(is_khmer) {
                matched.push((khmer_run(&word.text), place));
            } else if word.confidence.recognition >= MAX_CORRECTED_CONFIDENCE {
                local.push(word.text.as_str());
            }
        } else if word.text.chars().any(|ch| ch.is_ascii_alphabetic()) {
            corroborating += 1;
        }
    }
    let latin_label = local.iter().any(|word| {
        let word = word.trim_matches(|ch: char| ch.is_ascii_punctuation());
        word.len() >= 4 && word.bytes().all(|byte| byte.is_ascii_alphabetic())
    });
    if corroborating < MIN_LOCATED_WORDS
        || latin_label
        || ink_probe::read_contradicts_layer(&run.text, &local.join(" "))
    {
        return Some(Vec::new());
    }
    if matched.is_empty() {
        return Some(vec![run]);
    }
    Some(unmatched_occurrences(span, &matched).unwrap_or_default())
}

fn unmatched_occurrences(
    span: &xberg_native_pdf::layout::TextSpan,
    matched: &[(String, LineBox)],
) -> Option<Vec<PlacedWord>> {
    let count = span.text.chars().count();
    if span.char_x_offsets.len() != count || span.char_widths.len() != count {
        return None;
    }
    let chars = span.to_chars();
    let words: Vec<_> = chars
        .split(|glyph| glyph.char.is_whitespace())
        .filter_map(|glyphs| {
            let text: String = glyphs
                .iter()
                .map(|glyph| glyph.char)
                .filter(|ch| is_khmer(*ch))
                .collect();
            let place = glyphs
                .iter()
                .map(|glyph| native_box(glyph.bbox))
                .reduce(|a, b| LineBox {
                    left: a.left.min(b.left),
                    right: a.right.max(b.right),
                    top: a.top.min(b.top),
                    bottom: a.bottom.max(b.bottom),
                })?;
            (!text.is_empty()).then_some(PlacedWord {
                text,
                place,
                confidence: 1.0,
            })
        })
        .collect();
    let mut used = vec![false; words.len()];
    for (text, place) in matched {
        let mut overlapping = words
            .iter()
            .enumerate()
            .filter(|(_, word)| overlaps(place, &word.place));
        let (index, word) = overlapping.next()?;
        if overlapping.next().is_some() || used[index] || word.text != *text {
            return None;
        }
        used[index] = true;
    }
    Some(
        words
            .into_iter()
            .zip(used)
            .filter_map(|(word, used)| (!used).then_some(word))
            .collect(),
    )
}

#[cfg(test)]
#[test]
fn partial_khmer_read_spends_one_of_two_identical_occurrences() {
    let span = xberg_native_pdf::layout::TextSpan {
        text: "ក ក".into(),
        bbox: Rect::new(10.0, 20.0, 30.0, 10.0),
        char_x_offsets: vec![10.0, 20.0, 30.0],
        char_widths: vec![10.0; 3],
        ..Default::default()
    };
    let read = ("ក".to_owned(), native_box(Rect::new(10.0, 20.0, 10.0, 10.0)));
    let remaining = unmatched_occurrences(&span, &[read.clone()]).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].text, "ក");
    assert_eq!(remaining[0].place.left, 30.0);
    assert!(unmatched_occurrences(&span, &[read.clone(), read]).is_none());
}
