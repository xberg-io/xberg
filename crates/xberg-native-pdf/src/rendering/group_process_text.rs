//! Text-show geometry and per-side compositing for the page renderer.

use super::knockout_resources::Reservation;
use super::{
    PageRenderer, PaintModulatorContext, ResolvedColors, SidecarPaintModulatorSnapshots, source_for_overprint,
};
use crate::content::GraphicsState;
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::Object;
use crate::rendering::sidecar::CmykSidecar;
use crate::rendering::text_rasterizer::{GlyphPaint, TextRun, glyph_user_inverse};
use tiny_skia::{IntRect, Mask, Path, Pixmap, Transform};

#[derive(Clone, Copy)]
pub(super) struct GroupTextPaint<'a> {
    pub(super) gs: &'a GraphicsState,
    pub(super) doc: &'a PdfDocument,
    pub(super) resources: &'a Object,
    pub(super) clip: Option<&'a Mask>,
    pub(super) transform: Transform,
}

struct TextGlyphPaint {
    visible: GlyphPaint,
    coverage: GlyphPaint,
}

impl PageRenderer {
    pub(super) fn render_text_with_modulators(
        &mut self,
        dest: &mut Pixmap,
        text: TextRun<'_>,
        ctx: GroupTextPaint<'_>,
        page_num: usize,
        base_transform: Transform,
    ) -> Result<f32> {
        let paint = self.text_paint_state(ctx.gs);
        let ctx = GroupTextPaint { gs: &paint, ..ctx };
        let sides = text_sides(ctx.gs);
        let colors = self.pipeline_resolve_text_colors(ctx.doc, ctx.gs);
        if !sides.into_iter().any(|(_, paints)| paints) {
            return self.draw_text(dest, text, ctx, colors.as_ref());
        }

        let mask = self.paint_mask_snapshots(dest, ctx.gs)?;
        let staged = sides.into_iter().any(|(fill, paints)| {
            paints && (self.cmyk_compose_active(ctx.gs, ctx.doc, fill) || source_for_overprint(ctx.gs, fill).is_some())
        });
        let process_group = self.cmyk_sidecar.is_some() && self.knockout_resources.active();
        let advance = if process_group || (staged && sides.into_iter().all(|(_, paints)| paints)) {
            self.draw_composited_glyphs(dest, text, ctx, colors.as_ref())?
        } else if staged {
            self.draw_text_side(dest, text, ctx, colors.as_ref(), sides[0].1)?
        } else {
            let advance = self.draw_text(dest, text, ctx, colors.as_ref())?;
            for (fill, paints) in sides {
                if paints
                    && (self.knockout_shape.is_some() || self.spot_paint_active(ctx.gs, fill))
                    && let Some(coverage) = self.text_side_coverage(text, ctx, fill)?
                {
                    self.record_text_side(dest, ctx.gs, fill, &coverage);
                }
            }
            advance
        };
        if let Some(mask) = mask {
            self.apply_paint_mask(
                dest,
                mask,
                PaintModulatorContext {
                    gs: ctx.gs,
                    doc: ctx.doc,
                    page_num,
                    resources: ctx.resources,
                    base_transform,
                    fill_side: true,
                    knockout_coverage: None,
                },
            )?;
        }
        Ok(advance)
    }

    fn draw_composited_glyphs(
        &mut self,
        dest: &mut Pixmap,
        text: TextRun<'_>,
        ctx: GroupTextPaint<'_>,
        colors: Option<&ResolvedColors>,
    ) -> Result<f32> {
        let _scope = self.knockout_resources.reserve(Some(0))?;
        let mut coverage = Self::coverage_only_gs(ctx.gs);
        coverage.render_mode = ctx.gs.render_mode;
        let paint = TextGlyphPaint {
            visible: GlyphPaint::new(ctx.gs, colors),
            coverage: GlyphPaint::new(&coverage, None),
        };
        // The outline visitor composites into the renderer, so temporarily lend layout its own caches. ~keep
        let rasterizer = std::mem::take(&mut self.text_rasterizer);
        let fonts = std::mem::take(&mut self.fonts);
        let mut status = Ok(());
        let advance = rasterizer.visit_text_glyphs(
            text,
            ctx.transform,
            ctx.gs,
            ctx.doc,
            &fonts,
            &mut |path, glyph, user| {
                if status.is_ok() {
                    status = self.draw_composited_glyph(dest, ctx, &paint, path, (glyph, user));
                }
            },
        );
        self.fonts = fonts;
        self.text_rasterizer = rasterizer;
        status?;
        let advance = advance?;
        if self.knockout_shape.is_some() {
            for (fill, paints) in text_sides(ctx.gs) {
                if paints && let Some(coverage) = self.text_side_coverage(text, ctx, fill)? {
                    self.record_knockout_shape(Some(&coverage), ctx.gs, fill);
                }
            }
        }
        Ok(advance)
    }

    fn draw_composited_glyph(
        &mut self,
        dest: &mut Pixmap,
        ctx: GroupTextPaint<'_>,
        paint: &TextGlyphPaint,
        path: &Path,
        (glyph, user): (Transform, Transform),
    ) -> Result<()> {
        let Some(rect) = glyph_rect(dest, path, glyph, user, ctx.gs) else {
            return Ok(());
        };
        let mut crop = GlyphCrop::new(self, dest, rect, ctx.clip)?;
        let shift = Transform::from_translate(-(rect.x() as f32), -(rect.y() as f32));
        let glyph = shift.pre_concat(glyph);
        let user = shift.pre_concat(user);
        let ctx = GroupTextPaint {
            clip: crop.clip.as_ref(),
            ..ctx
        };

        let parent_sidecar = std::mem::replace(&mut self.cmyk_sidecar, crop.sidecar.take());
        let parent_shape = self.knockout_shape.take();
        let result = self.paint_cropped_glyph(&mut crop.pixmap, ctx, paint, path, (glyph, user));
        crop.sidecar = std::mem::replace(&mut self.cmyk_sidecar, parent_sidecar);
        self.knockout_shape = parent_shape;
        result?;
        crop.commit(dest, self.cmyk_sidecar.as_mut());
        Ok(())
    }

    fn paint_cropped_glyph(
        &mut self,
        dest: &mut Pixmap,
        ctx: GroupTextPaint<'_>,
        paint: &TextGlyphPaint,
        path: &Path,
        (glyph, user): (Transform, Transform),
    ) -> Result<()> {
        let mut scratch = self
            .cmyk_sidecar
            .as_ref()
            .map(|_| {
                Pixmap::new(dest.width(), dest.height())
                    .ok_or_else(|| Error::InvalidOperation("could not allocate glyph coverage".into()))
            })
            .transpose()?;
        for (fill, paints) in text_sides(ctx.gs) {
            if !paints || (!fill && glyph_user_inverse(user).is_none()) {
                continue;
            }
            let coverage = scratch.as_mut().map(|scratch| {
                scratch.data_mut().fill(0);
                paint.coverage.draw_side(scratch, path, glyph, user, ctx.clip, fill);
                Self::extract_alpha_as_coverage(scratch)
            });
            let snapshots = self.text_side_snapshots(dest, ctx, fill);
            paint.visible.draw_side(dest, path, glyph, user, ctx.clip, fill);
            self.apply_text_side(dest, coverage.as_deref(), snapshots, ctx, fill)?;
        }
        Ok(())
    }

    fn text_side_snapshots(
        &self,
        dest: &Pixmap,
        ctx: GroupTextPaint<'_>,
        fill: bool,
    ) -> SidecarPaintModulatorSnapshots {
        SidecarPaintModulatorSnapshots {
            cmyk_compose: self.cmyk_compose_snapshot(dest, ctx.gs, ctx.doc, fill),
            overprint: self.overprint_snapshot(dest, ctx.gs, fill),
            cmyk_sidecar: self.cmyk_sidecar_snapshot(dest, ctx.gs, fill),
            rgb_sidecar: self.cmyk_sidecar_snapshot_for_rgb_paint(dest, ctx.gs, fill),
            smask: None,
        }
    }

    fn apply_text_side(
        &mut self,
        dest: &mut Pixmap,
        coverage: Option<&[u8]>,
        snapshots: SidecarPaintModulatorSnapshots,
        ctx: GroupTextPaint<'_>,
        fill: bool,
    ) -> Result<()> {
        self.apply_sidecar_paint_modulators(
            dest,
            snapshots,
            coverage,
            PaintModulatorContext {
                gs: ctx.gs,
                doc: ctx.doc,
                page_num: 0,
                resources: ctx.resources,
                base_transform: ctx.transform,
                fill_side: fill,
                knockout_coverage: None,
            },
        )
    }

    fn draw_text_side(
        &mut self,
        dest: &mut Pixmap,
        text: TextRun<'_>,
        ctx: GroupTextPaint<'_>,
        colors: Option<&ResolvedColors>,
        fill: bool,
    ) -> Result<f32> {
        let mut side = ctx.gs.clone();
        side.render_mode = u8::from(!fill);
        let coverage = self.text_side_coverage(text, ctx, fill)?;
        let snapshots = self.text_side_snapshots(dest, ctx, fill);
        let advance = self.draw_text(dest, text, GroupTextPaint { gs: &side, ..ctx }, colors)?;
        self.apply_text_side(dest, coverage.as_deref(), snapshots, ctx, fill)?;
        Ok(advance)
    }

    fn record_text_side(&mut self, dest: &Pixmap, gs: &GraphicsState, fill: bool, coverage: &[u8]) {
        self.record_knockout_shape(Some(coverage), gs, fill);
        self.mirror_spot_paint_into_sidecar_with_coverage(dest, &[], Some(coverage), gs, fill);
    }

    fn text_side_coverage(
        &mut self,
        text: TextRun<'_>,
        ctx: GroupTextPaint<'_>,
        fill: bool,
    ) -> Result<Option<Vec<u8>>> {
        let Some((width, height)) = self.coverage_dimensions() else {
            return Ok(None);
        };
        if self.knockout_resources.active() {
            self.knockout_resources.raster(width as usize * height as usize)?;
        }
        let mut scratch = Pixmap::new(width, height)
            .ok_or_else(|| Error::InvalidOperation("could not allocate text coverage".into()))?;
        let mut paint = Self::coverage_only_gs(ctx.gs);
        paint.render_mode = u8::from(!fill);
        self.draw_text(&mut scratch, text, GroupTextPaint { gs: &paint, ..ctx }, None)?;
        Ok(Some(Self::extract_alpha_as_coverage(&scratch)))
    }

    fn draw_text(
        &self,
        dest: &mut Pixmap,
        text: TextRun<'_>,
        ctx: GroupTextPaint<'_>,
        colors: Option<&ResolvedColors>,
    ) -> Result<f32> {
        match text {
            TextRun::String(bytes) => self.text_rasterizer.render_text(
                dest,
                bytes,
                ctx.transform,
                ctx.gs,
                colors,
                ctx.resources,
                ctx.doc,
                ctx.clip,
                &self.fonts,
            ),
            TextRun::Array(array) => self.text_rasterizer.render_tj_array(
                dest,
                array,
                ctx.transform,
                ctx.gs,
                colors,
                ctx.resources,
                ctx.doc,
                ctx.clip,
                &self.fonts,
            ),
        }
    }
}

struct GlyphCrop {
    rect: IntRect,
    pixmap: Pixmap,
    clip: Option<Mask>,
    sidecar: Option<CmykSidecar>,
    _workspace: Reservation,
}

impl GlyphCrop {
    fn new(renderer: &mut PageRenderer, dest: &Pixmap, rect: IntRect, clip: Option<&Mask>) -> Result<Self> {
        let pixels = rect.width() as usize * rect.height() as usize;
        let inks = renderer
            .cmyk_sidecar
            .as_ref()
            .map_or(0, |sidecar| sidecar.spot_names().len());
        // Crop RGBA/clip, process and spot planes, two RGBA snapshots and coverage scratch. ~keep
        let names_bytes = renderer.cmyk_sidecar.as_ref().map_or(Some(0), |sidecar| {
            sidecar.spot_names().iter().try_fold(0usize, |total, name| {
                total
                    .checked_add(name.len())?
                    .checked_add(std::mem::size_of::<String>())
            })
        });
        let bytes = inks
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(22))
            .and_then(|bytes| bytes.checked_mul(pixels)?.checked_add(names_bytes?));
        let workspace = renderer.knockout_resources.reserve(bytes)?;
        renderer.knockout_resources.raster(pixels.saturating_mul(2))?;
        let pixmap = dest
            .clone_rect(rect)
            .ok_or_else(|| Error::InvalidOperation("could not allocate glyph region".into()))?;
        let clip = clip.map(|clip| crop_glyph_clip(clip, rect)).transpose()?;
        let sidecar = renderer.cmyk_sidecar.as_ref().map(|parent| {
            let mut crop = CmykSidecar::new(rect.width(), rect.height(), parent.spot_names().to_vec());
            for (src, dst) in glyph_rows(rect, dest.width()) {
                copy_glyph_row(
                    parent.cmyk(),
                    crop.cmyk_mut(),
                    src * 4,
                    dst * 4,
                    rect.width() as usize * 4,
                );
                for lane in 0..inks * 2 {
                    copy_glyph_row(
                        parent.spot_state(),
                        crop.spot_state_mut(),
                        src + lane * dest.width() as usize * dest.height() as usize,
                        dst + lane * pixels,
                        rect.width() as usize,
                    );
                }
            }
            crop
        });
        Ok(Self {
            rect,
            pixmap,
            clip,
            sidecar,
            _workspace: workspace,
        })
    }

    fn commit(self, dest: &mut Pixmap, sidecar: Option<&mut CmykSidecar>) {
        let row_width = self.rect.width() as usize;
        let width = dest.width();
        for (dst, src) in glyph_rows(self.rect, width) {
            copy_glyph_row(self.pixmap.data(), dest.data_mut(), src * 4, dst * 4, row_width * 4);
        }
        let (Some(source), Some(sidecar)) = (self.sidecar, sidecar) else {
            return;
        };
        let parent_pixels = width as usize * dest.height() as usize;
        let crop_pixels = row_width * self.rect.height() as usize;
        let inks = source.spot_names().len();
        for (dst, src) in glyph_rows(self.rect, width) {
            copy_glyph_row(source.cmyk(), sidecar.cmyk_mut(), src * 4, dst * 4, row_width * 4);
            for lane in 0..inks * 2 {
                copy_glyph_row(
                    source.spot_state(),
                    sidecar.spot_state_mut(),
                    src + lane * crop_pixels,
                    dst + lane * parent_pixels,
                    row_width,
                );
            }
        }
    }
}

fn glyph_rect(dest: &Pixmap, path: &Path, glyph: Transform, user: Transform, gs: &GraphicsState) -> Option<IntRect> {
    let [(_, fill), (_, stroke)] = text_sides(gs);
    let stroke = stroke && glyph_user_inverse(user).is_some();
    if !fill && !stroke {
        return None;
    }
    let [left, top, right, bottom] = crate::rendering::device_bounds(path, glyph)?;
    let stroke_reach = if stroke {
        let scale = (user.sx.abs() + user.kx.abs()).max(user.ky.abs() + user.sy.abs());
        f64::from(gs.line_width.abs()) * f64::from(gs.miter_limit.max(1.0)) * f64::from(scale)
    } else {
        0.0
    };
    let reach = 1.0 + stroke_reach;
    let x0 = (left - reach).floor().clamp(0.0, f64::from(dest.width())) as i32;
    let y0 = (top - reach).floor().clamp(0.0, f64::from(dest.height())) as i32;
    let x1 = (right + reach).ceil().clamp(0.0, f64::from(dest.width())) as i32;
    let y1 = (bottom + reach).ceil().clamp(0.0, f64::from(dest.height())) as i32;
    IntRect::from_ltrb(x0, y0, x1, y1)
}

fn glyph_rows(rect: IntRect, parent_width: u32) -> impl Iterator<Item = (usize, usize)> {
    let first = rect.y() as usize * parent_width as usize + rect.x() as usize;
    (0..rect.height() as usize).map(move |row| (first + row * parent_width as usize, row * rect.width() as usize))
}

fn copy_glyph_row(source: &[u8], dest: &mut [u8], from: usize, to: usize, bytes: usize) {
    dest[to..to + bytes].copy_from_slice(&source[from..from + bytes]);
}

fn crop_glyph_clip(clip: &Mask, rect: IntRect) -> Result<Mask> {
    let mut crop = Mask::new(rect.width(), rect.height())
        .ok_or_else(|| Error::InvalidOperation("could not allocate glyph clip".into()))?;
    for (src, dst) in glyph_rows(rect, clip.width()) {
        copy_glyph_row(clip.data(), crop.data_mut(), src, dst, rect.width() as usize);
    }
    Ok(crop)
}

fn text_sides(gs: &GraphicsState) -> [(bool, bool); 2] {
    [
        (true, matches!(gs.render_mode, 0 | 2 | 4 | 6)),
        (false, matches!(gs.render_mode, 1 | 2 | 5 | 6)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_only_glyph_bounds_do_not_depend_on_line_width() {
        let canvas = Pixmap::new(64, 64).expect("canvas");
        let mut path = tiny_skia::PathBuilder::new();
        path.push_rect(tiny_skia::Rect::from_xywh(12.0, 10.0, 8.0, 14.0).expect("glyph bounds"));
        let path = path.finish().expect("glyph outline");
        let mut gs = GraphicsState {
            line_width: 1_000.0,
            ..Default::default()
        };
        let bounds = |gs: &GraphicsState| {
            glyph_rect(&canvas, &path, Transform::identity(), Transform::identity(), gs).expect("nonempty region")
        };
        let fill = bounds(&gs);
        assert_eq!((fill.width(), fill.height()), (10, 16));
        gs.render_mode = 1;
        assert_eq!(
            bounds(&gs).width(),
            canvas.width(),
            "actual stroke padding remains admitted"
        );
    }

    #[test]
    fn nonfinite_inverse_cannot_request_a_stroke_only_crop() {
        let canvas = Pixmap::new(64, 64).expect("canvas");
        let mut path = tiny_skia::PathBuilder::new();
        path.push_rect(tiny_skia::Rect::from_xywh(12.0, 10.0, 8.0, 14.0).expect("glyph bounds"));
        let path = path.finish().expect("glyph outline");
        let singular = Transform::from_row(0.0, 0.0, 0.0, 1.0, 10.0, 0.0);
        let mut gs = GraphicsState {
            render_mode: 1,
            line_width: 1_000.0,
            ..Default::default()
        };
        assert!(glyph_rect(&canvas, &path, singular, singular, &gs).is_none());
        gs.render_mode = 2;
        let fill = glyph_rect(&canvas, &path, singular, singular, &gs).expect("fill-only device bounds");
        assert_eq!((fill.width(), fill.height()), (2, 16));
    }
}
