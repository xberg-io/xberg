use super::{PageRenderer, is_paint_operator};
use crate::content::GraphicsState;
use crate::content::operators::Operator;
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::Object;
use std::collections::HashMap;
use tiny_skia::{Pixmap, Transform};

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

    pub(super) fn clear_sidecar(&self, renderer: &mut PageRenderer) {
        if let Some(sidecar) = renderer.cmyk_sidecar.as_mut() {
            if let Some(cmyk) = &self.cmyk {
                sidecar.restore_cmyk(&vec![0; cmyk.len()]);
            }
            if let Some(spots) = &self.spots {
                sidecar.restore_spot_state(&vec![0; spots.len()]);
            }
        }
    }

    pub(super) fn composite_sidecar(&self, renderer: &mut PageRenderer, group: &Pixmap) {
        let Some(sidecar) = renderer.cmyk_sidecar.as_mut() else {
            return;
        };
        let alpha: Vec<u8> = group.data().as_chunks::<4>().0.iter().map(|pixel| pixel[3]).collect();
        if let Some(parent) = &self.cmyk {
            let mut result = sidecar.cmyk().to_vec();
            composite_lanes(&mut result, parent, &alpha, true);
            sidecar.restore_cmyk(&result);
        }
        if let Some(parent) = &self.spots {
            let mut result = sidecar.spot_state().to_vec();
            let pixels = alpha.len();
            let lanes = sidecar.spot_names().len();
            for plane in 0..result.len() / pixels {
                let range = plane * pixels..(plane + 1) * pixels;
                let alpha_start = (lanes + plane % lanes) * pixels;
                let lane_alpha = &sidecar.spot_state()[alpha_start..alpha_start + pixels];
                composite_lanes(&mut result[range.clone()], &parent[range], lane_alpha, false);
            }
            sidecar.restore_spot_state(&result);
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

fn composite_lanes(result: &mut [u8], parent: &[u8], alpha: &[u8], interleaved: bool) {
    for (index, (result, parent)) in result.iter_mut().zip(parent).enumerate() {
        let pixel = if interleaved { index / 4 } else { index % alpha.len() };
        let contribution = (u32::from(*parent) * u32::from(255 - alpha[pixel]) + 127) / 255;
        *result = (u32::from(*result) + contribution).min(255) as u8;
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
    ) -> Result<()> {
        let backdrop = KnockoutBackdrop::capture(self, pixmap);
        let mut accumulator = KnockoutBackdrop::capture(self, pixmap);
        let parent_shape = self.knockout_shape.take();
        let mut group_shape = KnockoutShape::new(pixmap.width(), pixmap.height());
        let result = (|| {
            for (index, _) in operators.iter().enumerate().filter(|(_, op)| is_paint_operator(op)) {
                let mut scratch = Pixmap::new(pixmap.width(), pixmap.height())
                    .ok_or_else(|| Error::InvalidPdf("knockout scratch pixmap alloc failed".into()))?;
                backdrop.restore(self, &mut scratch);
                // Execute earlier paints too: their path consumption, clipping and text advances
                // must survive. Only the selected paint sees the restored initial backdrop. ~keep
                self.execute_operators_with_backdrop(
                    &mut scratch,
                    base_transform,
                    &operators[..=index],
                    doc,
                    page_num,
                    resources,
                    Some(&backdrop),
                )?;
                if let Some(shape) = self.knockout_shape.take() {
                    accumulator.merge(self, &scratch, &backdrop, &shape);
                    group_shape.include_shape(&shape);
                }
            }
            Ok(())
        })();
        self.knockout_shape = parent_shape;
        if let Some(parent) = self.knockout_shape.as_mut() {
            parent.include_shape(&group_shape);
        }
        accumulator.restore(self, pixmap);
        result
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
}
