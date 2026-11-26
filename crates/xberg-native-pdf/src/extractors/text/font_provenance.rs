use super::*;
use crate::fonts::MappingProvenance;
use std::sync::Mutex;

const MAX_FONT_PROVENANCE_ENTRIES: usize = 128;

#[derive(Debug, Default)]
pub(super) struct FontProvenanceCache {
    entries: Mutex<HashMap<usize, (Arc<FontInfo>, MappingProvenance)>>,
    name_traits: Mutex<HashMap<usize, (Arc<FontInfo>, FontNameTraits)>>,
    #[cfg(test)]
    computations: std::sync::atomic::AtomicUsize,
}

/// What a font's `/BaseFont` name alone says about its glyphs. Every show-text buffer asks, and
/// deriving it case-folds the name and searches it for a dozen markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FontNameTraits {
    /// The name contains `GLYPHLESS` in any case, the synthetic font of OCR text layers.
    pub(super) glyphless: bool,
    /// [`is_monospace_font`] holds for the name.
    pub(super) monospace: bool,
}

impl FontNameTraits {
    fn of(font: &FontInfo) -> Self {
        Self {
            glyphless: font.base_font.to_uppercase().contains("GLYPHLESS"),
            monospace: is_monospace_font(&font.base_font),
        }
    }
}

impl FontProvenanceCache {
    pub(super) fn resolve(&self, font: &Arc<FontInfo>) -> MappingProvenance {
        self.resolve_with(font, || font.best_mapping_provenance())
    }

    fn resolve_with(&self, font: &Arc<FontInfo>, compute: impl FnOnce() -> MappingProvenance) -> MappingProvenance {
        let key = Arc::as_ptr(font) as usize;
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, provenance)) = entries.get(&key) {
            return *provenance;
        }
        if entries.len() >= MAX_FONT_PROVENANCE_ENTRIES {
            entries.clear();
        }
        #[cfg(test)]
        self.computations.fetch_add(1, Ordering::Relaxed);
        let provenance = compute();
        // Retaining the Arc prevents address reuse; Arc::make_mut gets a fresh identity. ~keep
        entries.insert(key, (Arc::clone(font), provenance));
        provenance
    }

    /// [`FontNameTraits`] of `font`, derived once per font. Keyed and bounded like the
    /// provenance answers.
    pub(super) fn name_traits(&self, font: &Arc<FontInfo>) -> FontNameTraits {
        let key = Arc::as_ptr(font) as usize;
        let mut traits = self
            .name_traits
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, known)) = traits.get(&key) {
            return *known;
        }
        if traits.len() >= MAX_FONT_PROVENANCE_ENTRIES {
            traits.clear();
        }
        let derived = FontNameTraits::of(font);
        traits.insert(key, (Arc::clone(font), derived));
        derived
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn type3_font(glyph_name: &str) -> Arc<FontInfo> {
        let pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
            2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n\
            3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n\
            xref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n\
            0000000058 00000 n \n0000000115 00000 n \n\
            trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n197\n%%EOF\n";
        let doc = crate::document::PdfDocument::from_bytes(pdf.to_vec()).expect("minimal PDF");
        let dictionary = Object::Dictionary(HashMap::from([
            ("Subtype".into(), Object::Name("Type3".into())),
            ("BaseFont".into(), Object::Name("ProcedureFont".into())),
        ]));
        let mut font = FontInfo::from_dict(&dictionary, &doc).expect("Type3 font");
        font.encoding = crate::fonts::Encoding::Custom(HashMap::new());
        font.diff_glyph_names = HashMap::from([(2, glyph_name.to_string())]);
        Arc::new(font)
    }

    #[test]
    fn repeated_shared_font_queries_compute_provenance_once() {
        let cache = FontProvenanceCache::default();
        let font = type3_font("g02");
        let alias = Arc::clone(&font);
        let computations = Cell::new(0);
        for current in [&font, &alias].into_iter().cycle().take(1000) {
            assert_eq!(
                cache.resolve_with(current, || {
                    computations.set(computations.get() + 1);
                    current.best_mapping_provenance()
                }),
                MappingProvenance::Fallback
            );
        }
        assert_eq!(computations.get(), 1);
    }

    #[test]
    fn identical_font_names_do_not_share_mapping_answers() {
        let cache = FontProvenanceCache::default();
        let procedural = type3_font("g02");
        let named = type3_font("A");
        assert_eq!(procedural.base_font, named.base_font);
        assert_eq!(cache.resolve(&procedural), MappingProvenance::Fallback);
        assert_eq!(cache.resolve(&named), MappingProvenance::EncodingName);
    }

    #[test]
    fn copy_on_write_font_setup_gets_a_fresh_mapping_answer() {
        let cache = FontProvenanceCache::default();
        let mut font = type3_font("g02");
        assert_eq!(cache.resolve(&font), MappingProvenance::Fallback);
        Arc::make_mut(&mut font).diff_glyph_names.insert(2, "A".into());
        assert_eq!(cache.resolve(&font), MappingProvenance::EncodingName);
    }

    #[test]
    fn provenance_storage_is_bounded_and_releases_font_ownership() {
        let cache = FontProvenanceCache::default();
        let font = type3_font("g02");
        let weak = Arc::downgrade(&font);
        cache.resolve(&font);
        drop(font);
        assert!(weak.upgrade().is_some());
        for _ in 0..MAX_FONT_PROVENANCE_ENTRIES {
            cache.resolve(&type3_font("g02"));
        }
        assert!(cache.entries.lock().unwrap().len() <= MAX_FONT_PROVENANCE_ENTRIES);
        assert!(weak.upgrade().is_none());
    }

    fn named_font(base_font: &str) -> Arc<FontInfo> {
        let mut font = (*type3_font("A")).clone();
        font.base_font = base_font.to_string();
        Arc::new(font)
    }

    #[test]
    fn name_traits_match_the_uncached_name_checks_for_each_font() {
        let cache = FontProvenanceCache::default();
        let cases = [
            ("GlyphLessFont", true, false),
            ("glyphlessfont", true, false),
            ("ABCDEF+CMTT10", false, true),
            ("Courier-Bold", false, true),
            ("Monotype-Corsiva", false, false),
            ("Times-Roman", false, false),
        ];
        let fonts: Vec<_> = cases.iter().map(|(name, ..)| named_font(name)).collect();
        for _ in 0..3 {
            for (font, (name, glyphless, monospace)) in fonts.iter().zip(cases) {
                let traits = cache.name_traits(font);
                assert_eq!(traits, FontNameTraits { glyphless, monospace }, "{name}");
                assert_eq!(traits.monospace, is_monospace_font(name), "{name}");
            }
        }
        assert_eq!(cache.name_traits.lock().unwrap().len(), cases.len());
    }

    #[test]
    fn each_span_in_a_run_of_one_alias_gets_that_fonts_name_and_mapping() {
        let mut procedural = (*type3_font("g02")).clone();
        procedural.base_font = "Procedural".into();
        let mut named = (*type3_font("A")).clone();
        named.base_font = "Named".into();
        let mut extractor = TextExtractor::new();
        extractor.add_font_shared("F1".into(), Arc::new(procedural));
        extractor.add_font_shared("F2".into(), Arc::new(named));
        let content = b"BT /F1 12 Tf 10 100 Td (\\002) Tj 0 -20 Td (\\002) Tj \
            /F2 12 Tf 0 -20 Td (\\002) Tj /F1 12 Tf 0 -20 Td (\\002) Tj ET";
        let spans = extractor.extract_text_spans(content).expect("Type3 extraction");
        let resolved: Vec<(&str, Option<MappingProvenance>)> = spans
            .iter()
            .map(|span| (span.font_name.as_str(), span.provenance))
            .collect();
        let fallback = ("Procedural", Some(MappingProvenance::Fallback));
        let encoded = ("Named", Some(MappingProvenance::EncodingName));
        assert_eq!(resolved, [fallback, fallback, encoded, fallback]);
    }

    #[test]
    fn dropping_extraction_cache_releases_its_font() {
        let font = type3_font("g02");
        let weak = Arc::downgrade(&font);
        let cache = FontProvenanceCache::default();
        cache.resolve(&font);
        drop(font);
        drop(cache);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn repeated_type3_text_preserves_placeholders_and_font_mapping() {
        let mut extractor = TextExtractor::new();
        extractor.add_font_shared("F1".into(), type3_font("g02"));
        let content = b"BT /F1 12 Tf 10 100 Td (\\002\\002) Tj 0 -20 Td (\\002) Tj ET";
        let spans = extractor.extract_text_spans(content).expect("Type3 extraction");
        let text: String = spans.iter().map(|span| span.text.as_str()).collect();
        assert_eq!(text.chars().filter(|ch| *ch == '?').count(), 3);
        assert!(
            spans
                .iter()
                .all(|span| span.provenance == Some(MappingProvenance::Fallback))
        );
        assert_eq!(extractor.font_provenance_cache.entries.lock().unwrap().len(), 1);
        assert_eq!(extractor.font_provenance_cache.computations.load(Ordering::Relaxed), 1);
    }
}
