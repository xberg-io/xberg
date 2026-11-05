//! A second Khmer read for the OCR pages of a PDF whose text layer is Khmer (GH#2509).
//!
//! The page keeps every word of the read with the configured languages; the Khmer words of a
//! read with `khm` added are put beside the words of the first read that they beat.

use crate::core::config::OcrConfig;
use crate::core::config::ocr::DEFAULT_OCR_LANGUAGE;
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel};

/// The Tesseract language that reads Khmer.
const KHMER_LANGUAGE: &str = "khm";

/// Fewest Khmer letters the text layer must hold before the document counts as Khmer. A few
/// stray letters in another document must not double its OCR time. ~keep
const MIN_KHMER_LAYER_LETTERS: usize = 20;

/// Smallest share of the smaller box that two word boxes must cover together to be the same
/// place on the page. ~keep
const MIN_BOX_OVERLAP: f64 = 0.3;

fn is_khmer_letter(c: char) -> bool {
    ('\u{1780}'..='\u{17FF}').contains(&c)
}

/// `true` when the configured languages already read Khmer, or are the `all` wildcard.
fn languages_read_khmer(languages: &[String]) -> bool {
    languages
        .iter()
        .any(|lang| matches!(lang.trim().to_ascii_lowercase().as_str(), "khm" | "km" | "all" | "*"))
}

/// `true` when the OCR pages of a document with text layer `native_text` get the second Khmer
/// read: the backend is Tesseract, the layer holds Khmer, the configured languages do not read
/// Khmer, and the Khmer model is on disk (a missing model is never downloaded for this read).
pub(crate) fn wants_khmer_read(ocr: &OcrConfig, native_text: &str) -> bool {
    if ocr.backend != "tesseract" {
        return false;
    }
    let languages = ocr.effective_tesseract_language();
    if languages_read_khmer(&languages) {
        return false;
    }
    let letters = native_text
        .chars()
        .filter(|&c| is_khmer_letter(c))
        .take(MIN_KHMER_LAYER_LETTERS);
    if letters.count() < MIN_KHMER_LAYER_LETTERS {
        return false;
    }
    khmer_model_on_disk(ocr, &languages)
}

#[cfg(feature = "ocr")]
fn khmer_model_on_disk(ocr: &OcrConfig, languages: &[String]) -> bool {
    let mut wanted = languages.to_vec();
    wanted.push(KHMER_LANGUAGE.to_string());
    let env = crate::ocr::processor::validation::TessdataEnv::from_process();
    crate::ocr::processor::validation::existing_tessdata_dir_for(&wanted, ocr.tessdata_path.as_deref(), &env).is_some()
}

#[cfg(not(feature = "ocr"))]
fn khmer_model_on_disk(_ocr: &OcrConfig, _languages: &[String]) -> bool {
    false
}

/// A copy of `ocr` whose Tesseract languages also hold `khm`. The language goes to the field
/// Tesseract reads, so the precedence of the two language fields stays as it is (#1572).
pub(crate) fn with_khmer_language(ocr: &OcrConfig) -> OcrConfig {
    let mut ocr = ocr.clone();
    let outer_is_default = ocr.language.len() == 1 && ocr.language[0] == DEFAULT_OCR_LANGUAGE;
    match ocr.tesseract_config.as_mut() {
        Some(tesseract) if outer_is_default && !tesseract.language.is_empty() => {
            tesseract.language.push(KHMER_LANGUAGE.to_string());
        }
        _ => ocr.language.push(KHMER_LANGUAGE.to_string()),
    }
    ocr
}

/// One word of a read: its text, its box as (left, top, right, bottom), and its confidence.
struct Word<'a> {
    text: &'a str,
    rect: (f64, f64, f64, f64),
    confidence: f64,
}

fn word_of(element: &OcrElement) -> Option<Word<'_>> {
    if element.level != OcrElementLevel::Word || element.text.trim().is_empty() {
        return None;
    }
    let OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    } = element.geometry
    else {
        return None;
    };
    let (left, top) = (f64::from(left), f64::from(top));
    Some(Word {
        text: element.text.trim(),
        rect: (left, top, left + f64::from(width), top + f64::from(height)),
        confidence: element.confidence.recognition,
    })
}

fn overlap(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> f64 {
    let width = (a.2.min(b.2) - a.0.max(b.0)).max(0.0);
    let height = (a.3.min(b.3) - a.1.max(b.1)).max(0.0);
    let area = |r: (f64, f64, f64, f64)| ((r.2 - r.0) * (r.3 - r.1)).max(1.0);
    width * height / area(a).min(area(b))
}

fn read_with_khmer(element: &OcrElement) -> bool {
    element
        .backend_metadata
        .get("word_language")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|lang| lang == KHMER_LANGUAGE)
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0usize), |(sum, count), value| (sum + value, count + 1));
    if count == 0 { 0.0 } else { sum / count as f64 }
}

/// Groups of the Khmer words of the second read with the base words their boxes overlap: each
/// group is (Khmer word indices, base word indices), both in reading order.
fn overlap_groups(khmer: &[Word<'_>], base: &[Word<'_>]) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut parent: Vec<usize> = (0..khmer.len() + base.len()).collect();
    fn root(parent: &mut [usize], mut node: usize) -> usize {
        while parent[node] != node {
            parent[node] = parent[parent[node]];
            node = parent[node];
        }
        node
    }
    for (k, khmer_word) in khmer.iter().enumerate() {
        for (b, base_word) in base.iter().enumerate() {
            if overlap(khmer_word.rect, base_word.rect) >= MIN_BOX_OVERLAP {
                let (left, right) = (root(&mut parent, k), root(&mut parent, khmer.len() + b));
                parent[left] = right;
            }
        }
    }
    let mut groups: Vec<(usize, Vec<usize>, Vec<usize>)> = Vec::new();
    for node in 0..parent.len() {
        let group_root = root(&mut parent, node);
        let at = match groups.iter().position(|(r, _, _)| *r == group_root) {
            Some(at) => at,
            None => {
                groups.push((group_root, Vec::new(), Vec::new()));
                groups.len() - 1
            }
        };
        if node < khmer.len() {
            groups[at].1.push(node);
        } else {
            groups[at].2.push(node - khmer.len());
        }
    }
    groups
        .into_iter()
        .filter(|(_, k, b)| !k.is_empty() && !b.is_empty())
        .map(|(_, k, b)| (k, b))
        .collect()
}

/// A base word's place in the page text: (paragraph index, byte start, byte end).
type Span = (usize, usize, usize);

/// The first place at or after byte `from` of `text` where `word` stands as a whole token, with no
/// letter or digit touching it on either side.
fn find_token(text: &str, from: usize, word: &str) -> Option<usize> {
    let mut at = from;
    while let Some(found) = text.get(at..)?.find(word) {
        let start = at + found;
        let before = text[..start].chars().next_back();
        let after = text[start + word.len()..].chars().next();
        if !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric) {
            return Some(start);
        }
        at = start + word.chars().next().map_or(1, char::len_utf8);
    }
    None
}

/// The place of each base word in `paragraphs`, found in reading order; `None` for a word the
/// text does not hold after the word before it, such as a word of a line the noise filter dropped.
fn word_spans(paragraphs: &[String], base: &[Word<'_>]) -> Vec<Option<Span>> {
    let mut cursor = (0, 0);
    base.iter()
        .map(|word| {
            let span = (cursor.0..paragraphs.len()).find_map(|paragraph| {
                let from = if paragraph == cursor.0 { cursor.1 } else { 0 };
                let start = find_token(&paragraphs[paragraph], from, word.text)?;
                Some((paragraph, start, start + word.text.len()))
            })?;
            cursor = (span.0, span.2);
            Some(span)
        })
        .collect()
}

/// The space between two words: a blank when their boxes share at least half the height of the
/// shorter one, else a line break.
fn gap(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> &'static str {
    let shared = (a.3.min(b.3) - a.1.max(b.1)).max(0.0);
    if shared * 2.0 >= (a.3 - a.1).min(b.3 - b.1) {
        " "
    } else {
        "\n"
    }
}

/// Where the Khmer words of a group go when the text holds none of its base words: after the
/// nearest earlier word the text holds, else before the nearest later one. `None` when the text
/// holds no base word at all.
fn insertion(group: &[usize], base: &[Word<'_>], spans: &[Option<Span>], khmer_text: &str) -> Option<(Span, String)> {
    let (first, last) = (*group.first()?, *group.last()?);
    if let Some((b, (paragraph, _, end))) = (0..first).rev().find_map(|b| spans[b].map(|span| (b, span))) {
        return Some((
            (paragraph, end, end),
            format!("{}{khmer_text}", gap(base[b].rect, base[first].rect)),
        ));
    }
    let (b, (paragraph, start, _)) = (last + 1..base.len()).find_map(|b| spans[b].map(|span| (b, span)))?;
    Some((
        (paragraph, start, start),
        format!("{khmer_text}{}", gap(base[last].rect, base[b].rect)),
    ))
}

/// The insertions that put the Khmer words into `paragraphs`, and the Khmer text of the groups
/// that have no place in them, which become a paragraph of their own. A group's Khmer words go
/// right after the last of its base words that the text holds. No base word is ever removed, so
/// every word and number of the first read stays. ~keep
fn khmer_edits(khmer: &[Word<'_>], base: &[Word<'_>], paragraphs: &[String]) -> (Vec<(Span, String)>, Vec<String>) {
    let spans = word_spans(paragraphs, base);
    let (mut edits, mut unplaced) = (Vec::new(), Vec::new());
    for (khmer_group, base_group) in overlap_groups(khmer, base) {
        let khmer_confidence = mean(khmer_group.iter().map(|&k| khmer[k].confidence));
        if khmer_confidence <= mean(base_group.iter().map(|&b| base[b].confidence)) {
            continue;
        }
        let khmer_text = khmer_group.iter().map(|&k| khmer[k].text).collect::<Vec<_>>().join(" ");
        match base_group.iter().rev().find_map(|&b| spans[b]) {
            Some((paragraph, _, end)) => edits.push(((paragraph, end, end), format!(" {khmer_text}"))),
            None => match insertion(&base_group, base, &spans, &khmer_text) {
                Some(edit) => edits.push(edit),
                None => unplaced.push(khmer_text),
            },
        }
    }
    (edits, unplaced)
}

/// Apply the insertions in `edits` to `paragraphs`, last place first, so that every byte offset
/// stays valid; two insertions at one place keep their order.
fn apply_edits(paragraphs: &mut [String], edits: Vec<(Span, String)>) {
    let mut ordered: Vec<(usize, (Span, String))> = edits.into_iter().enumerate().collect();
    ordered.sort_by_key(|(order, (span, _))| std::cmp::Reverse((*span, *order)));
    for (_, ((paragraph, start, end), text)) in ordered {
        paragraphs[paragraph].replace_range(start..end, &text);
    }
}

fn is_paragraph(element: &crate::types::internal::InternalElement) -> bool {
    !matches!(element.kind, crate::types::internal::ElementKind::PageBreak) && !element.text.is_empty()
}

/// The paragraphs of the page text. Plain output renders the content and every other format
/// renders the page's structured document, so both must take the same edits: `None` when the
/// content is not the structured document's paragraphs joined by blank lines, as on a page
/// whose content also renders an OCR table. ~keep
fn page_paragraphs(base: &ExtractedDocument) -> Option<Vec<String>> {
    let Some(document) = base.ocr_internal_document.as_ref() else {
        return Some(vec![base.content.clone()]);
    };
    let paragraphs: Vec<String> = document
        .elements
        .iter()
        .filter(|element| is_paragraph(element))
        .map(|element| element.text.clone())
        .collect();
    (paragraphs.join("\n\n") == base.content).then_some(paragraphs)
}

/// Write the edited `paragraphs` back to the content and to the structured document.
fn write_paragraphs(base: &mut ExtractedDocument, paragraphs: Vec<String>) {
    let kept: Vec<&str> = paragraphs
        .iter()
        .map(String::as_str)
        .filter(|text| !text.is_empty())
        .collect();
    base.content = kept.join("\n\n");
    let Some(document) = base.ocr_internal_document.as_mut() else {
        return;
    };
    let mut texts = paragraphs.into_iter();
    for element in document.elements.iter_mut().filter(|element| is_paragraph(element)) {
        if let Some(text) = texts.next() {
            element.text = text;
        }
    }
    for text in texts.filter(|text| !text.is_empty()) {
        push_paragraph(document, &text);
    }
}

fn push_paragraph(document: &mut crate::types::internal::InternalDocument, text: &str) {
    document.push_element(crate::types::internal::InternalElement::text(
        crate::types::internal::ElementKind::OcrText {
            level: OcrElementLevel::Block,
        },
        text,
        0,
    ));
}

/// Add `text` as a last paragraph of the content and of the structured document.
fn append_paragraph(base: &mut ExtractedDocument, text: &str) {
    if !base.content.is_empty() {
        base.content.push_str("\n\n");
    }
    base.content.push_str(text);
    if let Some(document) = base.ocr_internal_document.as_mut() {
        push_paragraph(document, text);
    }
}

/// Put the Khmer words of `khmer` into the page text of `base`, the read with the configured
/// languages. A group of overlapping words takes the Khmer words only when their mean confidence
/// beats the base words'; the Khmer words then go right after the group's base words. A group
/// whose base words the text does not hold gets its Khmer words next to the nearest word the
/// text holds, and when the text holds no base word (or is not its paragraphs), as a last
/// paragraph. No word of `base` is removed. Returns the number of groups that took Khmer words.
pub(crate) fn merge_khmer_words(base: &mut ExtractedDocument, khmer: &ExtractedDocument) -> usize {
    let khmer_words: Vec<Word<'_>> = khmer
        .ocr_elements
        .iter()
        .flatten()
        .filter(|element| read_with_khmer(element))
        .filter_map(word_of)
        .filter(|word| word.text.chars().any(is_khmer_letter))
        .collect();
    let base_elements = base.ocr_elements.take();
    let base_words: Vec<Word<'_>> = base_elements.iter().flatten().filter_map(word_of).collect();
    let paragraphs = page_paragraphs(base);
    let (edits, unplaced) = khmer_edits(&khmer_words, &base_words, paragraphs.as_deref().unwrap_or(&[]));
    let groups = edits.len() + unplaced.len();
    if groups > 0 {
        match paragraphs {
            Some(mut paragraphs) => {
                apply_edits(&mut paragraphs, edits);
                if !unplaced.is_empty() {
                    paragraphs.push(unplaced.join("\n"));
                }
                write_paragraphs(base, paragraphs);
            }
            None => append_paragraph(base, &unplaced.join("\n")),
        }
    }
    base.ocr_elements = base_elements;
    groups
}

/// Read one page as [`crate::extractors::image::process_whole_image_ocr_with_sparse_retry`]
/// does. With `second_khmer_read`, read it again with `khm` added and put the Khmer words in
/// with [`merge_khmer_words`]. A failed second read keeps the first read.
pub(crate) async fn read_page_with_khmer_layer(
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    image_bytes: std::sync::Arc<Vec<u8>>,
    configs: (&OcrConfig, &OcrConfig),
    retry_policy: crate::extractors::image::SparseRetryPolicy,
    cancel_token: Option<&crate::cancellation::CancellationToken>,
    second_khmer_read: bool,
) -> crate::Result<ExtractedDocument> {
    use crate::extractors::image::process_whole_image_ocr_with_sparse_retry as read;
    let (original, effective) = configs;
    let image = std::sync::Arc::clone(&image_bytes);
    let mut base = read(backend, image, original, effective, retry_policy, cancel_token).await?;
    // A pipeline hands the same page hints to every stage, a Paddle fallback stage included. ~keep
    if !second_khmer_read || backend.name() != "tesseract" {
        return Ok(base);
    }
    let (original, effective) = (with_khmer_language(original), with_khmer_language(effective));
    match read(backend, image_bytes, &original, &effective, retry_policy, cancel_token).await {
        Ok(khmer) => {
            merge_khmer_words(&mut base, &khmer);
        }
        Err(error @ crate::XbergError::Cancelled) => return Err(error),
        Err(error) => tracing::warn!(%error, "the Khmer read of a PDF page failed; the page keeps its first read"),
    }
    Ok(base)
}

#[cfg(all(test, feature = "ocr"))]
#[path = "khmer_layer_tests.rs"]
mod tests;
