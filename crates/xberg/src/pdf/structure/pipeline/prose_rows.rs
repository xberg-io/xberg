use super::*;
use crate::pdf::native::hierarchy::script_evidence::NativePageSegments;

pub(super) fn preserve_certified_panel_rows(
    paragraphs: &mut Vec<PdfParagraph>,
    page: &NativePageSegments,
    hyphens: &HyphenWitnesses,
) {
    if paragraphs.iter().any(|paragraph| !is_plain_paragraph(paragraph)) {
        return;
    }
    let segments = paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.lines)
        .flat_map(|line| line.segments.iter().cloned())
        .collect::<Vec<_>>();
    let Some(rows) = page.surviving_prose_rows(&segments) else {
        return;
    };
    let mut repaired = Vec::new();
    for row in rows {
        let mut projected = super::blocks_to_paragraphs(row, &[], &[]);
        if projected.len() != 1 || !is_plain_paragraph(&projected[0]) {
            return;
        }
        super::synchronize_paragraph_text_metadata(&mut projected);
        repaired.append(&mut projected);
    }
    let effective = |paragraphs: &[PdfParagraph]| {
        paragraphs
            .iter()
            .map(|paragraph| rendered_plain_paragraph(paragraph, hyphens))
            .collect::<Vec<_>>()
    };
    let before = effective(paragraphs);
    let after = effective(&repaired);
    if !before
        .iter()
        .flat_map(|text| text.split_whitespace())
        .eq(after.iter().flat_map(|text| text.split_whitespace()))
    {
        return;
    }
    *paragraphs = repaired;
}

fn rendered_plain_paragraph(paragraph: &PdfParagraph, hyphens: &HyphenWitnesses) -> String {
    if paragraph.text.is_empty() {
        super::super::assembly::extract_text_and_annotations(paragraph, hyphens).0
    } else {
        paragraph.text.clone()
    }
}

fn is_plain_paragraph(paragraph: &PdfParagraph) -> bool {
    paragraph_text_matches_retained_segments(paragraph)
        && paragraph.heading_level.is_none()
        && !paragraph.is_bold
        && !paragraph.is_list_item
        && !paragraph.is_code_block
        && !paragraph.is_formula
        && !paragraph.is_page_furniture
        && paragraph.layout_class.is_none()
        && paragraph.layout_region_path.is_none()
        && paragraph.caption_for.is_none()
        && paragraph.lines.iter().all(|line| {
            line.native_scripts.is_empty() && line.segments.iter().all(|segment| segment.assigned_role.is_none())
        })
}

fn paragraph_text_matches_retained_segments(paragraph: &PdfParagraph) -> bool {
    paragraph.text.is_empty()
        || paragraph
            .text
            .chars()
            .filter(|character| !character.is_whitespace())
            .eq(paragraph
                .lines
                .iter()
                .flat_map(|line| &line.segments)
                .flat_map(|segment| segment.text.chars())
                .filter(|character| !character.is_whitespace()))
}

#[cfg(test)]
mod tests {
    use super::*;
    type ParagraphMutation = (&'static str, fn(&mut PdfParagraph));

    mod fixture {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/helpers/pdf_panels.rs"));
    }

    fn panel_page(accurate: bool) -> (NativePageSegments, Vec<PdfParagraph>) {
        let (bytes, _) = if accurate {
            fixture::accurate_helvetica_panels(1.0)
        } else {
            fixture::issue_2323_panels(1.0, 2)
        };
        let native = xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap();
        let spans = native
            .extract_page_text_with_script_geometry(0, xberg_native_pdf::document::ReadingOrder::ColumnAware)
            .unwrap()
            .spans;
        let page = NativePageSegments::from_final_spans(
            spans,
            &Default::default(),
            crate::pdf::native::hierarchy::ArtifactRetention::default(),
        );
        let paragraphs =
            super::super::segments_to_paragraphs(page.segments.clone(), &[], &[], &TextRepairWitnesses::default());
        let segments = paragraphs
            .iter()
            .flat_map(|paragraph| &paragraph.lines)
            .flat_map(|line| line.segments.iter().cloned())
            .collect::<Vec<_>>();
        assert!(page.surviving_prose_rows(&segments).is_some());
        (page, paragraphs)
    }

    #[test]
    fn a_complete_projection_can_retain_all_eight_certified_rows() {
        for accurate in [false, true] {
            let (page, mut paragraphs) = panel_page(accurate);
            let original = paragraphs
                .iter()
                .flat_map(|paragraph| &paragraph.lines)
                .flat_map(|line| line.segments.iter().cloned())
                .collect::<Vec<_>>();
            preserve_certified_panel_rows(&mut paragraphs, &page, &Default::default());
            assert_eq!(paragraphs.len(), 8);
            let returned = paragraphs
                .iter()
                .flat_map(|paragraph| &paragraph.lines)
                .flat_map(|line| line.segments.iter().cloned())
                .collect::<Vec<_>>();
            assert_eq!(original.len(), returned.len());
            assert!(page.surviving_prose_rows(&returned).is_some());
        }
    }

    #[test]
    fn rich_paragraph_roles_prevent_resplitting() {
        let changes: [ParagraphMutation; 7] = [
            ("changed text", |paragraph| {
                paragraph.text = "count preserving replacement".into()
            }),
            ("heading", |paragraph| paragraph.heading_level = Some(2)),
            ("list", |paragraph| paragraph.is_list_item = true),
            ("formula", |paragraph| paragraph.is_formula = true),
            ("caption", |paragraph| paragraph.caption_for = Some(0)),
            ("layout", |paragraph| {
                paragraph.layout_class = Some(LayoutHintClass::Text)
            }),
            ("bold", |paragraph| paragraph.is_bold = true),
        ];
        for (name, change) in changes {
            let (page, mut paragraphs) = panel_page(false);
            change(&mut paragraphs[0]);
            let before = format!("{paragraphs:?}");
            preserve_certified_panel_rows(&mut paragraphs, &page, &Default::default());
            assert_eq!(format!("{paragraphs:?}"), before, "{name}");
        }
    }

    #[test]
    fn an_existing_word_boundary_repair_is_not_undone() {
        let (page, mut paragraphs) = panel_page(false);
        let original = rendered_plain_paragraph(&paragraphs[0], &Default::default());
        assert!(original.contains("harbor meadow"));
        paragraphs[0].text = original.replacen("harbor meadow", "harbormeadow", 1);
        let before = format!("{paragraphs:?}");
        preserve_certified_panel_rows(&mut paragraphs, &page, &Default::default());
        assert_eq!(format!("{paragraphs:?}"), before);
    }

    #[test]
    fn missing_duplicated_or_substituted_source_evidence_prevents_resplitting() {
        let (page, paragraphs) = panel_page(false);
        let original = paragraphs
            .iter()
            .flat_map(|paragraph| &paragraph.lines)
            .flat_map(|line| line.segments.iter().cloned())
            .collect::<Vec<_>>();
        for mutation in 0..3 {
            let mut segments = original.clone();
            match mutation {
                0 => {
                    segments.pop();
                }
                1 => segments[1] = segments[0].clone(),
                _ => segments[0].text = "count preserving substitution".into(),
            }
            assert!(page.surviving_prose_rows(&segments).is_none(), "mutation {mutation}");
        }
    }
}
