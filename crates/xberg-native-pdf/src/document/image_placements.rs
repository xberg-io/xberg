//! Bounded image placement evidence for page-local geometry.

use super::PdfDocument;

impl PdfDocument {
    /// Audit image placements in a bounded page stream. A Form or unresolved
    /// XObject makes the audit inconclusive.
    #[doc(hidden)]
    pub fn bounded_page_image_placements(&self, page_index: usize) -> Option<Vec<crate::geometry::Rect>> {
        use crate::content::parser::visit_complete_content_stream;
        use crate::content::{Matrix, Operator};
        use crate::geometry::Rect;

        let content = self.bounded_image_audit_content(page_index)?;
        let mut resources = None;
        let mut image_names = std::collections::HashSet::new();
        let mut ctm = Matrix::identity();
        let mut stack = Vec::new();
        let mut placements = Vec::new();
        let complete = visit_complete_content_stream(&content, 32_768, |op| {
            match op {
                Operator::SaveState => stack.push(ctm),
                Operator::RestoreState => match stack.pop() {
                    Some(saved) => ctm = saved,
                    None => return false,
                },
                Operator::Cm { a, b, c, d, e, f } => {
                    ctm = Matrix { a, b, c, d, e, f }.multiply(&ctm);
                    if ![ctm.a, ctm.b, ctm.c, ctm.d, ctm.e, ctm.f]
                        .into_iter()
                        .all(f32::is_finite)
                    {
                        return false;
                    }
                }
                Operator::Do { ref name } if !image_names.contains(name) => {
                    let is_image = resources
                        .get_or_insert_with(|| self.get_page_resources(page_index).ok())
                        .as_ref()
                        .is_some_and(|resources| self.is_image_xobject(resources, name));
                    if !is_image {
                        return false;
                    }
                    image_names.insert(name.clone());
                }
                _ => {}
            }
            if matches!(op, Operator::Do { .. } | Operator::InlineImage { .. }) {
                if placements.len() == 4096 {
                    return false;
                }
                let bounds = self.transform_bbox_with_ctm(&Rect::new(0.0, 0.0, 1.0, 1.0), ctm);
                if ![bounds.left(), bounds.right(), bounds.top(), bounds.bottom()]
                    .into_iter()
                    .all(f32::is_finite)
                {
                    return false;
                }
                placements.push(bounds);
            }
            true
        });
        (matches!(complete, Ok(true)) && stack.is_empty()).then_some(placements)
    }

    fn bounded_image_audit_content(&self, page_index: usize) -> Option<Vec<u8>> {
        const MAX_PAGE_BYTES: usize = 4 * 1024 * 1024;
        const MAX_PAGE_STREAMS: usize = 256;
        self.ensure_encryption_initialized().ok()?;
        let page = self.get_page(page_index).ok()?;
        let contents = page.as_dict()?.get("Contents")?;
        let resolved = self.deref_object(contents)?;
        let mut content = Vec::new();
        let streams = resolved
            .as_array()
            .map_or(std::slice::from_ref(contents), |array| array);
        if streams.len() > MAX_PAGE_STREAMS {
            return None;
        }
        for stream in streams {
            // A page content stream is indirect. Reserve one byte to delimit stream boundaries. ~keep
            let reference = stream.as_reference()?;
            let object = self.load_object(reference).ok()?;
            let remaining = MAX_PAGE_BYTES.checked_sub(content.len() + 1)?;
            let decoded = self.decode_form_stream_with_limit(&object, reference, remaining).ok()?;
            content.extend(decoded);
            content.push(b'\n');
        }
        Some(content)
    }
}
