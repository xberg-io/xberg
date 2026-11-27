//! Painted page content that text can sit on.
//!
//! White text is still the page's content when something painted lies under it: labels on a dark
//! box or a photo. Over nothing it cannot be seen, and extraction drops it
//! (`TextExtractor::invisible_text_page`).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::{MutexExt, PdfDocument};
use crate::content::Operator;
use crate::geometry::Rect;
use crate::layout::Color;
use crate::object::{Object, ObjectRef};

/// Operators read when auditing a page stream and the Forms it paints for paint the path walker
/// cannot colour, shared by the whole page.
const MAX_AUDIT_OPERATORS: usize = 1_000_000;
/// Decoded Form bytes one page audit reads.
const MAX_AUDIT_FORM_BYTES: usize = 64 * 1024 * 1024;
/// Deepest Form nesting the audit follows; a deeper paint is not resolved.
const MAX_AUDIT_FORM_DEPTH: usize = 16;
/// Pages whose painted regions stay memoized during the page passes. A page is interpreted a few
/// times in a row (per script geometry, per filter set), not again pages later.
pub(super) const PAINTED_REGIONS_CACHE_PAGES: usize = 8;
/// Smallest channel value of a white fill or stroke, for paths and for text alike.
pub(crate) const WHITE_PAINT_MIN: f32 = 0.999;

/// Whether `name` is a colour space whose values the text extractor resolves to a colour: the
/// device spaces and the calibrated ones, read like their device counterparts.
pub(crate) fn is_resolved_colour_space(name: &str) -> bool {
    matches!(name, "DeviceGray" | "DeviceRGB" | "DeviceCMYK" | "CalGray" | "CalRGB")
}

/// Whether the path walker knows the current fill and stroke colour. It models `rg`, `g` and `k`
/// (and their stroke forms) only: after `cs`, `sc` or `scn` its colour is stale.
#[derive(Clone, Copy)]
struct ColourModel {
    fill: bool,
    stroke: bool,
}

/// The work one page audit may still do, shared by the page stream and every Form it paints.
struct PaintAudit {
    operators: usize,
    form_bytes: usize,
    /// The Forms being audited, outermost first.
    forms: Vec<ObjectRef>,
    /// Verdicts of Forms with their own `/Resources`, by the colour model they were painted under:
    /// a logo placed on every row is audited once.
    verdicts: HashMap<(ObjectRef, bool, bool), bool>,
}

impl PdfDocument {
    /// Keep white text even where nothing painted lies under it. By default such text is dropped
    /// as unseen; a page whose painted content the check cannot fully resolve keeps it either way.
    /// Set it before reading pages: changing it empties the per-page span and character caches.
    pub fn set_keep_white_text(&self, keep: bool) {
        if self.keep_white_text.swap(keep, Ordering::Relaxed) == keep {
            return;
        }
        if let Some(cache) = self.raw_page_spans_cache.lock_or_recover().as_mut() {
            cache.clear();
        }
        self.page_spans_cache.lock_or_recover().clear();
        self.page_chars_cache.lock_or_recover().clear();
        self.search_index.lock_or_recover().clear();
    }

    pub(crate) fn keeps_white_text(&self) -> bool {
        self.keep_white_text.load(Ordering::Relaxed)
    }

    /// `(page index, span count)` of each page that dropped white text over unpainted page area,
    /// in page order. A page read several times counts the most spans any one read dropped.
    pub fn dropped_white_text_spans(&self) -> Vec<(usize, usize)> {
        let mut dropped: Vec<(usize, usize)> = self
            .dropped_white_text
            .lock_or_recover()
            .iter()
            .map(|(&page, &count)| (page, count))
            .collect();
        dropped.sort_unstable();
        dropped
    }

    pub(crate) fn note_dropped_white_text(&self, page_index: usize, spans: usize) {
        let mut dropped = self.dropped_white_text.lock_or_recover();
        let count = dropped.entry(page_index).or_insert(0);
        *count = (*count).max(spans);
    }

    /// The regions of `page_index` covered by painted content other than white: filled or stroked
    /// paths of any other colour and every image. `None` when the page or a Form it paints draws
    /// content whose colour the path walker does not resolve (a shading, a fill colour space other
    /// than a device or calibrated one, or a path painted in a colour set by `cs`, `sc` or `scn`)
    /// or the page cannot be read, so the caller keeps all text. Memoized during the page passes.
    pub(crate) fn painted_regions(&self, page_index: usize) -> Option<Arc<Vec<Rect>>> {
        if let Some(cache) = self.painted_regions_cache.lock_or_recover().as_mut()
            && let Some(regions) = cache.get(&page_index)
        {
            return regions.clone();
        }
        let regions = self.read_painted_regions(page_index).map(Arc::new);
        if let Some(cache) = self.painted_regions_cache.lock_or_recover().as_mut() {
            cache.insert(page_index, regions.clone());
        }
        regions
    }

    fn read_painted_regions(&self, page_index: usize) -> Option<Vec<Rect>> {
        let content = self.cached_page_content(page_index).ok()?;
        let page = self.get_page(page_index).ok()?;
        let resources = page
            .as_dict()
            .and_then(|page| page.get("Resources"))
            .and_then(|resources| self.deref_object(resources));
        let mut audit = PaintAudit {
            operators: MAX_AUDIT_OPERATORS,
            form_bytes: MAX_AUDIT_FORM_BYTES,
            forms: Vec::new(),
            verdicts: HashMap::new(),
        };
        let start = ColourModel {
            fill: true,
            stroke: true,
        };
        if !self.paints_resolved_colours(&content, resources.as_ref(), start, &mut audit) {
            return None;
        }
        let mut regions: Vec<Rect> = self
            .extract_paths(page_index)
            .ok()?
            .iter()
            .filter(|path| {
                path.fill_color.is_some_and(|color| !is_white(color))
                    || (path.has_stroke() && path.stroke_color.is_some_and(|color| !is_white(color)))
            })
            .map(crate::elements::PathContent::rendered_bbox)
            .collect();
        regions.extend(self.page_image_handles(page_index).ok()?.iter().map(|image| image.bbox));
        Some(regions)
    }

    /// Whether `content`, and every Form it paints, colours its paths only in ways the path walker
    /// models. Running out of the audit budget or failing to read a Form counts as not.
    fn paints_resolved_colours(
        &self,
        content: &[u8],
        resources: Option<&Object>,
        start: ColourModel,
        audit: &mut PaintAudit,
    ) -> bool {
        let xobjects = resources
            .and_then(Object::as_dict)
            .and_then(|resources| resources.get("XObject"))
            .and_then(|xobjects| self.deref_object(xobjects));
        let mut model = start;
        let mut saved: Vec<ColourModel> = Vec::new();
        let limit = audit.operators.saturating_add(1);
        let visited = crate::content::parser::visit_complete_content_stream(content, limit, |op| {
            if audit.operators == 0 {
                return false;
            }
            audit.operators -= 1;
            match op {
                Operator::PaintShading { .. } => false,
                Operator::SaveState => {
                    saved.push(model);
                    true
                }
                Operator::RestoreState => {
                    model = saved.pop().unwrap_or(model);
                    true
                }
                Operator::SetFillRgb { .. } | Operator::SetFillGray { .. } | Operator::SetFillCmyk { .. } => {
                    model.fill = true;
                    true
                }
                Operator::SetStrokeRgb { .. } | Operator::SetStrokeGray { .. } | Operator::SetStrokeCmyk { .. } => {
                    model.stroke = true;
                    true
                }
                Operator::SetFillColorSpace { name } => {
                    model.fill = false;
                    is_resolved_colour_space(&name)
                }
                Operator::SetFillColor { .. } | Operator::SetFillColorN { .. } => {
                    model.fill = false;
                    true
                }
                Operator::SetStrokeColorSpace { .. }
                | Operator::SetStrokeColor { .. }
                | Operator::SetStrokeColorN { .. } => {
                    model.stroke = false;
                    true
                }
                Operator::Fill | Operator::FillEvenOdd => model.fill,
                Operator::Stroke | Operator::CloseStroke => model.stroke,
                Operator::FillStroke
                | Operator::FillStrokeEvenOdd
                | Operator::CloseFillStroke
                | Operator::CloseFillStrokeEvenOdd => model.fill && model.stroke,
                Operator::Do { name } => {
                    let reference = xobjects
                        .as_ref()
                        .and_then(Object::as_dict)
                        .and_then(|xobjects| xobjects.get(&name))
                        .and_then(Object::as_reference);
                    reference
                        .is_none_or(|reference| self.form_paints_resolved_colours(reference, resources, model, audit))
                }
                _ => true,
            }
        });
        visited.unwrap_or(false)
    }

    /// [`Self::paints_resolved_colours`] for the XObject `reference`, painted with the caller's
    /// `resources` in effect. An Image XObject paints no path; a Form already being audited
    /// further out paints nothing new.
    fn form_paints_resolved_colours(
        &self,
        reference: ObjectRef,
        resources: Option<&Object>,
        model: ColourModel,
        audit: &mut PaintAudit,
    ) -> bool {
        let verdict_key = (reference, model.fill, model.stroke);
        if let Some(&verdict) = audit.verdicts.get(&verdict_key) {
            return verdict;
        }
        if audit.forms.contains(&reference) {
            return true;
        }
        if audit.forms.len() >= MAX_AUDIT_FORM_DEPTH {
            return false;
        }
        let Ok(xobject) = self.load_object(reference) else {
            return false;
        };
        let Some(dict) = xobject.as_dict() else {
            return false;
        };
        if dict.get("Subtype").and_then(Object::as_name) != Some("Form") {
            return true;
        }
        let Ok(data) = self.decode_form_stream_with_limit(&xobject, reference, audit.form_bytes) else {
            return false;
        };
        let Some(remaining) = audit.form_bytes.checked_sub(data.len()) else {
            return false;
        };
        audit.form_bytes = remaining;
        let form_resources = dict.get("Resources").and_then(|own| self.deref_object(own));
        audit.forms.push(reference);
        let resolved = self.paints_resolved_colours(&data, form_resources.as_ref().or(resources), model, audit);
        audit.forms.pop();
        if form_resources.is_some() {
            audit.verdicts.insert(verdict_key, resolved);
        }
        resolved
    }
}

fn is_white(color: Color) -> bool {
    [color.r, color.g, color.b]
        .iter()
        .all(|channel| *channel >= WHITE_PAINT_MIN)
}
