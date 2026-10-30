//! Glyphs without a Unicode value in native PDF text.
//!
//! The native extractor keeps one U+FFFD for each Type0 glyph that has no Unicode value, so the
//! OCR routing gates can count the loss. A page that OCR does not replace must not show the
//! marker to the caller. This module replaces each run of markers with one space between the
//! characters that survive, so that `A<marker>B` and `1<marker>2` do not fuse into `AB` and `12`.
//! It does this after the routing decision, keeps every byte offset that points into the text
//! correct, and builds the warning for a document that still lacks characters. ~keep

use crate::core::config::ExtractionConfig;
use crate::types::internal::InternalDocument;
use crate::types::{PageBoundary, PageContent, ProcessingWarning, Table};

/// The marker the native extractor leaves for a glyph without a Unicode value.
pub(super) const LOST_GLYPH_MARKER: char = '\u{FFFD}';

/// A run of consecutive markers in a text, and what replaces it. One lost glyph, or several in a
/// row, stands for characters that exist on the page, so the run leaves a single space between
/// the characters that survive. Next to whitespace, or at an edge of the text, it leaves nothing.
struct Run {
    start: usize,
    end: usize,
    replacement: &'static str,
}

/// The runs of markers in a text, in order. Maps an offset in the text with the markers to the same
/// place in the text without them.
struct MarkerRuns(Vec<Run>);

impl MarkerRuns {
    fn of(text: &str) -> Option<Self> {
        Self::outside(text, &[])
    }

    /// The runs made of markers that lie outside every protected byte range.
    fn outside(text: &str, protected: &[(usize, usize)]) -> Option<Self> {
        let width = LOST_GLYPH_MARKER.len_utf8();
        let mut runs: Vec<Run> = Vec::new();
        for (at, _) in text.match_indices(LOST_GLYPH_MARKER) {
            if protected.iter().any(|&(start, end)| (start..end).contains(&at)) {
                continue;
            }
            match runs.last_mut() {
                Some(run) if run.end == at => run.end = at + width,
                _ => runs.push(Run {
                    start: at,
                    end: at + width,
                    replacement: "",
                }),
            }
        }
        for run in &mut runs {
            let before = text[..run.start].chars().next_back();
            let after = text[run.end..].chars().next();
            if before.is_some_and(|c| !c.is_whitespace()) && after.is_some_and(|c| !c.is_whitespace()) {
                run.replacement = " ";
            }
        }
        (!runs.is_empty()).then_some(Self(runs))
    }

    fn count(&self) -> usize {
        self.0
            .iter()
            .map(|run| (run.end - run.start) / LOST_GLYPH_MARKER.len_utf8())
            .sum()
    }

    fn map(&self, offset: usize) -> usize {
        let mut removed = 0;
        for run in &self.0 {
            if offset >= run.end {
                removed += run.end - run.start - run.replacement.len();
            } else {
                if offset > run.start {
                    return run.start - removed;
                }
                break;
            }
        }
        offset - removed
    }

    fn apply(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut from = 0;
        for run in &self.0 {
            out.push_str(&text[from..run.start]);
            out.push_str(run.replacement);
            from = run.end;
        }
        out.push_str(&text[from..]);
        out
    }
}

fn without_markers(text: &str) -> String {
    MarkerRuns::of(text).map_or_else(|| text.to_string(), |runs| runs.apply(text))
}

/// Remove the markers that native extraction left in `text`. `boundaries` index `text` by byte and
/// move with it. A character U+FFFD inside a page in `ocr_pages` belongs to the OCR output, so it
/// stays. Returns the number of markers removed.
pub(super) fn strip_text(text: &mut String, boundaries: Option<&mut Vec<PageBoundary>>, ocr_pages: &[u32]) -> usize {
    let protected: Vec<(usize, usize)> = boundaries
        .as_deref()
        .into_iter()
        .flatten()
        .filter(|b| ocr_pages.contains(&b.page_number))
        .map(|b| (b.byte_start, b.byte_end))
        .collect();
    let Some(runs) = MarkerRuns::outside(text, &protected) else {
        return 0;
    };
    for boundary in boundaries.into_iter().flatten() {
        boundary.byte_start = runs.map(boundary.byte_start);
        boundary.byte_end = runs.map(boundary.byte_end);
    }
    *text = runs.apply(text);
    runs.count()
}

/// Remove every marker from the text of the elements of `doc`, with their annotation offsets.
pub(super) fn strip_document(doc: &mut InternalDocument) -> usize {
    let mut removed = 0;
    for element in &mut doc.elements {
        let Some(runs) = MarkerRuns::of(&element.text) else {
            continue;
        };
        for annotation in &mut element.annotations {
            annotation.start = runs.map(annotation.start as usize) as u32;
            annotation.end = runs.map(annotation.end as usize) as u32;
        }
        element.text = runs.apply(&element.text);
        removed += runs.count();
    }
    for table in &mut doc.tables {
        removed += strip_table(table);
    }
    removed
}

/// Remove every marker from a page: its text, its hierarchy blocks and its tables.
/// A page whose text becomes blank is marked blank.
pub(super) fn strip_page(page: &mut PageContent) -> usize {
    let mut removed = page.content.matches(LOST_GLYPH_MARKER).count();
    if removed > 0 {
        page.content = without_markers(&page.content);
        page.is_blank = Some(crate::extraction::blank_detection::is_page_text_blank(&page.content));
    }
    if let Some(hierarchy) = page.hierarchy.as_mut() {
        for block in &mut hierarchy.blocks {
            removed += block.text.matches(LOST_GLYPH_MARKER).count();
            block.text = without_markers(&block.text);
        }
    }
    for table in &mut page.tables {
        if table.markdown.contains(LOST_GLYPH_MARKER)
            || table.cells.iter().flatten().any(|c| c.contains(LOST_GLYPH_MARKER))
        {
            removed += strip_table(std::sync::Arc::make_mut(table));
        }
    }
    removed
}

/// Remove every marker from the cells and the Markdown of a table.
pub(super) fn strip_table(table: &mut Table) -> usize {
    let mut removed = table.markdown.matches(LOST_GLYPH_MARKER).count();
    table.markdown = without_markers(&table.markdown);
    for cell in table.cells.iter_mut().flatten() {
        removed += cell.matches(LOST_GLYPH_MARKER).count();
        *cell = without_markers(cell);
    }
    removed
}

/// The warning for a document that still lacks characters after extraction. It names the OCR
/// setting only for a caller who has not configured OCR and has not switched it off.
pub(super) fn lost_glyph_warning(lost: usize, config: &ExtractionConfig) -> Option<ProcessingWarning> {
    if lost == 0 {
        return None;
    }
    let message = if config.ocr.is_none() && !config.effective_disable_ocr() {
        "Some glyphs in this PDF have no Unicode value, so their characters are missing from the text. \
         Set `ocr` to read the affected pages with OCR."
    } else {
        "Some glyphs in this PDF have no Unicode value, and their characters are missing from the text \
         that OCR did not replace."
    };
    Some(ProcessingWarning {
        source: std::borrow::Cow::Borrowed("pdf"),
        message: std::borrow::Cow::Borrowed(message),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::OcrConfig;
    use crate::extractors::pdf::PdfExtractor;
    use crate::plugins::InternalDocumentExtractor;
    use crate::types::document_structure::{AnnotationKind, TextAnnotation};
    use crate::types::internal::{ElementKind, InternalElement};

    const MARK: &str = "\u{FFFD}";

    #[test]
    fn strip_text_removes_markers_and_moves_page_boundaries() {
        // "ab" + marker (bytes 0..5), then "cd" + marker + "ef" (bytes 5..12).
        let mut text = format!("ab{MARK}cd{MARK}ef");
        let mut boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 5,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 5,
                byte_end: 12,
                page_number: 2,
            },
        ];

        let removed = strip_text(&mut text, Some(&mut boundaries), &[]);

        assert_eq!(removed, 2);
        assert_eq!(text, "ab cd ef");
        assert_eq!(
            (boundaries[0].byte_start, boundaries[0].byte_end),
            (0, 3),
            "page 1 ends after its two letters and the space that stands for the lost glyph"
        );
        assert_eq!((boundaries[1].byte_start, boundaries[1].byte_end), (3, 8));
    }

    #[test]
    fn strip_text_leaves_a_boundary_where_a_glyph_was_lost() {
        let cases = [
            ("A\u{FFFD}B", "A B"),
            ("1\u{FFFD}2", "1 2"),
            ("A\u{FFFD}\u{FFFD}\u{FFFD}B", "A B"),
            ("A \u{FFFD}B", "A B"),
            ("A\u{FFFD} B", "A B"),
            ("\u{FFFD}A", "A"),
            ("A\u{FFFD}", "A"),
            ("A\u{FFFD}\nB", "A\nB"),
        ];
        for (before, after) in cases {
            let mut text = before.to_string();
            strip_text(&mut text, None, &[]);
            assert_eq!(text, after, "input {before:?}");
        }
    }

    #[test]
    fn strip_text_leaves_text_without_markers_untouched() {
        let mut text = "plain \u{e9} text".to_string();
        let mut boundaries = vec![PageBoundary {
            byte_start: 0,
            byte_end: text.len(),
            page_number: 1,
        }];
        let before: Vec<(usize, usize, u32)> = boundaries
            .iter()
            .map(|b| (b.byte_start, b.byte_end, b.page_number))
            .collect();

        assert_eq!(strip_text(&mut text, Some(&mut boundaries), &[]), 0);
        assert_eq!(text, "plain \u{e9} text");
        let after: Vec<(usize, usize, u32)> = boundaries
            .iter()
            .map(|b| (b.byte_start, b.byte_end, b.page_number))
            .collect();
        assert_eq!(after, before);
    }

    #[test]
    fn strip_document_moves_annotation_offsets_with_the_text() {
        let mut doc = InternalDocument::new("application/pdf");
        let mut element = InternalElement::text(ElementKind::Paragraph, format!("x{MARK}yz"), 0);
        // "yz" sits at bytes 4..6 while the marker is present.
        element.annotations = vec![TextAnnotation {
            start: 4,
            end: 6,
            kind: AnnotationKind::Bold,
        }];
        doc.elements.push(element);

        let removed = strip_document(&mut doc);

        assert_eq!(removed, 1);
        assert_eq!(doc.elements[0].text, "x yz");
        assert_eq!(
            (doc.elements[0].annotations[0].start, doc.elements[0].annotations[0].end),
            (2, 4)
        );
    }

    #[test]
    fn strip_page_marks_a_page_blank_when_only_markers_were_on_it() {
        let mut page = PageContent {
            page_number: 1,
            content: format!("{MARK}{MARK}\n{MARK}"),
            tables: Vec::new(),
            image_indices: Vec::new(),
            image_preprocessing: None,
            hierarchy: None,
            is_blank: Some(false),
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
            ocr_confidence: None,
            native_content: None,
        };

        assert_eq!(strip_page(&mut page), 3);
        assert_eq!(page.content, "\n");
        assert_eq!(page.is_blank, Some(true));
    }

    #[test]
    fn strip_table_cleans_cells_and_markdown() {
        let mut table = Table {
            cells: vec![vec![format!("a{MARK}"), "b".to_string()]],
            markdown: format!("| a{MARK} | b |"),
            page_number: 1,
            bounding_box: None,
            ..Table::default()
        };

        assert_eq!(strip_table(&mut table), 2);
        assert_eq!(table.cells[0][0], "a");
        assert_eq!(table.markdown, "| a | b |");
    }

    #[test]
    fn warning_stays_until_recovery_and_names_the_ocr_setting_only_when_it_helps() {
        let bare = ExtractionConfig::default();
        let with_block = ExtractionConfig {
            ocr: Some(OcrConfig::default()),
            ..ExtractionConfig::default()
        };
        let switched_off = ExtractionConfig {
            disable_ocr: true,
            ..ExtractionConfig::default()
        };

        let warning = lost_glyph_warning(3, &bare).expect("a caller without OCR gets the warning");
        assert_eq!(warning.source, "pdf");
        assert!(
            warning.message.contains("no Unicode value"),
            "message: {}",
            warning.message
        );
        assert!(
            warning.message.contains("`ocr`"),
            "message names the setting: {}",
            warning.message
        );
        assert!(lost_glyph_warning(0, &bare).is_none(), "no lost glyph, no warning");
        for config in [&with_block, &switched_off] {
            let warning = lost_glyph_warning(3, config).expect("lost glyphs that OCR did not recover keep a signal");
            assert!(
                warning.message.contains("no Unicode value"),
                "message: {}",
                warning.message
            );
            assert!(
                !warning.message.contains("`ocr`"),
                "no advice to set what is already set or off: {}",
                warning.message
            );
        }
    }

    #[test]
    fn strip_text_keeps_a_replacement_character_that_belongs_to_ocr_output() {
        // Page 1 is OCR output "A<mark>B" (bytes 0..5); page 2 is native "c<mark>d" (bytes 5..10).
        let mut text = format!("A{MARK}Bc{MARK}d");
        let mut boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 5,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 5,
                byte_end: 10,
                page_number: 2,
            },
        ];

        let removed = strip_text(&mut text, Some(&mut boundaries), &[1]);

        assert_eq!(removed, 1, "only the native marker counts as a loss");
        assert_eq!(text, format!("A{MARK}Bc d"));
        assert_eq!((boundaries[0].byte_start, boundaries[0].byte_end), (0, 5));
        assert_eq!((boundaries[1].byte_start, boundaries[1].byte_end), (5, 8));
    }

    /// A document whose pages each hold a Helvetica line of real words, then a Type0 line whose
    /// ToUnicode maps code 0x0041 to "A" and code 0x0044 to NUL (a glyph without a Unicode value).
    fn build_type0_lost_glyph_pdf_pages(page_texts: &[&str]) -> Vec<u8> {
        let to_unicode = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap \
            /CMapName /Test-UCS def /CMapType 2 def \
            1 begincodespacerange <0000> <FFFF> endcodespacerange \
            5 beginbfchar <0041> <0041> <0042> <0042> <0031> <0031> <0032> <0032> <0044> <0000> endbfchar endcmap end end";
        let first_page_object = 8;
        let kids: Vec<String> = (0..page_texts.len())
            .map(|index| format!("{} 0 R", first_page_object + 2 * index))
            .collect();
        let mut objects: Vec<String> = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            format!(
                "<< /Type /Pages /Kids [{}] /Count {} >>",
                kids.join(" "),
                page_texts.len()
            ),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
            "<< /Type /Font /Subtype /Type0 /BaseFont /AAAAAA+Test /Encoding /Identity-H \
             /DescendantFonts [5 0 R] /ToUnicode 7 0 R >>"
                .to_string(),
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /AAAAAA+Test \
             /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> \
             /DW 600 /FontDescriptor 6 0 R /CIDToGIDMap /Identity >>"
                .to_string(),
            "<< /Type /FontDescriptor /FontName /AAAAAA+Test /Flags 4 /FontBBox [0 0 1000 1000] \
             /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 >>"
                .to_string(),
            format!(
                "<< /Length {} >>\nstream\n{to_unicode}\nendstream",
                to_unicode.len() + 1
            ),
        ];
        for (index, text) in page_texts.iter().enumerate() {
            let content = format!(
                "BT /F1 12 Tf 72 700 Td ({text}) Tj /F2 12 Tf 0 -20 Td <00410044> Tj 0 -20 Td <004100440042> Tj 0 -20 Td <003100440032> Tj ET\n"
            );
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {} 0 R \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> >>",
                first_page_object + 2 * index + 1
            ));
            objects.push(format!("<< /Length {} >>\nstream\n{content}endstream", content.len()));
        }
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
        }
        let xref_pos = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", objects.len() + 1).as_bytes());
        for offset in &offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n",
                objects.len() + 1
            )
            .as_bytes(),
        );
        pdf
    }

    fn build_type0_lost_glyph_pdf() -> Vec<u8> {
        build_type0_lost_glyph_pdf_pages(&["Real words stay."])
    }

    async fn extract(config: &ExtractionConfig) -> crate::types::ExtractedDocument {
        let document = PdfExtractor::new()
            .extract_content(&build_type0_lost_glyph_pdf(), "application/pdf", config)
            .await
            .expect("the synthetic PDF extracts");
        crate::extraction::derive::derive_extraction_result(document, false, crate::core::config::OutputFormat::Plain)
    }

    /// The product path: a PDF with no `ocr` block and a Type0 glyph without Unicode.
    /// The caller sees the real words, no marker, and a warning that says how to recover the page.
    #[tokio::test]
    async fn extraction_without_ocr_hides_the_marker_and_warns() {
        let config = ExtractionConfig {
            ocr_near_empty_fallback: Some(false),
            ..ExtractionConfig::default()
        };

        let result = extract(&config).await;

        assert!(
            result.content.contains("Real words stay."),
            "content: {:?}",
            result.content
        );
        assert!(
            result.content.contains('A'),
            "the mapped Type0 glyph is read: {:?}",
            result.content
        );
        assert!(
            !result.content.contains(LOST_GLYPH_MARKER),
            "marker in content: {:?}",
            result.content
        );
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|w| w.message.contains("no Unicode value")),
            "warnings: {:?}",
            result.processing_warnings
        );
    }

    /// Native text keeps a boundary where a glyph was lost, so identifiers and numbers do not fuse.
    async fn assert_lost_glyphs_leave_a_boundary(config: &ExtractionConfig) {
        let result = extract(config).await;

        assert!(
            result.content.contains("A B"),
            "A<lost>B must not fuse: {:?}",
            result.content
        );
        assert!(
            result.content.contains("1 2"),
            "1<lost>2 must not fuse: {:?}",
            result.content
        );
        assert!(
            !result.content.contains("AB") && !result.content.contains("12"),
            "content: {:?}",
            result.content
        );
        assert!(
            !result.content.contains(LOST_GLYPH_MARKER),
            "marker in content: {:?}",
            result.content
        );
    }

    #[tokio::test]
    async fn native_text_with_lost_glyphs_keeps_a_boundary_without_ocr() {
        let config = ExtractionConfig {
            ocr_near_empty_fallback: Some(false),
            ..ExtractionConfig::default()
        };
        assert_lost_glyphs_leave_a_boundary(&config).await;
    }

    #[tokio::test]
    async fn native_text_with_lost_glyphs_keeps_a_boundary_with_ocr_off() {
        let config = ExtractionConfig {
            disable_ocr: true,
            ..ExtractionConfig::default()
        };
        assert_lost_glyphs_leave_a_boundary(&config).await;
    }

    /// With OCR switched off the marker leaves the text, and the caller keeps a signal that
    /// glyphs are missing: nothing recovered them.
    #[tokio::test]
    async fn extraction_with_ocr_off_hides_the_marker_and_keeps_the_loss_signal() {
        let config = ExtractionConfig {
            disable_ocr: true,
            ..ExtractionConfig::default()
        };

        let result = extract(&config).await;

        assert!(
            result.content.contains("Real words stay."),
            "content: {:?}",
            result.content
        );
        assert!(
            !result.content.contains(LOST_GLYPH_MARKER),
            "marker in content: {:?}",
            result.content
        );
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|w| w.message.contains("no Unicode value")),
            "silent loss with OCR off: {:?}",
            result.processing_warnings
        );
    }

    /// A stand-in OCR backend that returns a fixed transcript and counts its calls.
    #[cfg(feature = "ocr")]
    struct RecordingOcrBackend {
        transcript: &'static str,
        calls: std::sync::atomic::AtomicUsize,
    }

    #[cfg(feature = "ocr")]
    impl crate::plugins::Plugin for RecordingOcrBackend {
        fn name(&self) -> &str {
            "unmapped-glyphs-recording-ocr"
        }

        fn version(&self) -> String {
            "1.0.0".to_string()
        }

        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[cfg(feature = "ocr")]
    #[async_trait::async_trait]
    impl crate::plugins::OcrBackend for RecordingOcrBackend {
        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }

        fn supports_language(&self, _lang: &str) -> bool {
            true
        }

        async fn process_image(
            &self,
            _image_bytes: &[u8],
            _config: &OcrConfig,
        ) -> crate::Result<crate::types::ExtractedDocument> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(crate::types::ExtractedDocument {
                content: self.transcript.to_string(),
                mime_type: std::borrow::Cow::Borrowed("text/plain"),
                ..Default::default()
            })
        }
    }

    /// Mixed page set: page 1 goes to OCR and its transcript holds a real U+FFFD, page 2 stays
    /// native and lost a glyph. The transcript keeps its character, the native loss is removed
    /// and still reported.
    #[cfg(feature = "ocr")]
    #[tokio::test]
    #[serial_test::serial]
    async fn mixed_pages_keep_an_ocr_replacement_character_and_report_the_native_loss() {
        const TRANSCRIPT: &str = "Scanned transcript with a damaged letter A\u{FFFD}B and enough alphanumeric \
            characters to clear the retention guard for the page that OCR replaces";
        let backend = std::sync::Arc::new(RecordingOcrBackend {
            transcript: TRANSCRIPT,
            calls: std::sync::atomic::AtomicUsize::new(0),
        });
        crate::plugins::register_ocr_backend(backend.clone()).expect("the stand-in backend registers");
        let config = ExtractionConfig {
            force_ocr_pages: Some(vec![1]),
            ocr: Some(OcrConfig {
                backend: "unmapped-glyphs-recording-ocr".to_string(),
                language: vec!["eng".to_string()],
                ..OcrConfig::default()
            }),
            pages: Some(crate::core::config::PageConfig {
                extract_pages: true,
                ..Default::default()
            }),
            use_cache: false,
            ..ExtractionConfig::default()
        };

        let document = PdfExtractor::new()
            .extract_content(
                &build_type0_lost_glyph_pdf_pages(&["First page words.", "Second page words."]),
                "application/pdf",
                &config,
            )
            .await;
        let _ = crate::plugins::unregister_ocr_backend("unmapped-glyphs-recording-ocr");
        let result = crate::extraction::derive::derive_extraction_result(
            document.expect("the two page PDF extracts"),
            false,
            crate::core::config::OutputFormat::Plain,
        );

        assert_eq!(
            backend.calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "only page 1 reaches OCR"
        );
        assert!(
            result.content.contains(TRANSCRIPT),
            "the OCR transcript keeps its own U+FFFD: {:?}",
            result.content
        );
        assert_eq!(
            result.content.matches(LOST_GLYPH_MARKER).count(),
            1,
            "the native loss on page 2 is gone: {:?}",
            result.content
        );
        assert!(
            result.content.contains("Second page words."),
            "content: {:?}",
            result.content
        );
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|w| w.message.contains("no Unicode value")),
            "the native page still reports its loss: {:?}",
            result.processing_warnings
        );
    }
}
