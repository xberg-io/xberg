use super::{PageRenderer, is_paint_operator};
use crate::content::GraphicsState;
use crate::content::operators::Operator;
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::Object;
use std::collections::HashMap;
use tiny_skia::{Pixmap, Transform};

/// Whether a resolved `/Group` dictionary is a transparency group with `/K true`; pages and forms share it.
pub(super) fn group_is_knockout(group: &Object) -> bool {
    group.as_dict().is_some_and(|group| {
        group.get("S").and_then(Object::as_name) == Some("Transparency")
            && matches!(group.get("K"), Some(Object::Boolean(true)))
    })
}

pub(super) fn page_is_knockout(doc: &PdfDocument, page_num: usize) -> bool {
    let resolved = (|| {
        let page = doc.get_page(page_num)?;
        page.as_dict()
            .and_then(|dict| dict.get("Group"))
            .map(|group| doc.resolve_object(group))
            .transpose()
    })();
    match resolved {
        Ok(group) => group.as_ref().is_some_and(group_is_knockout),
        Err(error) => {
            tracing::warn!(
                error_code = error.telemetry_code(),
                error_offset = ?error.telemetry_offset(),
                "ignoring malformed optional page transparency group"
            );
            false
        }
    }
}

#[derive(Clone)]
pub(super) struct KnockoutShape {
    dimensions: (u32, u32),
    coverage: Vec<u8>,
    spots: HashMap<usize, Vec<u8>>,
}

impl KnockoutShape {
    pub(super) fn new(width: u32, height: u32) -> Self {
        Self {
            dimensions: (width, height),
            spots: HashMap::new(),
            coverage: vec![0; width as usize * height as usize],
        }
    }

    pub(super) fn allocation_bytes(&self) -> Option<usize> {
        super::knockout_resources::shape_bytes(self.coverage.len(), self.spots.len())
    }

    fn clear(&mut self) {
        self.coverage.fill(0);
        for coverage in self.spots.values_mut() {
            coverage.fill(0);
        }
    }

    pub(super) fn clip(&mut self, mask: &tiny_skia::Mask) {
        for coverage in std::iter::once(&mut self.coverage).chain(self.spots.values_mut()) {
            for (value, mask) in coverage.iter_mut().zip(mask.data()) {
                *value = ((u32::from(*value) * u32::from(*mask) + 127) / 255) as u8;
            }
        }
    }

    pub(super) fn include(&mut self, coverage: &[u8]) {
        union_coverage(&mut self.coverage, coverage);
    }

    fn include_spot(&mut self, index: usize, coverage: &[u8]) {
        let destination = self.spots.entry(index).or_insert_with(|| vec![0; coverage.len()]);
        union_coverage(destination, coverage);
    }

    pub(super) fn include_shape(&mut self, source: &Self) {
        self.include(&source.coverage);
        for (index, coverage) in &source.spots {
            self.include_spot(*index, coverage);
        }
    }
}

fn union_coverage(destination: &mut [u8], source: &[u8]) {
    for (destination, source) in destination.iter_mut().zip(source) {
        *destination = (255 - ((255 - u32::from(*destination)) * (255 - u32::from(*source)) + 127) / 255) as u8;
    }
}

pub(super) struct PatternShape {
    cell: Pixmap,
    pub(super) page: Pixmap,
    average: f32,
}

impl PatternShape {
    pub(super) fn new(shape: KnockoutShape, width: u32, height: u32) -> Result<Self> {
        let (cw, ch) = shape.dimensions;
        let mut cell = Pixmap::new(cw, ch).ok_or_else(|| Error::InvalidPdf("pattern shape alloc failed".into()))?;
        for (pixel, coverage) in cell.data_mut().as_chunks_mut::<4>().0.iter_mut().zip(&shape.coverage) {
            pixel.fill(*coverage);
        }
        let average = shape.coverage.iter().map(|value| f32::from(*value)).sum::<f32>() / shape.coverage.len() as f32;
        let page = Pixmap::new(width, height).ok_or_else(|| Error::InvalidPdf("pattern shape alloc failed".into()))?;
        Ok(Self { cell, page, average })
    }

    pub(super) fn paint_tile(&mut self, transform: Transform, mask: &tiny_skia::Mask) {
        self.page.draw_pixmap(
            0,
            0,
            self.cell.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            transform,
            Some(mask),
        );
    }
}

pub(super) struct KnockoutBackdrop {
    rgba: Vec<u8>,
    cmyk: Option<Vec<u8>>,
    spots: Option<Vec<u8>>,
}

impl KnockoutBackdrop {
    pub(super) fn capture(renderer: &PageRenderer, pixmap: &Pixmap) -> Self {
        #[cfg(test)]
        renderer
            .rgba_snapshot_bytes
            .fetch_add(pixmap.data().len(), std::sync::atomic::Ordering::Relaxed);
        Self {
            rgba: pixmap.data().to_vec(),
            cmyk: renderer.cmyk_sidecar.as_ref().map(|sidecar| sidecar.cmyk().to_vec()),
            spots: renderer
                .cmyk_sidecar
                .as_ref()
                .map(|sidecar| sidecar.spot_state().to_vec()),
        }
    }

    pub(super) fn restore(&self, renderer: &mut PageRenderer, pixmap: &mut Pixmap) {
        pixmap.data_mut().copy_from_slice(&self.rgba);
        if let Some(sidecar) = renderer.cmyk_sidecar.as_mut() {
            if let Some(cmyk) = &self.cmyk {
                sidecar.restore_cmyk(cmyk);
            }
            if let Some(spots) = &self.spots {
                sidecar.restore_spot_state(spots);
            }
        }
    }

    fn merge(&mut self, renderer: &PageRenderer, scratch: &Pixmap, backdrop: &Self, shape: &KnockoutShape) {
        merge_lanes(&mut self.rgba, scratch.data(), &backdrop.rgba, &shape.coverage, true);
        if let Some(sidecar) = &renderer.cmyk_sidecar {
            if let (Some(accumulator), Some(initial)) = (&mut self.cmyk, &backdrop.cmyk) {
                merge_lanes(accumulator, sidecar.cmyk(), initial, &shape.coverage, true);
            }
            if let (Some(accumulator), Some(initial)) = (&mut self.spots, &backdrop.spots) {
                let pixels = shape.coverage.len();
                let lanes = sidecar.spot_names().len();
                for (index, coverage) in &shape.spots {
                    for plane in (*index..accumulator.len() / pixels).step_by(lanes) {
                        let range = plane * pixels..(plane + 1) * pixels;
                        merge_lanes(
                            &mut accumulator[range.clone()],
                            &sidecar.spot_state()[range.clone()],
                            &initial[range],
                            coverage,
                            false,
                        );
                    }
                }
            }
        }
    }
}

fn merge_lanes(accumulator: &mut [u8], scratch: &[u8], initial: &[u8], shape: &[u8], interleaved: bool) {
    for (index, ((accumulator, scratch), initial)) in accumulator.iter_mut().zip(scratch).zip(initial).enumerate() {
        let pixel = if interleaved { index / 4 } else { index % shape.len() };
        // Scratch already includes fractional shape: ISO 32000-1 §11.4.6.2 gives
        // S + (1-f)·(A-B), rather than applying shape a second time. ~keep
        let retained = (1.0 - f32::from(shape[pixel]) / 255.0) * (f32::from(*accumulator) - f32::from(*initial));
        *accumulator = (f32::from(*scratch) + retained).round().clamp(0.0, 255.0) as u8;
    }
}

pub(super) struct KnockoutGroup {
    backdrop: KnockoutBackdrop,
    accumulator: KnockoutBackdrop,
    shape: KnockoutShape,
    pending_paint: bool,
}

impl KnockoutGroup {
    fn new(renderer: &PageRenderer, pixmap: &Pixmap) -> Self {
        Self {
            backdrop: KnockoutBackdrop::capture(renderer, pixmap),
            accumulator: KnockoutBackdrop::capture(renderer, pixmap),
            shape: KnockoutShape::new(pixmap.width(), pixmap.height()),
            pending_paint: false,
        }
    }

    pub(super) fn before_operator(&mut self, renderer: &mut PageRenderer, pixmap: &mut Pixmap, operator: &Operator) {
        self.finish_paint(renderer, pixmap);
        if !is_paint_operator(operator) {
            return;
        }
        self.backdrop.restore(renderer, pixmap);
        if let Some(shape) = renderer.knockout_shape.as_mut() {
            shape.clear();
        } else {
            renderer.knockout_shape = Some(KnockoutShape::new(pixmap.width(), pixmap.height()));
        }
        self.pending_paint = true;
    }

    pub(super) fn finish_paint(&mut self, renderer: &PageRenderer, pixmap: &Pixmap) {
        if self.pending_paint {
            if let Some(shape) = renderer.knockout_shape.as_ref() {
                self.accumulator.merge(renderer, pixmap, &self.backdrop, shape);
                self.shape.include_shape(shape);
            }
            self.pending_paint = false;
        }
    }
}

pub(super) fn is_path_paint(op: &Operator) -> bool {
    matches!(
        op,
        Operator::Fill
            | Operator::FillEvenOdd
            | Operator::Stroke
            | Operator::CloseStroke
            | Operator::FillStroke
            | Operator::FillStrokeEvenOdd
            | Operator::CloseFillStroke
            | Operator::CloseFillStrokeEvenOdd
    )
}

fn is_text_show(op: &Operator) -> bool {
    matches!(
        op,
        Operator::Tj { .. } | Operator::TJ { .. } | Operator::Quote { .. } | Operator::DoubleQuote { .. }
    )
}

fn path_pixels(pixmap: &Pixmap, path: &tiny_skia::Path, transform: Transform, padding: f64) -> usize {
    let Some([left, top, right, bottom]) = crate::rendering::device_bounds(path, transform) else {
        return 0;
    };
    let span = |lo: f64, hi: f64, size: u32| {
        let limit = f64::from(size);
        ((hi + padding).ceil().clamp(0.0, limit) - (lo - padding).floor().clamp(0.0, limit)) as usize
    };
    span(left, right, pixmap.width()) * span(top, bottom, pixmap.height())
}

impl PageRenderer {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_pattern_flood_shape(
        &mut self,
        pattern: Option<&PatternShape>,
        path: &tiny_skia::Path,
        transform: Transform,
        rule: tiny_skia::FillRule,
        clip: Option<&tiny_skia::Mask>,
        gs: &GraphicsState,
    ) {
        let Some(pattern) = pattern else { return };
        if let Some(mut coverage) = self.rasterise_fill_coverage(path, transform, rule, clip) {
            for value in &mut coverage {
                *value = (f32::from(*value) * pattern.average / 255.0).round() as u8;
            }
            self.record_knockout_shape(Some(&coverage), gs, true);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn collect_inline_image_shape(
        &mut self,
        image: &Object,
        transform: Transform,
        gs: &GraphicsState,
        doc: &PdfDocument,
        clip: Option<&tiny_skia::Mask>,
        is_mask: bool,
    ) -> Result<()> {
        let Some((width, height)) = self.coverage_dimensions() else {
            return Ok(());
        };
        let mut scratch =
            Pixmap::new(width, height).ok_or_else(|| Error::InvalidPdf("knockout image shape alloc failed".into()))?;
        let state = Self::coverage_only_gs(gs);
        let result = if is_mask {
            self.render_image_mask(&mut scratch, image, None, transform, doc, clip, &state)
        } else {
            self.render_image(&mut scratch, image, None, transform, doc, clip, None, None, &state)
        };
        if result.is_err() {
            return Ok(());
        }
        let coverage = Self::extract_alpha_as_coverage(&scratch);
        if is_mask {
            self.record_knockout_shape(Some(&coverage), gs, true);
        } else if let Some(shape) = self.knockout_shape.as_mut() {
            shape.include(&coverage);
        }
        Ok(())
    }

    pub(super) fn record_knockout_shape(&mut self, coverage: Option<&[u8]>, gs: &GraphicsState, fill_side: bool) {
        let (Some(shape), Some(coverage)) = (self.knockout_shape.as_mut(), coverage) else {
            return;
        };
        shape.include(coverage);
        let inks = if fill_side {
            &gs.fill_spot_inks
        } else {
            &gs.stroke_spot_inks
        };
        if let Some(sidecar) = &self.cmyk_sidecar {
            for (name, _) in inks {
                if let Some(index) = sidecar.spot_index(name) {
                    shape.include_spot(index, coverage);
                }
            }
        }
    }

    pub(super) fn charge_path_work(
        &mut self,
        pixmap: &Pixmap,
        path: &tiny_skia::Path,
        transform: Transform,
        gs: &GraphicsState,
        op: &Operator,
    ) -> Result<()> {
        if !self.knockout_resources.active() {
            return Ok(());
        }
        let full_surface = self.coverage_dimensions().is_some()
            || gs.smask.is_some()
            || gs.fill_overprint
            || gs.stroke_overprint
            || gs.fill_pattern_name.is_some()
            || crate::rendering::pdf_blend_mode_is_nonseparable(&gs.blend_mode).is_some();
        let pixels = if full_surface {
            pixmap.data().len() / 4
        } else {
            let stroke = !matches!(op, Operator::Fill | Operator::FillEvenOdd);
            let extent = if stroke {
                let scale = (transform.sx.abs() + transform.kx.abs()).max(transform.ky.abs() + transform.sy.abs());
                f64::from(gs.line_width.abs()) * f64::from(gs.miter_limit.max(1.0)) * f64::from(scale)
            } else {
                0.0
            };
            path_pixels(pixmap, path, transform, 1.0 + extent)
        };
        self.knockout_resources.raster(pixels)
    }

    pub(super) fn reserve_raster_workspace(
        &mut self,
        pixmap: &Pixmap,
        gs: &GraphicsState,
        coverage_bytes_per_pixel: usize,
    ) -> Result<Option<super::knockout_resources::Reservation>> {
        if !self.knockout_resources.active() {
            return Ok(None);
        }
        let coverage = if self.coverage_dimensions().is_some() {
            coverage_bytes_per_pixel
        } else {
            0
        };
        let snapshots = if self.cmyk_sidecar.is_some() {
            2
        } else if gs.fill_overprint || gs.stroke_overprint {
            1
        } else {
            0
        };
        let blend = usize::from(crate::rendering::pdf_blend_mode_is_nonseparable(&gs.blend_mode).is_some());
        let bytes = pixmap
            .data()
            .len()
            .checked_mul(snapshots + blend)
            .and_then(|value| (pixmap.data().len() / 4).checked_mul(coverage)?.checked_add(value));
        if bytes == Some(0) {
            return Ok(None);
        }
        self.knockout_resources.reserve(bytes).map(Some)
    }

    pub(super) fn reserve_operator_workspace(
        &mut self,
        pixmap: &Pixmap,
        gs: &GraphicsState,
        op: &Operator,
    ) -> Result<Option<super::knockout_resources::Reservation>> {
        let coverage = match op {
            Operator::Fill | Operator::FillEvenOdd => 1,
            Operator::FillStroke
            | Operator::FillStrokeEvenOdd
            | Operator::CloseFillStroke
            | Operator::CloseFillStrokeEvenOdd => 6,
            Operator::Stroke
            | Operator::CloseStroke
            | Operator::InlineImage { .. }
            | Operator::PaintShading { .. }
            | Operator::Tj { .. }
            | Operator::TJ { .. }
            | Operator::Quote { .. }
            | Operator::DoubleQuote { .. } => 5,
            _ => return Ok(None),
        };
        self.reserve_raster_workspace(pixmap, gs, coverage)
    }

    pub(super) fn coverage_dimensions(&self) -> Option<(u32, u32)> {
        self.knockout_shape
            .as_ref()
            .map(|shape| shape.dimensions)
            .or_else(|| self.cmyk_sidecar.as_ref().map(|sidecar| sidecar.dims()))
    }

    pub(super) fn execute_knockout_group(
        &mut self,
        pixmap: &mut Pixmap,
        base_transform: Transform,
        operators: &[Operator],
        doc: &PdfDocument,
        page_num: usize,
        resources: &Object,
    ) -> Result<Option<super::knockout_resources::WorkFallback>> {
        let pixels = pixmap.width() as usize * pixmap.height() as usize;
        let (sidecar_bytes, spot_count) = self.cmyk_sidecar.as_ref().map_or((0, 0), |sidecar| {
            (
                sidecar.cmyk().len() + sidecar.spot_state().len(),
                sidecar.spot_names().len(),
            )
        });
        let paints = operators.iter().filter(|operator| is_paint_operator(operator)).count();
        if paints == 0 {
            return Ok(None);
        }
        // Showing text charges its page-sized coverage raster in addition to the paint itself. ~keep
        let surfaces_in_pass = paints + operators.iter().filter(|operator| is_text_show(operator)).count();
        let workspace = self.knockout_resources.knockout(
            pixels,
            pixmap.data().len(),
            sidecar_bytes,
            spot_count,
            paints,
            surfaces_in_pass,
        )?;
        let Some(workspace) = workspace else {
            return self.execute_without_knockout(pixmap, base_transform, operators, doc, page_num, resources);
        };
        let parent_shape = self.knockout_shape.take();
        let mut group = KnockoutGroup::new(self, pixmap);
        let result = self.execute_operators_with_backdrop(
            pixmap,
            base_transform,
            operators,
            doc,
            page_num,
            resources,
            Some(&mut group),
        );
        self.knockout_shape = parent_shape;
        if self.knockout_resources.work_exhausted() {
            group.backdrop.restore(self, pixmap);
            drop(group);
            drop(workspace);
            return self.execute_without_knockout(pixmap, base_transform, operators, doc, page_num, resources);
        }
        if let Some(parent) = self.knockout_shape.as_mut() {
            parent.include_shape(&group.shape);
        }
        group.accumulator.restore(self, pixmap);
        result.map(|_| None)
    }

    fn execute_without_knockout(
        &mut self,
        pixmap: &mut Pixmap,
        transform: Transform,
        operators: &[Operator],
        doc: &PdfDocument,
        page_num: usize,
        resources: &Object,
    ) -> Result<Option<super::knockout_resources::WorkFallback>> {
        let before = self.knockout_resources.degradations();
        let fallback = self.knockout_resources.fallback()?;
        if self.knockout_resources.degradations() != before {
            tracing::warn!(
                page_num,
                "knockout surface work limit reached; using non-knockout compositing"
            );
        }
        self.execute_operators(pixmap, transform, operators, doc, page_num, resources)?;
        Ok(Some(fallback))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectRef;
    use crate::rendering::{RenderOptions, sidecar::CmykSidecar};

    fn document_with_failing_child(knockout: bool) -> PdfDocument {
        let invalid = "not-deflated";
        let content = "1 g 0 0 32 16 re f /Broken Do";
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 32 16] /Contents 4 0 R >>".to_string(),
            "<< /Length 0 >>\nstream\n\nendstream".to_string(),
            format!(
                "<< /Type /XObject /Subtype /Form /Filter /FlateDecode /Length {} >>\nstream\n{invalid}\nendstream",
                invalid.len()
            ),
            format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 32 16] \
                /Group << /S /Transparency /I true /K {knockout} >> \
                /Resources << /XObject << /Broken 5 0 R >> >> /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
        ];
        document_from_objects(&objects)
    }

    fn document_from_objects(objects: &[String]) -> PdfDocument {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(bytes.len());
            bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
        }
        let xref = bytes.len();
        bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
        for offset in &offsets {
            bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        bytes.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                offsets.len() + 1
            )
            .as_bytes(),
        );
        PdfDocument::from_bytes(bytes).expect("synthetic PDF")
    }

    #[test]
    fn failed_isolated_group_restores_parent_lanes_and_discards_its_shape() {
        for knockout in [false, true] {
            let doc = document_with_failing_child(knockout);
            let broken = doc.load_object(ObjectRef::new(5, 0)).expect("child stream");
            assert!(
                broken.decode_stream_data().is_err(),
                "child must exercise the decode error"
            );
            let form = doc.load_object(ObjectRef::new(6, 0)).expect("outer form");
            let dict = form.as_dict().expect("form dictionary");
            let data = form.decode_stream_data().expect("outer content");
            let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
            let mut pixmap = Pixmap::new(32, 16).expect("parent canvas");
            pixmap.fill(tiny_skia::Color::from_rgba8(51, 51, 51, 255));
            let parent_rgb = pixmap.data().to_vec();
            let mut sidecar = CmykSidecar::new(32, 16, vec!["TestInk".to_string()]);
            let (tint, alpha) = sidecar.spot_lane_mut(0).expect("spot lane");
            tint.fill(204);
            alpha.fill(255);
            renderer.cmyk_sidecar = Some(sidecar);
            renderer.knockout_shape = Some(KnockoutShape::new(32, 16));
            let result = renderer.render_form_xobject(
                &mut pixmap,
                dict,
                &data,
                Transform::identity(),
                &doc,
                0,
                dict.get("Resources").expect("form resources"),
            );
            assert!(result.is_err(), "outer group must report its child's failure");
            assert_eq!(pixmap.data(), parent_rgb);
            assert_eq!(
                renderer.cmyk_sidecar_spot_plane(0).expect("preserved lane"),
                vec![204; 32 * 16]
            );
            assert_eq!(
                renderer.knockout_shape.as_ref().expect("parent shape").coverage,
                vec![0; 32 * 16]
            );
        }
    }

    #[test]
    fn text_without_ink_render_keeps_parent_knockout_content() {
        let content = "0.2 g 0 0 32 16 re f BT /F1 12 Tf 8 2 Td 1 g (M) Tj ET";
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 32 16] /Contents 4 0 R \
              /Group << /S /Transparency /K true >> \
              /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>"
                .to_string(),
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
        ];
        let doc = document_from_objects(&objects);
        let expected: Vec<u8> = (0..32 * 16).flat_map(|_| [51, 51, 51, 255]).collect();
        let visible = PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
            .render_page(&doc, 0)
            .expect("visible page");
        assert_ne!(visible.data, expected, "visible control must paint real glyphs");
        let hidden = PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
            .without_text_ink()
            .render_page(&doc, 0)
            .expect("page without glyph ink");
        assert_eq!(hidden.data, expected);
    }
    fn knockout_document(paints: usize, size: u32, page_group: bool) -> PdfDocument {
        let content = "0.2 g 0 0 256 256 re f ".repeat(paints);
        knockout_document_with_content(&content, size, page_group)
    }

    fn knockout_document_with_content(content: &str, size: u32, page_group: bool) -> PdfDocument {
        let group = "/Group << /S /Transparency /K true >>";
        let page_content = if page_group { content } else { "/Fm Do" };
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {size} {size}] /Contents 4 0 R {} /Resources << /XObject << /Fm 5 0 R >> >> >>",
                if page_group { group } else { "" }
            ),
            format!(
                "<< /Length {} >>\nstream\n{page_content}\nendstream",
                page_content.len()
            ),
            format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 {size} {size}] /Group << /S /Transparency /I true /K false >> /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            ),
        ];
        document_from_objects(&objects)
    }

    #[test]
    fn knockout_paints_are_executed_once_as_the_group_grows() {
        for paints in [8, 32, 128] {
            let doc = knockout_document(paints, 16, true);
            let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
            let image = renderer.render_page(&doc, 0).expect("render real knockout PDF");
            assert_eq!(&image.data[..4], &[51, 51, 51, 255]);
            assert_eq!(renderer.paint_operator_calls, paints, "each paint executes once");
        }
    }

    #[test]
    fn isolated_rgb_only_group_does_not_clone_parent_rgba() {
        let doc = knockout_document(1, 16, false);
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
        let image = renderer.render_page(&doc, 0).expect("isolated RGB form");
        assert_eq!(&image.data[..4], &[51, 51, 51, 255]);
        assert!(renderer.cmyk_sidecar.is_none(), "RGB-only control has no sidecar");
        assert_eq!(
            renderer.rgba_snapshot_bytes.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[test]
    fn knockout_scratch_refuses_a_budget_that_only_fits_live_pixels() {
        let doc = knockout_document(1, 512, true);
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
        let result = renderer.render_page_with_budget(
            0,
            &doc,
            Some(super::super::PngRenderBudget {
                max_working_bytes: 512 * 512 * 12 + 256 * 1024,
                minimum_dpi: 72,
            }),
        );
        assert!(
            result.is_err(),
            "new knockout scratch must fit the admitted working budget"
        );
    }
    #[test]
    fn first_knockout_stroke_scratch_must_fit_the_admitted_budget() {
        let budget = || super::super::PngRenderBudget {
            max_working_bytes: 15 * 512 * 512,
            minimum_dpi: 72,
        };
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
        let normal = knockout_document_with_content("0 G 8 w 0 256 m 512 256 l S", 512, false);
        assert!(renderer.render_page_with_budget(0, &normal, Some(budget())).is_ok());
        let knockout = knockout_document_with_content("0 G 8 w 0 256 m 512 256 l S", 512, true);
        let refused = renderer.render_page_with_budget(0, &knockout, Some(budget()));
        assert!(
            refused.is_err(),
            "first stroke allocates an RGBA scratch and alpha coverage"
        );
    }

    #[test]
    fn ordinary_rgb_group_budget_allows_many_small_fills() {
        let body = "0.2 g 0 0 1 1 re f ".repeat(1_040);
        let doc = knockout_document_with_content(&body, 1024, false);
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
        let result = renderer
            .render_page_with_budget(
                0,
                &doc,
                Some(super::super::PngRenderBudget {
                    max_working_bytes: 15 * 1024 * 1024,
                    minimum_dpi: 72,
                }),
            )
            .expect("tiny fills do not rasterize the full surface");
        assert_eq!(renderer.paint_operator_calls, 1_041);
        let last_row = (1023 * 1024) * 4;
        assert_eq!(&result.0.data[last_row..last_row + 4], &[51, 51, 51, 255]);
    }

    fn text_work_document(paints: usize, form: bool) -> PdfDocument {
        let body = format!("0.2 g /Half gs BT /F1 8 Tf 0 8 Td {} ET", "(M) Tj ".repeat(paints));
        let resources = "/Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> /ExtGState << /Half << /ca 0.5 >> /Blend << /BM /Multiply >> >> /XObject << /Fm 5 0 R >>";
        let page = if form { "/Blend gs /Fm Do" } else { body.as_str() };
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 32 16] /Contents 4 0 R /Resources << {resources} >> {} >>",
                if form {
                    ""
                } else {
                    "/Group << /S /Transparency /K true >>"
                }
            ),
            format!("<< /Length {} >>\nstream\n{page}\nendstream", page.len()),
            format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 32 16] /Group << /S /Transparency /I false /K true >> /Length {} >>\nstream\n{body}\nendstream",
                body.len()
            ),
        ];
        document_from_objects(&objects)
    }

    #[test]
    fn knockout_text_and_nonisolated_alpha_pass_have_linear_paint_counts() {
        for form in [false, true] {
            for paints in [8, 32, 128] {
                let doc = text_work_document(paints, form);
                let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
                let image = renderer.render_page(&doc, 0).expect("real glyph knockout render");
                assert!(
                    image.data.as_chunks::<4>().0.iter().any(|pixel| pixel[0] < 240),
                    "visible glyph control must contain ink"
                );
                let expected = if form { 2 * paints + 1 } else { paints };
                assert_eq!(renderer.paint_operator_calls, expected);
            }
        }
    }

    #[test]
    fn knockout_plan_that_cannot_finish_degrades_before_the_pass_and_paints_once() {
        // 13_700 fills of a 256x256 page plan 898M pixels and then charge 898M more while they run,
        // past the 1G limit. The pass used to run for about 1_700 paints before the limit stopped it
        // and every paint ran again without knockout, so the paints executed were more than the page has.
        let paints = 13_700;
        let doc = knockout_document_with_content(&"0.2 g 0 0 1 1 re f ".repeat(paints), 256, true);
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
        // The degradation warns; a subscriber keeps that callsite enabled for tests that read it. ~keep
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::WARN)
            .with_writer(std::io::sink)
            .finish();
        let image = tracing::subscriber::with_default(subscriber, || renderer.render_page(&doc, 0))
            .expect("work-limited group remains renderable");
        let corner = 255 * 256 * 4;
        assert_eq!(&image.data[corner..corner + 4], &[51, 51, 51, 255]);
        assert_eq!(renderer.paint_operator_calls, paints, "each paint executes once");
        assert_eq!(
            renderer.rgba_snapshot_bytes.load(std::sync::atomic::Ordering::Relaxed),
            0,
            "the degradation is decided before any knockout backdrop is captured"
        );
    }

    #[test]
    fn excessive_knockout_surface_work_degrades_without_snapshotting_each_paint() {
        let doc = knockout_document_with_content(&"0.2 g 0 0 1 1 re f ".repeat(20_000), 256, true);
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
        renderer
            .render_page(&doc, 0)
            .expect("work-limited group remains renderable");
        assert_eq!(
            renderer.paint_operator_calls, 20_000,
            "normal fallback executes every paint once"
        );
        assert_eq!(
            renderer.rgba_snapshot_bytes.load(std::sync::atomic::Ordering::Relaxed),
            0,
            "preflight degradation allocates no knockout backdrops"
        );
    }
}
