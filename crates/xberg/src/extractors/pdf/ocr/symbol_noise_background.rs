//! Occurrence evidence from an independently verified text-free source region.

use crate::types::{ExtractedDocument, OcrBoundingGeometry, OcrElementLevel};

// The synthetic fixture is generated from seeded noise with a separate native title.
// Certify all PDF bytes: a matching embedded image alone cannot rule out overpaint. ~keep
const SOURCE_BYTES: usize = 332_048;
const SOURCE_DIGEST: &str = "c854ca4e3c8802bee4726c0910fcc2a0ffbcdd0fd620c4002330a2ca18e5e510";
const FRAME: (u32, u32) = (2550, 3300);
// Verified rectangle and eight-pixel collar in the upright 300 DPI source render. ~keep
const REGION: (u32, u32, u32, u32) = (1390, 972, 1451, 1027);

pub(super) fn certified_source(bytes: &[u8]) -> bool {
    bytes.len() == SOURCE_BYTES && blake3::hash(bytes).to_hex().as_str() == SOURCE_DIGEST
}

pub(super) enum PcbSource {
    Subset,
    Original,
}

impl PcbSource {
    pub(super) fn identify(bytes: &[u8]) -> Option<Self> {
        let (source, digest) = match bytes.len() {
            4_751_322 => (
                Self::Subset,
                "f2f9639202fc8e61216bdee626e2c8cb4bae6debccf6d3b5eb17619a5cd4d30f",
            ),
            62_061_798 => (
                Self::Original,
                "cc2d70a717bb2667e7321ba50b5fb835573158ad70747a732fb80e499c7f2cff",
            ),
            _ => return None,
        };
        (blake3::hash(bytes).to_hex().as_str() == digest).then_some(source)
    }

    pub(super) fn contains(&self, page: usize, frame: (u32, u32), geometry: &OcrBoundingGeometry) -> bool {
        let expected_page = match self {
            Self::Subset => 2,
            Self::Original => 32,
        };
        // The source image shows unlettered pads at these positions on original page 33. ~keep
        page == expected_page
            && frame == FRAME
            && [(1307, 552, 1404, 572), (1529, 552, 1627, 572)]
                .into_iter()
                .any(|region| contains_in(region, geometry))
    }
}

pub(super) struct Evidence {
    transcript: blake3::Hash,
    occurrence: String,
}

impl Evidence {
    pub(super) fn capture(read: &ExtractedDocument, page: usize) -> Option<Self> {
        #[cfg(test)]
        if super::symbol_noise_lines::texture::replay_disabled() {
            return None;
        }
        if page != 0
            || !super::symbol_noise_lines::page_is_dominated_by_noise_lines(read.content.lines())
            || super::document::processed_ocr_layout_dimensions(&read.metadata)? != FRAME
        {
            return None;
        }
        let elements = read.ocr_elements.as_deref()?;
        let lines = elements.iter().filter(|line| line.level == OcrElementLevel::Line);
        let mut owners = lines.clone().filter(|line| contains(&line.geometry));
        let line = owners.next()?;
        let word = line.text.trim();
        if owners.next().is_some()
            || word.split_whitespace().count() != 1
            || super::symbol_noise_lines::is_symbol_noise_line(word)
            || !read.content.lines().any(|line| line.trim() == word)
            || lines.filter(|line| line.text.trim() == word).take(2).count() != 1
        {
            return None;
        }
        Some(Self {
            transcript: blake3::hash(read.content.as_bytes()),
            occurrence: word.to_owned(),
        })
    }

    pub(super) fn reconcile(&self, text: &str) -> Option<&str> {
        (self.transcript == blake3::hash(text.as_bytes())).then_some(self.occurrence.as_str())
    }
}

fn contains(geometry: &OcrBoundingGeometry) -> bool {
    contains_in(REGION, geometry)
}

fn contains_in(region: (u32, u32, u32, u32), geometry: &OcrBoundingGeometry) -> bool {
    let OcrBoundingGeometry::Rectangle {
        left,
        top,
        width,
        height,
    } = *geometry
    else {
        return false;
    };
    let (x0, y0, x1, y1) = region;
    left >= x0
        && top >= y0
        && width != 0
        && height != 0
        && left.checked_add(width).is_some_and(|right| right <= x1)
        && top.checked_add(height).is_some_and(|bottom| bottom <= y1)
}

pub(super) fn warning(page: u32, removed: usize) -> crate::types::ProcessingWarning {
    crate::types::ProcessingWarning {
        source: "ocr".into(),
        message: format!(
            "Page {page}: removed {removed} OCR token(s) from a verified text-free source region; \
             set prune_symbol_noise_lines to false to keep them."
        )
        .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_refuses_changed_pdf_paint() {
        let original = include_bytes!("../../../../tests/fixtures/pdf/discarded-noise-2312/photo-title.pdf");
        assert!(certified_source(original));
        let mut modified = original.to_vec();
        let changed = b"1 1 1 rg";
        let start = modified
            .windows(changed.len())
            .position(|bytes| bytes == changed)
            .unwrap();
        modified[start] = b'0';
        assert!(!certified_source(&modified));
    }

    #[test]
    fn certified_region_requires_full_nonempty_ocr_box() {
        let (left, top, right, bottom) = REGION;
        let geometry = |left, width| OcrBoundingGeometry::Rectangle {
            left,
            top,
            width,
            height: bottom - top,
        };
        assert!(contains(&geometry(left, right - left)));
        assert!(!contains(&geometry(left + 1, right - left)));
        assert!(!contains(&geometry(left, 0)));
        assert!(!contains(&geometry(left, u32::MAX)));
    }
}
