use super::*;
use std::collections::HashSet;

#[path = "latin_rows/source_glyphs.rs"]
mod source_glyphs;

const MAX_ROWS: usize = 128;
const MAX_ROW_CHARS: usize = 256;
const MIN_MATCHED_ROWS: usize = 4;
const MIN_ACCENTED_MATCHES: usize = 3;

struct RasterRow {
    text: String,
    place: LineBox,
    area: PixelBox,
    element: usize,
}

struct SourceRow {
    text: String,
    place: LineBox,
    words: Vec<PlacedWord>,
    owner: Option<usize>,
}

pub(in crate::extractors::pdf::ocr) fn recover_latin_rows(
    pdf: &PdfDocument,
    page: usize,
    read: &mut ExtractedDocument,
    decision_layer: Option<&str>,
    limits: &InkProofLimits<'_>,
) -> Option<()> {
    if !is_english_read(read) || !read.tables.is_empty() || read.document.is_some() || (limits.cancelled)() {
        return None;
    }
    let frame = Frame {
        raster: super::super::document::processed_ocr_layout_dimensions(&read.metadata)?,
        media: upright_media_box(pdf, page)?,
    };
    let aspect = frame.raster.0 as f32 / frame.raster.1 as f32;
    if (aspect / (frame.media.width / frame.media.height) - 1.0).abs() > MAX_ASPECT_DRIFT {
        return None;
    }
    let elements = read.ocr_elements.as_deref()?;
    let rows = raster_rows(elements, &frame)?;
    let source = source_rows(pdf, page, &rows)?;
    if !corroborates_latin_body(&source) {
        return None;
    }
    let rasters = xberg_native_pdf::rendering::render_text_ink_rasters(
        pdf,
        page,
        INK_PROOF_DPI,
        limits.max_working_bytes,
        limits.cancelled,
    )
    .ok()??;
    let edits = row_edits(&source, &rows, &rasters, &frame, &read.content, limits.cancelled)?;
    let content = rewrite_rows(&read.content, &edits)?;
    if !keeps_layer_decision(&read.content, &content, decision_layer, limits) {
        return None;
    }
    let public = rewrite_elements(elements, &edits, &frame)?;
    let structured = match read.ocr_internal_document.as_ref() {
        Some(document) => Some(rewrite_internal(document, &edits, &frame)?),
        None => None,
    };
    if (limits.cancelled)() {
        return None;
    }
    read.content = content;
    read.ocr_elements = Some(public);
    read.ocr_internal_document = structured;
    Some(())
}

fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn raster_rows(elements: &[OcrElement], frame: &Frame) -> Option<Vec<RasterRow>> {
    let mut rows = Vec::new();
    if elements.len() > MAX_READ_WORDS {
        return None;
    }
    for (element, row) in elements
        .iter()
        .enumerate()
        .filter(|(_, e)| e.level == OcrElementLevel::Line)
    {
        if rows.len() == MAX_ROWS || !is_iterator_word(row) {
            return None;
        }
        rows.push(RasterRow {
            text: normalize(&row.text),
            place: ocr_word_in_page_points(row, frame.raster, frame.media)?,
            area: PixelBox::of(row)?,
            element,
        });
    }
    (!rows.is_empty()).then_some(rows)
}

fn latin_accent(ch: char) -> char {
    match ch {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        _ => ch,
    }
}

fn row_letters(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .filter(|c| c.is_alphanumeric())
        .map(latin_accent)
        .collect()
}

fn has_accent(text: &str) -> bool {
    text.chars()
        .flat_map(char::to_lowercase)
        .any(|ch| latin_accent(ch) != ch)
}

fn resembles_row(native: &str, read: &str, glyphs_confirmed: bool) -> bool {
    if read.len() > MAX_ROW_CHARS * 4 {
        return false;
    }
    let native_letters = row_letters(native);
    let read_letters = row_letters(read);
    native_letters.len() >= 24
        && strsim::normalized_levenshtein(&native_letters, &read_letters) >= 0.85
        && corroborates_words(native, read, glyphs_confirmed)
}

// Match one native word to one or two adjacent iterator words. Every change must
// be explained by accent/case folding or a small set of glyph confusions.
fn corroborates_words(native: &str, read: &str, glyphs_confirmed: bool) -> bool {
    let native: Vec<_> = native.split_whitespace().map(row_letters).collect();
    let read: Vec<_> = read.split_whitespace().map(row_letters).collect();
    let mut reachable = vec![false; read.len() + 1];
    reachable[0] = true;
    for word in native {
        let mut next = vec![false; read.len() + 1];
        for (at, first) in read.iter().enumerate().filter(|(at, _)| reachable[*at]) {
            if corroborates_word(&word, first, glyphs_confirmed) {
                next[at + 1] = true;
            }
            if let Some(second) = read.get(at + 1)
                && !first.is_empty()
                && !second.is_empty()
                && corroborates_word(&word, &format!("{first}{second}"), glyphs_confirmed)
            {
                next[at + 2] = true;
            }
        }
        reachable = next;
    }
    reachable[read.len()]
}

fn corroborates_word(native: &str, read: &str, glyphs_confirmed: bool) -> bool {
    if native == read {
        return true;
    }
    if !glyphs_confirmed || native.len() < 3 || !native.is_ascii() || !read.is_ascii() {
        return false;
    }
    const CONFUSIONS: &[(&str, &str)] = &[
        ("n", "f"),
        ("n", "fi"),
        ("n", "h"),
        ("o", "6"),
        ("o", "d"),
        ("o", "de"),
        ("o", "e"),
        ("o", "oe"),
        ("o", "6e"),
    ];
    CONFUSIONS.iter().any(|(expected, observed)| {
        native.match_indices(expected).any(|(at, _)| {
            read.strip_prefix(&native[..at])
                .and_then(|tail| tail.strip_prefix(observed))
                == Some(&native[at + expected.len()..])
        })
    })
}

fn source_rows(pdf: &PdfDocument, page: usize, rows: &[RasterRow]) -> Option<Vec<SourceRow>> {
    let painting = pdf.extract_text_without_nonpainting_glyphs(page).ok()?;
    let painted = normalize(&painting);
    let lines = pdf.extract_text_lines(page).ok()?;
    if lines.len() > MAX_ROWS {
        return None;
    }
    let glyphs = source_glyphs::painted_standard_rows(pdf, page).unwrap_or_default();
    let mut source = Vec::new();
    for line in lines {
        let text = normalize(&line.text);
        if !is_latin_prose(&text) || !unique_phrase(&painted, &text) {
            continue;
        }
        let place = native_box(line.bbox);
        let glyphs_confirmed = source_glyphs::confirms_row(&glyphs, &text, &place);
        let owner = row_owner(&text, &place, rows, glyphs_confirmed);
        source.push(SourceRow {
            text,
            place,
            words: line
                .words
                .into_iter()
                .map(|word| PlacedWord {
                    text: word.text,
                    place: native_box(word.bbox),
                    confidence: 1.0,
                })
                .collect(),
            owner,
        });
    }
    source.sort_by(|left, right| right.place.top.total_cmp(&left.place.top));
    Some(source)
}

fn is_latin_prose(text: &str) -> bool {
    text.len() <= MAX_ROW_CHARS
        && text.split_whitespace().count() >= 5
        && text.chars().all(|ch| {
            ch.is_ascii() && !ch.is_ascii_control()
                || ch.to_lowercase().all(|lower| latin_accent(lower) != lower)
                || matches!(ch, '¿' | '¡' | '’' | '“' | '”')
        })
        && !text.chars().any(|ch| ch.is_ascii_digit())
}

fn unique_phrase(surface: &str, phrase: &str) -> bool {
    let mut matches = surface.match_indices(phrase).filter(|(at, _)| {
        (*at == 0 || surface.as_bytes()[at - 1] == b' ')
            && (at + phrase.len() == surface.len() || surface.as_bytes()[at + phrase.len()] == b' ')
    });
    matches.next().is_some() && matches.next().is_none()
}

fn row_owner(text: &str, place: &LineBox, rows: &[RasterRow], glyphs_confirmed: bool) -> Option<usize> {
    let mut owners = rows.iter().enumerate().filter(|(_, row)| place.owns(&row.place));
    let (index, row) = owners.next()?;
    (owners.next().is_none() && resembles_row(text, &row.text, glyphs_confirmed)).then_some(index)
}

fn corroborates_latin_body(source: &[SourceRow]) -> bool {
    let matched = source.iter().filter_map(|row| row.owner.map(|owner| (owner, row)));
    let mut previous = None;
    let mut count = 0;
    let mut accented = 0;
    for (owner, row) in matched {
        if previous.is_some_and(|previous| owner <= previous) {
            return false;
        }
        previous = Some(owner);
        count += 1;
        accented += usize::from(has_accent(&row.text));
    }
    count >= MIN_MATCHED_ROWS && accented >= MIN_ACCENTED_MATCHES && count * 2 >= source.len()
}

struct RowEdit<'a> {
    anchor: &'a RasterRow,
    replacement: &'a SourceRow,
    preceding: Vec<&'a SourceRow>,
}

fn row_edits<'a>(
    source: &'a [SourceRow],
    rows: &'a [RasterRow],
    rasters: &xberg_native_pdf::rendering::TextInkRasters,
    frame: &Frame,
    content: &str,
    cancelled: &dyn Fn() -> bool,
) -> Option<Vec<RowEdit<'a>>> {
    let has_ink = |row: &SourceRow| {
        !row.words.is_empty()
            && row
                .words
                .iter()
                .all(|word| glyph_ink_in(rasters, &word.place, frame.media, INK_PROOF_DPI as f32 / 72.0))
    };
    plan_row_edits(source, rows, content, &has_ink, cancelled)
}

fn plan_row_edits<'a>(
    source: &'a [SourceRow],
    rows: &'a [RasterRow],
    content: &str,
    has_ink: &dyn Fn(&SourceRow) -> bool,
    cancelled: &dyn Fn() -> bool,
) -> Option<Vec<RowEdit<'a>>> {
    let mut edits = Vec::new();
    let mut omitted = Vec::new();
    let mut previous: Option<usize> = None;
    for row in source {
        if cancelled() {
            return None;
        }
        let Some(owner) = row.owner else {
            omitted.clear();
            previous = None;
            continue;
        };
        let anchor = &rows[owner];
        let presence = row_presence(content, &anchor.text);
        if matches!(presence, RowPresence::Repeated) {
            omitted.clear();
            previous = None;
            continue;
        }
        if matches!(presence, RowPresence::Absent) {
            // Iterator rows survive the dictionary filter even when the transcript drops them.
            // Restore only a short gap bounded by consecutive, corroborated rows.
            if previous.is_some_and(|prior| prior + 1 == owner) && omitted.len() < 3 && has_ink(row) {
                omitted.push(row);
                previous = Some(owner);
            } else {
                omitted.clear();
                previous = None;
            }
            continue;
        }
        let preceding = if previous.is_some_and(|prior| prior + 1 == owner) {
            std::mem::take(&mut omitted)
        } else {
            omitted.clear();
            Vec::new()
        };
        previous = Some(owner);
        if (row.text != anchor.text || !preceding.is_empty()) && has_ink(row) {
            edits.push(RowEdit {
                anchor,
                replacement: row,
                preceding,
            });
        }
    }
    (!edits.is_empty()).then_some(edits)
}

fn keeps_layer_decision(before: &str, after: &str, layer: Option<&str>, limits: &InkProofLimits<'_>) -> bool {
    let Some(native) = layer else { return true };
    let confirms = |text| {
        super::super::document::ocr_confirms_layer(
            native,
            &super::super::scoring::repair_ocr_list_markers(text),
            limits.thresholds,
        )
    };
    // Corroboration for later page selection must come from the original read.
    confirms(before) || !confirms(after)
}

#[derive(Debug, PartialEq, Eq)]
enum RowPresence {
    Absent,
    Unique(std::ops::Range<usize>),
    Repeated,
}

fn row_span(surface: &str, row: &str) -> Option<std::ops::Range<usize>> {
    match row_presence(surface, row) {
        RowPresence::Unique(span) => Some(span),
        RowPresence::Absent | RowPresence::Repeated => None,
    }
}

fn row_presence(surface: &str, row: &str) -> RowPresence {
    let mut tokens = Vec::new();
    let mut start = None;
    for (at, ch) in surface.char_indices().chain(std::iter::once((surface.len(), ' '))) {
        if ch.is_whitespace() {
            if let Some(from) = start.take() {
                tokens.push(from..at);
            }
        } else if start.is_none() {
            start = Some(at);
        }
    }
    let wanted: Vec<_> = row.split_whitespace().collect();
    if wanted.is_empty() {
        return RowPresence::Absent;
    }
    let mut matches = tokens.windows(wanted.len()).filter(|window| {
        window
            .iter()
            .zip(&wanted)
            .all(|(span, text)| &surface[span.clone()] == *text)
    });
    match (matches.next(), matches.next()) {
        (None, _) => RowPresence::Absent,
        (Some(_), Some(_)) => RowPresence::Repeated,
        (Some(range), None) => RowPresence::Unique(range[0].start..range[range.len() - 1].end),
    }
}

fn edit_text(edit: &RowEdit<'_>) -> String {
    edit.preceding
        .iter()
        .map(|row| row.text.as_str())
        .chain(std::iter::once(edit.replacement.text.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn rewrite_rows(surface: &str, edits: &[RowEdit<'_>]) -> Option<String> {
    let mut changes = edits
        .iter()
        .map(|edit| Some((row_span(surface, &edit.anchor.text)?, edit_text(edit))))
        .collect::<Option<Vec<_>>>()?;
    changes.sort_unstable_by_key(|(span, _)| span.start);
    if changes.windows(2).any(|pair| pair[0].0.end > pair[1].0.start) {
        return None;
    }
    let mut output = surface.to_owned();
    for (span, text) in changes.into_iter().rev() {
        output.replace_range(span, &text);
    }
    Some(output)
}

fn rewrite_internal(
    document: &crate::types::internal::InternalDocument,
    edits: &[RowEdit<'_>],
    frame: &Frame,
) -> Option<crate::types::internal::InternalDocument> {
    if !document.tables.is_empty() || document.pre_rendered_content.is_some() || document.prebuilt_pages.is_some() {
        return None;
    }
    let mut updated = document.clone();
    for edit in edits {
        let mut found = updated.elements.iter_mut().filter_map(|element| {
            let span = row_span(&element.text, &edit.anchor.text)?;
            Some((element, span))
        });
        let (element, span) = found.next()?;
        if found.next().is_some()
            || !element.annotations.is_empty()
            || !matches!(element.kind, ElementKind::Paragraph | ElementKind::OcrText { .. })
        {
            return None;
        }
        element.text.replace_range(span, &edit_text(edit));
    }
    if let Some(elements) = &document.prebuilt_ocr_elements {
        updated.prebuilt_ocr_elements = Some(rewrite_elements(elements, edits, frame)?);
    }
    Some(updated)
}

fn rewrite_elements(elements: &[OcrElement], edits: &[RowEdit<'_>], frame: &Frame) -> Option<Vec<OcrElement>> {
    let mut output = Vec::new();
    for edit in edits {
        let mut text = Vec::new();
        for word in elements.iter().filter(|element| element.level == OcrElementLevel::Word) {
            let area = PixelBox::of(word)?;
            if owns_word(&edit.anchor.area, &area) {
                if !is_iterator_word(word) {
                    return None;
                }
                text.push(word.text.as_str());
            }
        }
        if normalize(&text.join(" ")) != edit.anchor.text {
            return None;
        }
    }
    let replaced: HashSet<_> = edits.iter().map(|edit| edit.anchor.element).collect();
    let omitted: Vec<_> = edits.iter().flat_map(|edit| &edit.preceding).collect();
    let is_omitted = |element: &OcrElement| {
        ocr_word_in_page_points(element, frame.raster, frame.media)
            .is_some_and(|place| omitted.iter().any(|row| row.place.owns(&place)))
    };
    for (index, element) in elements.iter().enumerate() {
        match element.level {
            OcrElementLevel::Word => {
                let area = PixelBox::of(element)?;
                if !is_omitted(element) && !edits.iter().any(|edit| owns_word(&edit.anchor.area, &area)) {
                    output.push(element.clone());
                }
            }
            OcrElementLevel::Line if replaced.contains(&index) => {
                let edit = edits.iter().find(|edit| edit.anchor.element == index)?;
                for row in edit.preceding.iter().copied().chain(std::iter::once(edit.replacement)) {
                    output.push(recovered_element(
                        element,
                        &row.text,
                        &row.place,
                        OcrElementLevel::Line,
                        frame,
                    )?);
                    for word in &row.words {
                        output.push(recovered_element(
                            element,
                            &word.text,
                            &word.place,
                            OcrElementLevel::Word,
                            frame,
                        )?);
                    }
                }
            }
            OcrElementLevel::Line if is_omitted(element) => {}
            OcrElementLevel::Line => output.push(element.clone()),
            OcrElementLevel::Page | OcrElementLevel::Block => {
                output.push(element.clone());
            }
        }
    }
    let lines: Vec<_> = output
        .iter()
        .filter(|element| element.level == OcrElementLevel::Line)
        .filter_map(|element| Some((PixelBox::of(element)?, element.text.clone())))
        .collect();
    for element in output
        .iter_mut()
        .filter(|element| matches!(element.level, OcrElementLevel::Page | OcrElementLevel::Block))
    {
        let area = PixelBox::of(element)?;
        let text = lines
            .iter()
            .filter(|(line, _)| owns_word(&area, line))
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>();
        if !text.is_empty() {
            element.text = text.join("\n");
        }
    }
    Some(output)
}

fn owns_word(line: &PixelBox, word: &PixelBox) -> bool {
    let x = (word.left + word.right) / 2;
    let y = (word.top + word.bottom) / 2;
    (line.left..line.right).contains(&x) && (line.top..line.bottom).contains(&y)
}

fn recovered_element(
    anchor: &OcrElement,
    text: &str,
    place: &LineBox,
    level: OcrElementLevel,
    frame: &Frame,
) -> Option<OcrElement> {
    let (left, top, width, height) = frame.to_raster(place)?;
    Some(OcrElement {
        text: text.to_owned(),
        geometry: OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        },
        level,
        page_number: anchor.page_number,
        backend_metadata: HashMap::from([("backend".into(), RECOVERY_BACKEND.into())]),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_rewrite_leaves_equal_footer_token_at_a_different_position() {
        let anchor = RasterRow {
            text: "Este ano la poblacion crecio".into(),
            place: LineBox {
                left: 0.0,
                top: 0.0,
                right: 100.0,
                bottom: 12.0,
            },
            area: PixelBox {
                left: 0,
                top: 0,
                right: 100,
                bottom: 12,
            },
            element: 0,
        };
        let replacement = SourceRow {
            text: "Este año la población creció".into(),
            place: anchor.place,
            words: Vec::new(),
            owner: Some(0),
        };
        let edit = RowEdit {
            anchor: &anchor,
            replacement: &replacement,
            preceding: Vec::new(),
        };
        assert_eq!(
            rewrite_rows("Este ano la poblacion crecio\nControl visible ano", &[edit]).as_deref(),
            Some("Este año la población creció\nControl visible ano")
        );
        assert!(
            row_span(
                "Este ano la poblacion crecio\nEste ano la poblacion crecio",
                &anchor.text
            )
            .is_none()
        );
    }

    #[test]
    fn similar_row_cannot_replace_an_unrelated_word_at_the_same_position() {
        let native = "El jardín de la plaza tendrá un sistema de riego automático.";
        let wrong = "El jardín de la feria tendrá un sistema de riego automático.";
        assert!(strsim::normalized_levenshtein(&row_letters(native), &row_letters(wrong)) >= 0.85);
        let place = LineBox {
            left: 0.0,
            top: 0.0,
            right: 100.0,
            bottom: 12.0,
        };
        let row = |text: &str| RasterRow {
            text: text.into(),
            place,
            area: PixelBox {
                left: 0,
                top: 0,
                right: 100,
                bottom: 12,
            },
            element: 0,
        };
        assert_eq!(row_owner(native, &place, &[row(wrong)], true), None);
        assert_eq!(
            row_owner(
                native,
                &place,
                &[row("El jardi n de la plaza tendra un sistema de riego automatico.")],
                false
            ),
            Some(0)
        );
    }

    #[test]
    fn only_absent_transcript_anchors_are_restored_between_neighbors() {
        let place = LineBox {
            left: 0.0,
            top: 0.0,
            right: 100.0,
            bottom: 12.0,
        };
        let area = PixelBox {
            left: 0,
            top: 0,
            right: 100,
            bottom: 12,
        };
        let rows: Vec<_> = [
            "Antes de la lectura.",
            "El jardin tendra riego.",
            "Después de la lectura.",
        ]
        .into_iter()
        .enumerate()
        .map(|(element, text)| RasterRow {
            text: text.into(),
            place,
            area,
            element,
        })
        .collect();
        let source: Vec<_> = [
            "Antes de la lectura.",
            "El jardín tendrá riego.",
            "Después de la lectura.",
        ]
        .into_iter()
        .enumerate()
        .map(|(owner, text)| SourceRow {
            text: text.into(),
            place,
            words: Vec::new(),
            owner: Some(owner),
        })
        .collect();
        let absent = "Antes de la lectura.\nDespués de la lectura.";
        let edits = plan_row_edits(&source, &rows, absent, &|_| true, &|| false).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].preceding.len(), 1);
        assert_eq!(edits[0].preceding[0].text, source[1].text);
        let repeated = "Antes de la lectura.\nEl jardin tendra riego.\nEl jardin tendra riego.\nDespués de la lectura.";
        assert!(plan_row_edits(&source, &rows, repeated, &|_| true, &|| false).is_none());
    }

    #[test]
    fn accent_fold_accepts_split_read_but_rejects_unrelated_row() {
        assert!(resembles_row(
            "El jardín de la plaza tendrá un sistema de riego automático.",
            "El jardi n de la plaza tendra un sistema de riego automatico.",
            false,
        ));
        assert!(!resembles_row(
            "El jardín de la plaza tendrá un sistema de riego automático.",
            "El informe anual estuvo disponible en la página oficial.",
            true,
        ));
    }
}
