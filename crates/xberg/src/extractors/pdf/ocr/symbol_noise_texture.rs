//! Positive raster evidence for word-shaped OCR noise on repeating backgrounds.

use super::{is_prunable_element, page_is_dominated_by_noise_lines, prune_document_elements, prune_text};
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
const TEXT_BAND_START: usize = 6;
const TEXT_BAND_END: usize = 10;
const COLLAR_SAMPLES: u32 = (SAMPLE_WIDTH * (SAMPLE_HEIGHT - (TEXT_BAND_END - TEXT_BAND_START))) as u32;
const MIN_CONTRAST: u8 = 48;
const MAX_SCALE_MISMATCH: f64 = 0.01;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

type Sample = [u8; SAMPLE_WIDTH * SAMPLE_HEIGHT];

impl BoxInRaster {
    fn window(self, dx: i32, dy: i32) -> Option<Self> {
        Some(Self {
            x: self
                .x
                .checked_add(self.width / 2)?
                .checked_add_signed(dx)?
                .checked_sub(self.width * 3 / 5)?,
            y: self.y.checked_add_signed(dy)?.checked_sub(self.height * 2)?,
            width: self.width * 6 / 5,
            height: self.height * 5,
        })
    }

    fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.width
            && other.x < self.x + self.width
            && self.y < other.y + other.height
            && other.y < self.y + self.height
    }
}

fn sample(image: &image::RgbImage, window: BoxInRaster) -> Option<Sample> {
    if window.width < SAMPLE_WIDTH as u32
        || window.height < SAMPLE_HEIGHT as u32
        || window.x.checked_add(window.width)? > image.width()
        || window.y.checked_add(window.height)? > image.height()
    {
        return None;
    }
    Some(std::array::from_fn(|i| {
        let x = window.x + (i % SAMPLE_WIDTH) as u32 * window.width / SAMPLE_WIDTH as u32;
        let y = window.y + (i / SAMPLE_WIDTH) as u32 * window.height / SAMPLE_HEIGHT as u32;
        let [r, g, b] = image.get_pixel(x, y).0;
        ((u16::from(r) + u16::from(g) + u16::from(b)) / 3) as u8
    }))
}

fn sample_difference(target: &Sample, peer: &Sample) -> u32 {
    target.iter().zip(peer).map(|(&a, &b)| u32::from(a.abs_diff(b))).sum()
}

fn collar_difference(target: &Sample, peer: &Sample) -> u32 {
    target
        .iter()
        .zip(peer)
        .enumerate()
        .filter(|(i, _)| !(TEXT_BAND_START..TEXT_BAND_END).contains(&(i / SAMPLE_WIDTH)))
        .map(|(_, (&a, &b))| u32::from(a.abs_diff(b)))
        .sum()
}

fn best_peer(
    image: &image::RgbImage,
    peer: BoxInRaster,
    dimensions: BoxInRaster,
    target: &Sample,
) -> Option<(BoxInRaster, Sample)> {
    let peer = BoxInRaster {
        width: dimensions.width,
        height: dimensions.height,
        ..peer
    };
    (-1..=1)
        .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
        .filter_map(|(dx, dy)| {
            let window = peer.window(dx, dy)?;
            let pixels = sample(image, window)?;
            Some((window, pixels))
        })
        .min_by_key(|(_, pixels)| collar_difference(target, pixels))
        .filter(|(_, pixels)| {
            collar_difference(target, pixels) <= MAX_MEAN_DIFFERENCE * COLLAR_SAMPLES
                && sample_difference(target, pixels) <= MAX_MEAN_DIFFERENCE * target.len() as u32
        })
}

fn matching_peers(image: &image::RgbImage, target: BoxInRaster, peers: impl Iterator<Item = BoxInRaster>) -> bool {
    let Some(window) = target.window(0, 0) else {
        return false;
    };
    let Some(sampled) = sample(image, window) else {
        return false;
    };
    let Some((&low, &high)) = sampled.iter().min().zip(sampled.iter().max()) else {
        return false;
    };
    if high - low < MIN_CONTRAST {
        return false;
    }
    let mut distinct = Vec::<(BoxInRaster, Sample)>::with_capacity(MIN_PEERS);
    for peer in peers {
        if let Some((donor, pixels)) = best_peer(image, peer, target, &sampled)
            && !donor.overlaps(window)
            && distinct.iter().all(|(previous, _)| !donor.overlaps(*previous))
        {
            distinct.push((donor, pixels));
            if distinct.len() == MIN_PEERS {
                return !has_local_residual(&sampled, &distinct);
            }
        }
    }
    false
}

fn has_local_residual(target: &Sample, donors: &[(BoxInRaster, Sample)]) -> bool {
    // A localized mark can survive a low whole-window average. Test the OCR box interior
    // against the full donor range so small registration differences do not become ink.
    let mut outside = [false; SAMPLE_WIDTH * SAMPLE_HEIGHT];
    for y in TEXT_BAND_START..TEXT_BAND_END {
        for x in 3..SAMPLE_WIDTH - 3 {
            let i = y * SAMPLE_WIDTH + x;
            outside[i] = donors
                .iter()
                .all(|(_, pixels)| target[i] > pixels[i].saturating_add(20))
                || donors
                    .iter()
                    .all(|(_, pixels)| pixels[i] > target[i].saturating_add(20));
            if outside[i] && outside[i - 1] && outside[i - SAMPLE_WIDTH] && outside[i - SAMPLE_WIDTH - 1] {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
tokio::task_local! {
    static REPLAY: Option<ReplayTokens>;
}

#[cfg(test)]
type ReplayTokens = std::sync::Arc<parking_lot::Mutex<Vec<String>>>;

#[cfg(test)]
#[path = "symbol_noise_texture_replay.rs"]
mod public_replay;

pub(in crate::extractors::pdf::ocr) struct Evidence {
    transcript: blake3::Hash,
    occurrences: Vec<Occurrence>,
}

struct Occurrence {
    text: String,
    geometry: OcrBoundingGeometry,
}

fn decoder_limits(
    config: &crate::core::config::ExtractionConfig,
    encoded: usize,
    pixels: usize,
) -> Option<crate::extractors::security::SecurityLimits> {
    let mut limits = config.security_limits.clone().unwrap_or_default();
    let cap = super::super::hidden_layer::preview_byte_limit(config);
    if encoded.checked_add(pixels.checked_mul(3)?)? > cap {
        return None;
    }
    // The standard decoder accounts for encoded bytes in its own peak estimate.
    limits.max_content_size = cap;
    Some(limits)
}

pub(in crate::extractors::pdf::ocr) fn capture(
    raster: &super::super::rendering::EncodedPage,
    read: &ExtractedDocument,
    config: &crate::core::config::ExtractionConfig,
    remaining_work: &mut usize,
) -> Option<Evidence> {
    #[cfg(test)]
    if REPLAY.try_with(Option::is_none).unwrap_or(false) {
        return None;
    }
    let render = (raster.2, raster.3);
    let processed = super::super::document::processed_ocr_layout_dimensions(&read.metadata)?;
    let scale_x = f64::from(render.0) / f64::from(processed.0);
    let scale_y = f64::from(render.1) / f64::from(processed.1);
    if (scale_x - scale_y).abs() > scale_x.max(scale_y) * MAX_SCALE_MISMATCH {
        return None;
    }
    let px = u64::from(render.0) * u64::from(render.1);
    if px == 0
        || px > MAX_RASTER_PIXELS
        || config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled())
        || !page_is_dominated_by_noise_lines(read.content.lines())
    {
        return None;
    }
    let limits = decoder_limits(config, raster.1.len(), px as usize)?;
    let elements = read.ocr_elements.as_deref()?;
    let lines = elements.iter().filter(|line| line.level == OcrElementLevel::Line);
    if elements.len() > MAX_LINE_BOXES * 8 || lines.clone().take(MAX_LINE_BOXES + 1).count() > MAX_LINE_BOXES {
        return None;
    }
    let boxes = lines
        .clone()
        .filter(|line| candidate(line))
        .filter_map(|line| Some((line, BoxInRaster::from_line(line, processed, render)?)))
        .take(MAX_CANDIDATES + 1)
        .collect::<Vec<_>>();
    if !(MIN_PEERS + 1..=MAX_CANDIDATES).contains(&boxes.len()) {
        return None;
    }
    // Charge the worst-case pair and registration search before decoding.
    let comparisons = boxes.len().pow(2) * 9 * SAMPLE_WIDTH * SAMPLE_HEIGHT;
    *remaining_work = remaining_work.checked_sub(px as usize * 3 + comparisons)?;
    let image = crate::extraction::image_decode::decode_standard_rgb8_with_security_limits(&raster.1, &limits).ok()?;
    if image.dimensions() != render {
        return None;
    }
    let margins = crate::pdf::native::text::PageMarginFractions::from_extraction_config(Some(config));
    let occurrences = find_repeated_lines(&image, &boxes, lines)
        .into_iter()
        .filter(|source| source.inside_margins(processed.1, margins))
        .collect::<Vec<_>>();
    (!occurrences.is_empty()).then(|| Evidence {
        transcript: blake3::hash(read.content.as_bytes()),
        occurrences,
    })
}

fn find_repeated_lines<'a>(
    image: &image::RgbImage,
    boxes: &[(&OcrElement, BoxInRaster)],
    lines: impl Iterator<Item = &'a OcrElement> + Clone,
) -> Vec<Occurrence> {
    boxes
        .iter()
        .filter(|&&(line, target)| {
            let text = line.text.trim();
            let unique = lines.clone().filter(|line| line.text.trim() == text).take(2).count() == 1;
            let matched = unique
                && matching_peers(
                    image,
                    target,
                    boxes
                        .iter()
                        .filter_map(|&(other, peer)| (text != other.text.trim() && target.peer(peer)).then_some(peer)),
                );
            #[cfg(test)]
            if REPLAY.try_with(Option::is_some).unwrap_or(false) && matches!(text, "ceeces" | "ccccee") {
                eprintln!("raster {text}: box={target:?} unique={unique} matched={matched}");
            }
            matched
        })
        .map(|&(line, _)| Occurrence {
            text: line.text.trim().to_owned(),
            geometry: line.geometry.clone(),
        })
        .collect()
}

impl Occurrence {
    fn inside_margins(&self, height: u32, margins: crate::pdf::native::text::PageMarginFractions) -> bool {
        let OcrBoundingGeometry::Rectangle {
            top,
            height: box_height,
            ..
        } = self.geometry
        else {
            return false;
        };
        let Some(baseline) = top
            .checked_add(box_height)
            .and_then(|bottom| height.checked_sub(bottom))
        else {
            return false;
        };
        crate::pdf::native::text::baseline_is_inside_page_margins(baseline as f32, 0.0, height as f32, margins)
    }
}

impl Evidence {
    pub(in crate::extractors::pdf::ocr) fn reconciled_tokens<'a>(&'a self, text: &str, read: &str) -> Vec<&'a str> {
        // Source geometry is checked before publication filters. If mixed assembly
        // changed the transcript, the original occurrence can no longer authorize removal.
        if text != read || blake3::hash(read.as_bytes()) != self.transcript {
            return Vec::new();
        }
        self.occurrences.iter().map(|source| source.text.as_str()).collect()
    }
}

pub(in crate::extractors::pdf::ocr) fn prune(
    text: &mut String,
    read: Option<&mut String>,
    paragraphs: Option<&mut Vec<PdfParagraph>>,
    document: Option<&mut InternalDocument>,
    candidates: &[&str],
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
        .copied()
        .filter(|text| owned(text))
        .collect::<Vec<_>>();
    let (pruned, removed) = prune_text(text);
    let (pruned, texture_removed) = strip_tokens(&pruned, &accepted);
    #[cfg(test)]
    let _ = REPLAY.try_with(|replay| {
        if let Some(tokens) = replay {
            tokens.lock().extend(accepted.iter().map(|token| (*token).to_owned()));
        }
    });
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
        assert!(!matching_peers(
            &texture,
            target,
            [rect(110), rect(111), rect(200), rect(201)].into_iter()
        ));
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
        assert_eq!(prune(&mut output, Some(&mut read), None, None, &["ccccee"]), (0, 1));
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
            assert_eq!(prune(&mut output, Some(&mut read), None, None, &["ccccee"]), (0, 0));
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

#[cfg(test)]
mod revision_tests {
    use super::*;

    #[test]
    fn similar_glyph_band_does_not_compensate_for_a_different_collar() {
        let target = [100; SAMPLE_WIDTH * SAMPLE_HEIGHT];
        let rect = BoxInRaster {
            x: 50,
            y: 45,
            width: 72,
            height: 8,
        };
        let window = rect.window(0, 0).unwrap();
        let image = image::RgbImage::from_fn(100, 100, |_, y| {
            let tone = if (window.y + 15..window.y + 25).contains(&y) {
                100
            } else {
                132
            };
            image::Rgb([tone; 3])
        });
        let peer = sample(&image, window).unwrap();
        assert!(sample_difference(&target, &peer) <= MAX_MEAN_DIFFERENCE * target.len() as u32);
        assert!(best_peer(&image, rect, rect, &target).is_none());
    }

    #[test]
    fn localized_faint_ink_vetoes_same_pad_texture_match() {
        let rect = |x| BoxInRaster {
            x,
            y: 45,
            width: 72,
            height: 8,
        };
        let mut texture = image::RgbImage::from_fn(480, 100, |x, _| {
            let tone = if x % 90 < 45 { 48 } else { 160 };
            image::Rgb([tone; 3])
        });
        let target = rect(20);
        let peers = [rect(110), rect(200), rect(290), rect(380)];
        assert!(matching_peers(&texture, target, peers.into_iter()));
        for y in 44..54 {
            for x in 32..43 {
                texture.put_pixel(x, y, image::Rgb([78; 3]));
            }
        }
        let reference = sample(&texture, rect(110).window(0, 0).unwrap()).unwrap();
        let marked = sample(&texture, target.window(0, 0).unwrap()).unwrap();
        assert!(sample_difference(&reference, &marked) <= MAX_MEAN_DIFFERENCE * marked.len() as u32);
        assert!(!matching_peers(&texture, target, peers.into_iter()));
    }

    #[test]
    fn surviving_margin_or_body_token_must_own_the_original_box_and_transcript() {
        let occurrence = Occurrence {
            text: "ccccee".into(),
            geometry: OcrBoundingGeometry::Rectangle {
                left: 50,
                top: 20,
                width: 72,
                height: 8,
            },
        };
        let margins = crate::pdf::native::text::PageMarginFractions { top: 0.1, bottom: 0.0 };
        assert!(!occurrence.inside_margins(400, margins));
        assert!(occurrence.inside_margins(400, Default::default()));
        let text = "ccccee genuine text";
        let evidence = Evidence {
            transcript: blake3::hash(text.as_bytes()),
            occurrences: vec![occurrence],
        };
        assert!(evidence.reconciled_tokens("genuine text ccccee", text).is_empty());
        assert!(evidence.reconciled_tokens("ccccee", "ccccee").is_empty());
        assert_eq!(evidence.reconciled_tokens(text, text), ["ccccee"]);
    }

    #[test]
    fn decoder_receives_minimum_cap_and_charges_the_encoded_payload() {
        use crate::core::config::{ExtractionConfig, OcrConfig};
        use crate::extractors::security::SecurityLimits;

        for (extraction, ocr) in [(100, 500), (500, 100)] {
            let config = ExtractionConfig {
                security_limits: Some(SecurityLimits {
                    max_content_size: extraction,
                    ..Default::default()
                }),
                ocr: Some(OcrConfig {
                    security_limits: Some(SecurityLimits {
                        max_content_size: ocr,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            };
            assert!(decoder_limits(&config, 41, 20).is_none());
            assert_eq!(decoder_limits(&config, 40, 20).unwrap().max_content_size, 100);
            assert!(decoder_limits(&config, usize::MAX, 20).is_none());
        }
    }

    #[test]
    fn real_png_decode_rejects_combined_budget_one_byte_short() {
        use image::{ExtendedColorType, ImageEncoder};

        let mut encoded = Vec::new();
        let rgb = [42; 12];
        image::codecs::png::PngEncoder::new(&mut encoded)
            .write_image(&rgb, 2, 2, ExtendedColorType::Rgb8)
            .unwrap();
        let cap = encoded.len() + rgb.len();
        let config = |max_content_size| crate::core::config::ExtractionConfig {
            ocr: Some(crate::core::config::OcrConfig {
                security_limits: Some(crate::extractors::security::SecurityLimits {
                    max_content_size,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(decoder_limits(&config(cap - 1), encoded.len(), 4).is_none());
        let limits = decoder_limits(&config(cap), encoded.len(), 4).unwrap();
        assert_eq!(
            crate::extraction::image_decode::decode_standard_rgb8_with_security_limits(&encoded, &limits)
                .unwrap()
                .as_raw(),
            &rgb
        );
    }

    #[test]
    fn removing_texture_conserves_every_other_token_and_separator() {
        let gap = "\u{2003}\u{2003}";
        assert_eq!(
            strip_tokens(
                &format!("ccccee café{gap}Nina ccccee\n\tother  prose\nccccee"),
                &["ccccee"]
            ),
            (format!("café{gap}Nina\n\tother  prose"), 3),
        );
    }
}
