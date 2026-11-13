//! Positive raster evidence for word-shaped OCR noise on repeating backgrounds.

use super::{
    is_prunable_element, is_symbol_noise_line, page_is_dominated_by_noise_lines, prune_document_elements, prune_text,
};
use crate::pdf::structure::types::PdfParagraph;
use crate::types::internal::InternalDocument;
use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElement, OcrElementLevel};

const MAX_LINE_BOXES: usize = 1024;
const MAX_CANDIDATES: usize = 128;
const MAX_RASTER_PIXELS: u64 = 12_000_000;
const MIN_PEERS: usize = 4;
const SAMPLE_WIDTH: usize = 32;
const SAMPLE_HEIGHT: usize = 16;
const MAX_MEAN_DIFFERENCE: u32 = 25;
const MIN_CONTRAST: u8 = 48;

#[derive(Clone, Copy)]
struct BoxInRaster {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl BoxInRaster {
    fn from_line(line: &OcrElement, processed: (u32, u32), render: (u32, u32)) -> Option<Self> {
        let OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height,
        } = line.geometry
        else {
            return None;
        };
        let (pw, ph) = processed;
        let (rw, rh) = render;
        let right = left.checked_add(width)?;
        let bottom = top.checked_add(height)?;
        if right > pw || bottom > ph || width < height.saturating_mul(5) || !(5..=24).contains(&height) {
            return None;
        }
        let scale = |value, from, to| (u64::from(value) * u64::from(to) / u64::from(from)) as u32;
        let x = scale(left, pw, rw);
        let y = scale(top, ph, rh);
        Some(Self {
            x,
            y,
            width: scale(right, pw, rw).checked_sub(x)?,
            height: scale(bottom, ph, rh).checked_sub(y)?,
        })
    }

    fn peer(self, other: Self) -> bool {
        self.width.abs_diff(other.width) <= self.width / 6
            && self.height.abs_diff(other.height) <= 2
            && self.y.abs_diff(other.y) <= self.height
            && self.x.abs_diff(other.x) >= self.width.max(other.width)
            && self.x.abs_diff(other.x) <= 10 * self.width
    }
}

fn candidate(line: &OcrElement) -> bool {
    // Restrict deletion to the low-confidence curved-letter artifacts this evidence covers.
    let word = line.text.trim();
    line.level == OcrElementLevel::Line
        && (0.0..=0.50).contains(&line.confidence.recognition)
        && (4..=12).contains(&word.len())
        && word.bytes().all(|byte| byte.is_ascii_lowercase())
        && word.bytes().filter(|byte| matches!(byte, b'c' | b'e' | b'o')).count() * 4 >= word.len() * 3
}

fn sample(image: &image::RgbImage, rect: BoxInRaster, dx: i32, dy: i32) -> Option<[u8; SAMPLE_WIDTH * SAMPLE_HEIGHT]> {
    let x = (rect.x.checked_add(rect.width / 2)?).checked_add_signed(dx)?;
    let y = rect.y.checked_add_signed(dy)?;
    let left = x.checked_sub(rect.width * 3 / 5)?;
    let top = y.checked_sub(rect.height * 2)?;
    let width = rect.width * 6 / 5;
    let height = rect.height * 5;
    if width < SAMPLE_WIDTH as u32
        || height < SAMPLE_HEIGHT as u32
        || left.checked_add(width)? > image.width()
        || top.checked_add(height)? > image.height()
    {
        return None;
    }
    Some(std::array::from_fn(|i| {
        let x = left + (i % SAMPLE_WIDTH) as u32 * width / SAMPLE_WIDTH as u32;
        let y = top + (i / SAMPLE_WIDTH) as u32 * height / SAMPLE_HEIGHT as u32;
        let [r, g, b] = image.get_pixel(x, y).0;
        ((u16::from(r) + u16::from(g) + u16::from(b)) / 3) as u8
    }))
}

fn matching_peers(image: &image::RgbImage, target: BoxInRaster, peers: impl Iterator<Item = BoxInRaster>) -> bool {
    let Some(sampled) = sample(image, target, 0, 0) else {
        return false;
    };
    let Some((&low, &high)) = sampled.iter().min().zip(sampled.iter().max()) else {
        return false;
    };
    if high - low < MIN_CONTRAST {
        return false;
    }
    peers
        .filter(|peer| {
            (-1..=1).any(|dy| {
                (-1..=1).any(|dx| {
                    sample(image, *peer, dx, dy).is_some_and(|pixels| {
                        sampled
                            .iter()
                            .zip(pixels)
                            .map(|(&a, b)| u32::from(a.abs_diff(b)))
                            .sum::<u32>()
                            <= MAX_MEAN_DIFFERENCE * sampled.len() as u32
                    })
                })
            })
        })
        .take(MIN_PEERS)
        .count()
        == MIN_PEERS
}

pub(in crate::extractors::pdf::ocr) fn capture(
    raster: &super::super::rendering::EncodedPage,
    read: &ExtractedDocument,
    config: &crate::core::config::ExtractionConfig,
    remaining_work: &mut usize,
) -> Option<Vec<String>> {
    let render = (raster.2, raster.3);
    let processed = super::super::document::processed_ocr_layout_dimensions(&read.metadata)?;
    let scale_x = f64::from(render.0) / f64::from(processed.0);
    let scale_y = f64::from(render.1) / f64::from(processed.1);
    if (scale_x - scale_y).abs()
        > scale_x.max(scale_y) * crate::extractors::image::MAX_OCR_COORDINATE_SCALE_RELATIVE_DIFFERENCE
    {
        return None;
    }
    let px = u64::from(render.0) * u64::from(render.1);
    let limits = config.security_limits.clone().unwrap_or_default();
    if px == 0
        || px > MAX_RASTER_PIXELS
        || px * 3 > super::super::hidden_layer::preview_byte_limit(config) as u64
        || config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled())
        || !page_is_dominated_by_noise_lines(read.content.lines())
    {
        return None;
    }
    let elements = read.ocr_elements.as_deref()?;
    let lines = elements.iter().filter(|line| line.level == OcrElementLevel::Line);
    if elements.len() > MAX_LINE_BOXES * 8 || lines.clone().take(MAX_LINE_BOXES + 1).count() > MAX_LINE_BOXES {
        return None;
    }
    let boxes = lines
        .clone()
        .filter(|line| candidate(line))
        .filter_map(|line| Some((line.text.trim(), BoxInRaster::from_line(line, processed, render)?)))
        .take(MAX_CANDIDATES + 1)
        .collect::<Vec<_>>();
    if !(MIN_PEERS + 1..=MAX_CANDIDATES).contains(&boxes.len()) {
        return None;
    }
    // Charge the worst-case pair and registration search before decoding.
    let comparisons = boxes.len().pow(2) * 9 * SAMPLE_WIDTH * SAMPLE_HEIGHT;
    *remaining_work = remaining_work.checked_sub(px as usize * 3 + comparisons)?;
    let image = crate::extraction::image::load_image_for_ocr(&raster.1, &limits)
        .ok()?
        .into_rgb8();
    if image.dimensions() != render {
        return None;
    }
    let result = find_repeated_lines(&image, &boxes, lines);
    (!result.is_empty()).then_some(result)
}

fn find_repeated_lines<'a>(
    image: &image::RgbImage,
    boxes: &[(&str, BoxInRaster)],
    lines: impl Iterator<Item = &'a OcrElement> + Clone,
) -> Vec<String> {
    boxes
        .iter()
        .filter(|&&(text, target)| {
            lines.clone().filter(|line| line.text.trim() == text).take(2).count() == 1
                && matching_peers(
                    image,
                    target,
                    boxes
                        .iter()
                        .filter_map(|&(other, peer)| (text != other && target.peer(peer)).then_some(peer)),
                )
        })
        .map(|&(text, _)| text.to_owned())
        .collect()
}

pub(in crate::extractors::pdf::ocr) fn prune(
    text: &mut String,
    read: Option<&mut String>,
    paragraphs: Option<&mut Vec<PdfParagraph>>,
    document: Option<&mut InternalDocument>,
    candidates: &[String],
) -> (usize, usize) {
    let owned = |candidate: &str| {
        unique(text.split_whitespace(), candidate)
            && read
                .as_ref()
                .is_some_and(|read| unique(read.split_whitespace(), candidate))
            && paragraphs
                .as_ref()
                .is_none_or(|paragraphs| unique_paragraph_token(paragraphs, candidate))
            && document
                .as_ref()
                .is_none_or(|document| document_owns_token(document, candidate))
    };
    // Reconcile against every text surface before changing any of them.
    let accepted = candidates
        .iter()
        .filter(|text| owned(text))
        .map(String::as_str)
        .collect::<Vec<_>>();
    let (pruned, removed) = prune_text(text);
    let (pruned, texture_removed) = strip_tokens(&pruned, &accepted);
    *text = pruned;
    if let Some(read) = read {
        *read = strip_tokens(&prune_text(read).0, &accepted).0;
    }
    if let Some(paragraphs) = paragraphs {
        super::prune_paragraphs(paragraphs);
        strip_paragraph_tokens(paragraphs, &accepted);
    }
    if let Some(document) = document {
        prune_document_elements(document);
        document.elements.retain_mut(|element| {
            if !is_prunable_element(element) || !element.annotations.is_empty() {
                return true;
            }
            let (text, removed) = strip_tokens(&element.text, &accepted);
            element.text = text;
            removed == 0 || !element.text.trim().is_empty()
        });
    }
    (removed, texture_removed)
}

fn strip_tokens(text: &str, accepted: &[&str]) -> (String, usize) {
    let mut kept = String::with_capacity(text.len());
    let mut removed = 0;
    for line in text.split_inclusive('\n') {
        let line_start = kept.len();
        let mut previous_end = 0;
        let mut copy_from = 0;
        let mut has_survivor = false;
        for token in line.split_whitespace() {
            let offset = token.as_ptr() as usize - line.as_ptr() as usize;
            let end = offset + token.len();
            let remove = accepted.contains(&token);
            if remove {
                kept.push_str(&line[copy_from..previous_end]);
                copy_from = end;
                removed += 1;
            } else if !has_survivor && copy_from != 0 {
                copy_from = offset;
            }
            has_survivor |= !remove;
            previous_end = end;
        }
        if !has_survivor && copy_from != 0 {
            kept.truncate(line_start);
        } else {
            kept.push_str(&line[copy_from..]);
        }
    }
    if removed != 0 && !text.ends_with('\n') && kept.ends_with('\n') {
        kept.pop();
    }
    (kept, removed)
}

fn unique<'a>(tokens: impl Iterator<Item = &'a str>, candidate: &str) -> bool {
    tokens.filter(|&token| token == candidate).take(2).count() == 1
}

fn unique_paragraph_token(paragraphs: &[PdfParagraph], candidate: &str) -> bool {
    let mut owners = paragraphs.iter().filter(|paragraph| {
        paragraph.text.split_whitespace().any(|token| token == candidate)
            || paragraph_segments(paragraph).any(|token| token == candidate)
    });
    owners.next().is_some_and(|paragraph| {
        !paragraph.is_formula
            && !paragraph.is_code_block
            && unique(paragraph.text.split_whitespace(), candidate)
            && (paragraph.lines.is_empty() || unique(paragraph_segments(paragraph), candidate))
    }) && owners.next().is_none()
}

fn paragraph_segments(paragraph: &PdfParagraph) -> impl Iterator<Item = &str> {
    paragraph
        .lines
        .iter()
        .flat_map(|line| &line.segments)
        .flat_map(|segment| segment.text.split_whitespace())
}

fn document_owns_token(document: &InternalDocument, candidate: &str) -> bool {
    let mut owners = document
        .elements
        .iter()
        .filter(|element| element.text.split_whitespace().any(|token| token == candidate));
    owners.next().is_some_and(|element| {
        is_prunable_element(element)
            && element.annotations.is_empty()
            && unique(element.text.split_whitespace(), candidate)
    }) && owners.next().is_none()
}

fn strip_paragraph_tokens(paragraphs: &mut Vec<PdfParagraph>, accepted: &[&str]) {
    paragraphs.retain_mut(|paragraph| {
        if paragraph.text.is_empty() {
            return true;
        }
        let (text, removed) = strip_tokens(&paragraph.text, accepted);
        if removed == 0 {
            return true;
        }
        for line in &mut paragraph.lines {
            line.segments.retain_mut(|segment| {
                let (text, removed) = strip_tokens(&segment.text, accepted);
                segment.text = text;
                removed == 0 || !segment.text.trim().is_empty()
            });
        }
        paragraph.lines.retain(|line| !line.segments.is_empty());
        paragraph.word_count = text.split_whitespace().count();
        paragraph.text = text;
        !paragraph.text.trim().is_empty()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAINT_WORD: &str = concat!("Sa", "de");

    #[test]
    fn repeated_regions_require_four_peers_and_contrast() {
        let rect = |x| BoxInRaster {
            x,
            y: 45,
            width: 72,
            height: 8,
        };
        let peers = [rect(110), rect(200), rect(290), rect(380)];
        let target = rect(20);
        let blank = image::RgbImage::from_pixel(480, 100, image::Rgb([220; 3]));
        let texture = image::RgbImage::from_fn(480, 100, |x, y| {
            let tone = if (x % 90 / 5 + y / 3) % 2 == 0 { 40 } else { 220 };
            image::Rgb([tone; 3])
        });
        assert!(matching_peers(&texture, target, peers.into_iter()));
        assert!(!matching_peers(&texture, target, peers.into_iter().take(3)));
        assert!(!matching_peers(&blank, target, peers.into_iter()));
        let mut different = texture;
        for y in 30..68 {
            for x in 20..91 {
                different.put_pixel(x, y, image::Rgb([128; 3]));
            }
        }
        assert!(!matching_peers(&different, target, peers.into_iter()));
    }

    #[test]
    fn unique_noise_token_is_removed_from_a_joined_read() {
        let mut output = format!("{FAINT_WORD} ccccee Nina");
        let mut read = format!("{FAINT_WORD}\nccccee\nNina");
        assert_eq!(
            prune(&mut output, Some(&mut read), None, None, &["ccccee".to_owned()]),
            (0, 1)
        );
        assert_eq!(output, format!("{FAINT_WORD} Nina"));
        assert_eq!(read, format!("{FAINT_WORD}\nNina"));
    }

    #[test]
    fn duplicate_or_split_output_owner_keeps_the_token_everywhere() {
        for input in [
            format!("{FAINT_WORD} ccccee ccccee Nina"),
            format!("{FAINT_WORD} ccc cee Nina"),
        ] {
            let mut output = input.to_owned();
            let mut read = format!("{FAINT_WORD}\nccccee\nNina");
            assert_eq!(
                prune(&mut output, Some(&mut read), None, None, &["ccccee".to_owned()]),
                (0, 0)
            );
            assert_eq!(output, input);
            assert_eq!(read, format!("{FAINT_WORD}\nccccee\nNina"));
        }
    }
}

pub(in crate::extractors::pdf::ocr) fn warning(page: u32, removed: usize) -> crate::types::ProcessingWarning {
    crate::types::ProcessingWarning {
        source: "ocr".into(),
        message: format!(
            "Page {page}: removed {removed} OCR token(s) from repeated background texture; \
             set prune_symbol_noise_lines to false to keep them."
        )
        .into(),
    }
}
