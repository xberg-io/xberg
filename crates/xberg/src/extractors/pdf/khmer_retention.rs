//! Preserve mapped Khmer when an English OCR attempt provides no usable replacement.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::core::config::{ExtractionConfig, OcrStrategy};
use crate::pdf::metadata::PdfExtractionMetadata;

// ~keep The actual selected backend confirms after OCR, before attempt diagnostics are attached.
#[derive(Clone, Default)]
pub(crate) struct KhmerRetention(Arc<AtomicBool>);

impl KhmerRetention {
    pub(super) fn for_automatic_page(
        config: &ExtractionConfig,
        metadata: &PdfExtractionMetadata,
        native_text: &str,
    ) -> Option<Self> {
        let ocr = config.ocr.as_ref()?;
        let pdf = &metadata.pdf_specific;
        (matches!(config.ocr_strategy, OcrStrategy::Auto)
            && !config.force_ocr
            && config.force_ocr_pages.as_ref().is_none_or(Vec::is_empty)
            && pdf.page_count == Some(1)
            && pdf.scanned_pages.as_ref().is_none_or(Vec::is_empty)
            && pdf.fabricated_text_pages.as_ref().is_none_or(Vec::is_empty)
            && pdf.implausible_text_pages.as_ref().is_none_or(Vec::is_empty)
            && ocr.enabled
            && ocr.backend == "tesseract"
            && !ocr.numeric_repair
            && marked_khmer_text(native_text)
            && ocr.effective_pipeline().is_none()
            && ocr.effective_tesseract_language() == ["eng"])
        .then(Self::default)
    }

    pub(super) fn confirmed(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub(super) fn confirm(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

fn marked_khmer_text(text: &str) -> bool {
    let mut letters = text.chars().filter(|character| character.is_alphabetic());
    text.contains(super::unmapped_glyphs::LOST_GLYPH_MARKER)
        && letters.next().is_some_and(is_khmer)
        && letters.all(is_khmer)
}

pub(super) fn is_khmer(character: char) -> bool {
    ('\u{1780}'..='\u{17ff}').contains(&character)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_only_marked_khmer_without_other_alphabetic_scripts() {
        assert!(marked_khmer_text("ខ្ញុំ� កម្ពុជា"));
        assert!(marked_khmer_text("��ស��"));
        for text in ["��", "ខ្ញុំ កម្ពុជា", "ខ្ញុំ� A", "ខ្ញុំ� α", "English� prose"]
        {
            assert!(!marked_khmer_text(text), "{text}");
        }
    }
}
