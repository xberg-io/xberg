//! Paint order of a page's text against a picture that covers the page.
//!
//! The `impl` is the same inherent `impl PdfDocument`. ~keep

use super::*;
use crate::content::{Matrix, Operator, TextElement};

/// Deepest chain of form XObjects this walk follows.
const MAX_FORM_DEPTH: usize = 100;

/// Glyph bytes a page shows, counted by [`PdfDocument::glyphs_over_covering_image`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphsOverPicture {
    /// Visible glyph bytes shown after the last image that covers the page.
    pub over: usize,
    /// Visible glyph bytes shown anywhere on the page.
    pub visible: usize,
    /// Glyph bytes shown anywhere on the page, invisible ones (render mode 3 or 7) included.
    pub shown: usize,
}

/// Most operators one page walk reads, its own and those of every form it paints. A form painted
/// twice by each of a chain of forms is walked 2^depth times; the walk stops at this bound.
const MAX_WALK_OPERATORS: usize = 1_000_000;

/// The running counts of one page walk.
struct PictureTextWalk {
    page: (f32, f32, f32, f32),
    min_coverage: f32,
    shown: usize,
    visible: usize,
    over: usize,
    covered: bool,
    forms: Vec<ObjectRef>,
    budget: usize,
}

impl PdfDocument {
    /// The glyph bytes the page shows in `Tj`, `TJ`, `'` and `"`: those shown visibly after the
    /// last image that alone covers at least `min_coverage` of its media box, those shown visibly,
    /// and all of them. A glyph in render mode 3 (invisible) or 7 (clip only) is not visible. Form
    /// XObjects are followed with their `/Matrix` and resources. `None` when no single image covers
    /// the page, when the page's content cannot be read, or when the walk reads more than
    /// [`MAX_WALK_OPERATORS`] operators.
    pub fn glyphs_over_covering_image(&self, page_index: usize, min_coverage: f32) -> Option<GlyphsOverPicture> {
        let (x0, y0, x1, y1) = self.get_page_media_box(page_index).ok()?;
        let resources = self.get_page_resources(page_index).ok()?;
        let operators = crate::content::parse_content_stream(&self.get_page_content_data(page_index).ok()?).ok()?;
        let mut walk = PictureTextWalk {
            page: (x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)),
            min_coverage,
            shown: 0,
            visible: 0,
            over: 0,
            covered: false,
            forms: Vec::new(),
            budget: MAX_WALK_OPERATORS,
        };
        self.walk_picture_text(&operators, &resources, Matrix::identity(), 0, &mut walk)?;
        walk.covered.then_some(GlyphsOverPicture {
            over: walk.over,
            visible: walk.visible,
            shown: walk.shown,
        })
    }

    /// Walks one content stream, starting from the CTM and text render mode in effect where it is
    /// painted. A `Q` with no `q` of this stream leaves the state as it is. `None` when the walk
    /// runs out of its operator budget.
    fn walk_picture_text(
        &self,
        operators: &[Operator],
        resources: &Object,
        ctm: Matrix,
        render: u8,
        walk: &mut PictureTextWalk,
    ) -> Option<()> {
        walk.budget = walk.budget.checked_sub(operators.len())?;
        let (mut ctm, mut render, mut saved) = (ctm, render, Vec::new());
        for operator in operators {
            let shown = match operator {
                Operator::SaveState => {
                    saved.push((ctm, render));
                    0
                }
                Operator::RestoreState => {
                    (ctm, render) = saved.pop().unwrap_or((ctm, render));
                    0
                }
                Operator::Cm { a, b, c, d, e, f } => {
                    ctm = Matrix {
                        a: *a,
                        b: *b,
                        c: *c,
                        d: *d,
                        e: *e,
                        f: *f,
                    }
                    .multiply(&ctm);
                    0
                }
                Operator::Tr { render: mode } => {
                    render = *mode;
                    0
                }
                Operator::Do { name } => {
                    self.paint_picture_text_xobject(name, resources, ctm, render, walk)?;
                    0
                }
                Operator::InlineImage { .. } => {
                    walk.paint_image(&ctm);
                    0
                }
                Operator::Tj { text } | Operator::Quote { text } | Operator::DoubleQuote { text, .. } => text.len(),
                Operator::TJ { array } => array
                    .iter()
                    .map(|element| match element {
                        TextElement::String(bytes) => bytes.len(),
                        TextElement::Offset(_) => 0,
                    })
                    .sum(),
                _ => 0,
            };
            walk.shown += shown;
            if !matches!(render, 3 | 7) {
                walk.visible += shown;
                walk.over += shown;
            }
        }
        Some(())
    }

    /// Paints the XObject `name` of `resources`: an image may cover the page, a form is walked
    /// with its own matrix and resources (its parent's when it has none). `None` when the walk runs
    /// out of its operator budget.
    fn paint_picture_text_xobject(
        &self,
        name: &str,
        resources: &Object,
        ctm: Matrix,
        render: u8,
        walk: &mut PictureTextWalk,
    ) -> Option<()> {
        let entry = resources
            .as_dict()
            .and_then(|resources| resources.get("XObject"))
            .and_then(|xobjects| self.resolve_object(xobjects).ok())
            .and_then(|xobjects| xobjects.as_dict()?.get(name).cloned());
        let Some(entry) = entry else { return Some(()) };
        let Ok(xobject) = self.resolve_object(&entry) else {
            return Some(());
        };
        let Some(dict) = xobject.as_dict() else { return Some(()) };
        match dict.get("Subtype").and_then(Object::as_name) {
            Some("Image") => walk.paint_image(&ctm),
            Some("Form") => {
                let Some(form) = entry.as_reference() else {
                    return Some(());
                };
                if walk.forms.contains(&form) || walk.forms.len() >= MAX_FORM_DEPTH {
                    return Some(());
                }
                let Ok(data) = self.decode_stream_with_encryption(&xobject, form) else {
                    return Some(());
                };
                let Ok(operators) = crate::content::parse_content_stream(&data) else {
                    return Some(());
                };
                let form_resources = match dict.get("Resources") {
                    Some(own) => self.resolve_object(own).unwrap_or_else(|_| resources.clone()),
                    None => resources.clone(),
                };
                let matrix = dict
                    .get("Matrix")
                    .and_then(|matrix| self.parse_matrix_from_object(matrix))
                    .unwrap_or_else(Matrix::identity);
                walk.forms.push(form);
                let walked = self.walk_picture_text(&operators, &form_resources, matrix.multiply(&ctm), render, walk);
                walk.forms.pop();
                walked?;
            }
            _ => {}
        }
        Some(())
    }
}

impl PictureTextWalk {
    /// An image painted into the unit square under `ctm`: when it covers the page, the text
    /// shown so far is under it.
    fn paint_image(&mut self, ctm: &Matrix) {
        if self.covers_page(ctm) {
            (self.covered, self.over) = (true, 0);
        }
    }

    fn covers_page(&self, ctm: &Matrix) -> bool {
        let (left, bottom, right, top) = self.page;
        let corners = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)].map(|(x, y)| ctm.transform_point(x, y));
        let (mut min_x, mut max_x, mut min_y, mut max_y) =
            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
        for point in corners {
            (min_x, max_x) = (min_x.min(point.x), max_x.max(point.x));
            (min_y, max_y) = (min_y.min(point.y), max_y.max(point.y));
        }
        let width = (max_x.min(right) - min_x.max(left)).max(0.0);
        let height = (max_y.min(top) - min_y.max(bottom)).max(0.0);
        let page_area = (right - left) * (top - bottom);
        page_area > f32::EPSILON && width * height >= self.min_coverage * page_area
    }
}
