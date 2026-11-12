use super::PageRenderer;
use crate::content::GraphicsState;
use crate::content::operators::Operator;
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::rendering::sidecar::CmykSidecar;
use crate::rendering::sidecar::{BlendModeClass, SpotBlendDispatch, blend_subtractive_tint};
use tiny_skia::{Mask, Pixmap, PixmapPaint, Transform};

#[derive(Clone, Copy)]
pub(super) struct FormInvocation<'a> {
    pub(super) gs: &'a GraphicsState,
    pub(super) clip: Option<&'a Mask>,
    pub(super) mask_transform: Transform,
    pub(super) mask_resources: &'a crate::object::Object,
}

impl FormInvocation<'_> {
    fn scale(self, pixel: usize) -> f32 {
        self.gs.fill_alpha.clamp(0.0, 1.0) * self.clip.map_or(1.0, |clip| f32::from(clip.data()[pixel]) / 255.0)
    }
}

pub(super) struct GroupSidecar {
    pub(super) cmyk: Vec<u8>,
    pub(super) spots: Vec<u8>,
}

impl GroupSidecar {
    pub(super) fn capture(renderer: &PageRenderer) -> Option<Self> {
        renderer.cmyk_sidecar.as_ref().map(|sidecar| Self {
            cmyk: sidecar.cmyk().to_vec(),
            spots: sidecar.spot_state().to_vec(),
        })
    }

    pub(super) fn clear(&self, renderer: &mut PageRenderer) {
        if let Some(sidecar) = renderer.cmyk_sidecar.as_mut() {
            sidecar.cmyk_mut().fill(0);
            sidecar.spot_state_mut().fill(0);
        }
    }

    pub(super) fn restore(&self, renderer: &mut PageRenderer) {
        if let Some(sidecar) = renderer.cmyk_sidecar.as_mut() {
            sidecar.restore_cmyk(&self.cmyk);
            sidecar.restore_spot_state(&self.spots);
        }
    }

    pub(super) fn composite(
        &self,
        renderer: &mut PageRenderer,
        parent: &Pixmap,
        group: &Pixmap,
        invocation: Option<FormInvocation<'_>>,
        isolated: bool,
    ) {
        let Some(sidecar) = renderer.cmyk_sidecar.as_mut() else {
            return;
        };
        let mode = invocation.map_or("Normal", |context| context.gs.blend_mode.as_str());
        composite_process(sidecar.cmyk_mut(), &self.cmyk, parent, group, invocation, isolated);
        let pixels = group.width() as usize * group.height() as usize;
        let lanes = sidecar.spot_names().len();
        let spot_mode = if BlendModeClass::from_name(mode).spot_dispatch() == SpotBlendDispatch::UseRequested {
            mode
        } else {
            "Normal"
        };
        for lane in 0..lanes {
            let (tints, alphas) = sidecar.spot_lane_mut(lane).expect("allocated spot lane");
            let parent_tints = &self.spots[lane * pixels..(lane + 1) * pixels];
            let parent_alphas = &self.spots[(lanes + lane) * pixels..(lanes + lane + 1) * pixels];
            composite_spot(
                (tints, alphas),
                (parent_tints, parent_alphas),
                invocation,
                isolated,
                spot_mode,
            );
        }
    }
}

fn composite_process(
    destination: &mut [u8],
    parent: &[u8],
    parent_rgba: &Pixmap,
    group: &Pixmap,
    invocation: Option<FormInvocation<'_>>,
    isolated: bool,
) {
    let mode = invocation.map_or("Normal", |context| context.gs.blend_mode.as_str());
    for (pixel, source) in destination.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let scale = invocation.map_or(1.0, |context| context.scale(pixel));
        let background = &parent[pixel * 4..pixel * 4 + 4];
        if !isolated {
            interpolate(source, background, scale);
            continue;
        }
        let alpha = f32::from(group.data()[pixel * 4 + 3]) / 255.0;
        let backdrop_alpha = f32::from(parent_rgba.data()[pixel * 4 + 3]) / 255.0;
        let straight = std::array::from_fn(|index| {
            if alpha == 0.0 {
                0.0
            } else {
                f32::from(source[index]) / 255.0 / alpha
            }
        });
        let blended = process_blend(mode, background, backdrop_alpha, straight);
        for index in 0..4 {
            source[index] =
                byte(alpha * scale * blended[index] + (1.0 - alpha * scale) * f32::from(background[index]) / 255.0);
        }
    }
}

fn interpolate(destination: &mut [u8], parent: &[u8], scale: f32) {
    for (destination, parent) in destination.iter_mut().zip(parent) {
        *destination = (f32::from(*destination) * scale + f32::from(*parent) * (1.0 - scale)).round() as u8;
    }
}

fn byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub(super) fn process_blend(mode: &str, parent: &[u8], alpha: f32, source: [f32; 4]) -> [f32; 4] {
    let Some(nonseparable) = crate::rendering::pdf_blend_mode_is_nonseparable(mode) else {
        return std::array::from_fn(|index| {
            blend_subtractive_tint(mode, f32::from(parent[index]) / 255.0, alpha, source[index])
        });
    };
    if alpha == 0.0 {
        return source;
    };
    let mut background = [
        byte(1.0 - f32::from(parent[0]) / 255.0 / alpha),
        byte(1.0 - f32::from(parent[1]) / 255.0 / alpha),
        byte(1.0 - f32::from(parent[2]) / 255.0 / alpha),
        255,
    ];
    let foreground = [byte(1.0 - source[0]), byte(1.0 - source[1]), byte(1.0 - source[2]), 255];
    crate::rendering::blend_nonsep::compose_in_place(&mut background, &foreground, nonseparable);
    std::array::from_fn(|index| {
        let blended = if index < 3 {
            1.0 - f32::from(background[index]) / 255.0
        } else if mode == "Luminosity" {
            source[3]
        } else {
            f32::from(parent[3]) / 255.0 / alpha
        };
        (1.0 - alpha) * source[index] + alpha * blended
    })
}

fn composite_spot(
    source: (&mut [u8], &mut [u8]),
    parent: (&[u8], &[u8]),
    invocation: Option<FormInvocation<'_>>,
    isolated: bool,
    mode: &str,
) {
    let (tints, alphas) = source;
    let (parent_tints, parent_alphas) = parent;
    for pixel in 0..tints.len() {
        let scale = invocation.map_or(1.0, |context| context.scale(pixel));
        if !isolated {
            tints[pixel] =
                byte((f32::from(tints[pixel]) * scale + f32::from(parent_tints[pixel]) * (1.0 - scale)) / 255.0);
            alphas[pixel] =
                byte((f32::from(alphas[pixel]) * scale + f32::from(parent_alphas[pixel]) * (1.0 - scale)) / 255.0);
            continue;
        }
        let alpha = f32::from(alphas[pixel]) / 255.0;
        let parent_alpha = f32::from(parent_alphas[pixel]) / 255.0;
        let tint = if alpha == 0.0 {
            0.0
        } else {
            f32::from(tints[pixel]) / 255.0 / alpha
        };
        let parent_tint = f32::from(parent_tints[pixel]) / 255.0;
        let blended = blend_subtractive_tint(mode, parent_tint, parent_alpha, tint);
        tints[pixel] = byte(alpha * scale * blended + (1.0 - alpha * scale) * parent_tint);
        alphas[pixel] = byte(alpha * scale + (1.0 - alpha * scale) * parent_alpha);
    }
}

pub(super) fn composite_rgb(
    parent: &mut Pixmap,
    group: &Pixmap,
    invocation: Option<FormInvocation<'_>>,
    isolated: bool,
    resources: &mut super::knockout_resources::KnockoutResources,
) -> Result<()> {
    if !isolated {
        for (pixel, destination) in parent.data_mut().as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let scale = invocation.map_or(1.0, |context| context.scale(pixel));
            let source = &group.data()[pixel * 4..pixel * 4 + 4];
            for index in 0..4 {
                destination[index] =
                    (f32::from(source[index]) * scale + f32::from(destination[index]) * (1.0 - scale)).round() as u8;
            }
        }
        return Ok(());
    }
    let mode = invocation.map_or("Normal", |context| context.gs.blend_mode.as_str());
    let mut paint = PixmapPaint::default();
    paint.opacity = invocation.map_or(1.0, |context| context.gs.fill_alpha.clamp(0.0, 1.0));
    paint.blend_mode = crate::rendering::pdf_blend_mode_to_skia(mode);
    let clip = invocation.and_then(|context| context.clip);
    if let Some(nonseparable) = crate::rendering::pdf_blend_mode_is_nonseparable(mode) {
        let _workspace = resources.reserve(Some(group.data().len()))?;
        let mut source = Pixmap::new(group.width(), group.height())
            .ok_or_else(|| Error::InvalidPdf("group blend allocation failed".into()))?;
        source.draw_pixmap(0, 0, group.as_ref(), &paint, Transform::identity(), clip);
        crate::rendering::blend_nonsep::compose_in_place(parent.data_mut(), source.data(), nonseparable);
    } else {
        parent.draw_pixmap(0, 0, group.as_ref(), &paint, Transform::identity(), clip);
    }
    Ok(())
}

fn recover_byte(result: u8, parent: u8, alpha: u8) -> u8 {
    (f32::from(result) - f32::from(255 - alpha) * f32::from(parent) / 255.0)
        .round()
        .clamp(0.0, f32::from(alpha)) as u8
}

fn recover_rgba(parent: &Pixmap, group: &mut Pixmap, alpha: &Pixmap) {
    for (pixel, source) in group.data_mut().as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let source_alpha = alpha.data()[pixel * 4 + 3];
        for channel in 0..3 {
            source[channel] = recover_byte(source[channel], parent.data()[pixel * 4 + channel], source_alpha);
        }
        source[3] = source_alpha;
    }
}

impl GroupSidecar {
    fn recover_source(
        &self,
        renderer: &mut PageRenderer,
        alpha: &Pixmap,
        alpha_sidecar: Option<&CmykSidecar>,
    ) -> Result<()> {
        let Some(sidecar) = renderer.cmyk_sidecar.as_mut() else {
            return Ok(());
        };
        for (index, value) in sidecar.cmyk_mut().iter_mut().enumerate() {
            *value = recover_byte(*value, self.cmyk[index], alpha.data()[index / 4 * 4 + 3]);
        }
        let pixels = alpha.data().len() / 4;
        let lanes = sidecar.spot_names().len();
        let alpha_sidecar = alpha_sidecar.ok_or_else(|| Error::InvalidOperation("missing group ink alpha".into()))?;
        for lane in 0..lanes {
            let source_alpha = &alpha_sidecar.spot_state()[(lanes + lane) * pixels..(lanes + lane + 1) * pixels];
            let parent_tint = &self.spots[lane * pixels..(lane + 1) * pixels];
            let (tints, alphas) = sidecar.spot_lane_mut(lane).expect("allocated group ink");
            for pixel in 0..pixels {
                tints[pixel] = recover_byte(tints[pixel], parent_tint[pixel], source_alpha[pixel]);
                alphas[pixel] = source_alpha[pixel];
            }
        }
        Ok(())
    }
}

pub(super) struct GroupReplay<'a> {
    pub(super) operators: &'a [Operator],
    pub(super) transform: Transform,
    pub(super) doc: &'a PdfDocument,
    pub(super) page_num: usize,
    pub(super) resources: &'a crate::object::Object,
    pub(super) knockout: bool,
}

pub(super) fn sidecar_replay_bytes(renderer: &PageRenderer) -> Option<usize> {
    let Some(sidecar) = renderer.cmyk_sidecar.as_ref() else {
        return Some(0);
    };
    let buffers = sidecar.cmyk().len().checked_add(sidecar.spot_state().len())?;
    sidecar.spot_names().iter().try_fold(buffers, |total, name| {
        total
            .checked_add(name.capacity())?
            .checked_add(std::mem::size_of::<String>())
    })
}

impl PageRenderer {
    pub(super) fn recover_nonisolated_source(
        &mut self,
        parent: &Pixmap,
        group: &mut Pixmap,
        backdrop: Option<&GroupSidecar>,
        replay: GroupReplay<'_>,
    ) -> Result<()> {
        let bytes = sidecar_replay_bytes(self).and_then(|bytes| bytes.checked_add(group.data().len()));
        let _workspace = self.knockout_resources.reserve(bytes)?;
        let mut alpha = Pixmap::new(group.width(), group.height())
            .ok_or_else(|| Error::InvalidPdf("group alpha allocation failed".into()))?;
        let before_replay = self.knockout_resources.degradations();
        let (mut alpha_sidecar, work_fallback) = self.render_source_alpha(&mut alpha, &replay)?;
        let alpha_already_degraded = work_fallback.is_some();
        if alpha_already_degraded || self.knockout_resources.degradations() != before_replay {
            let _fallback = match work_fallback {
                Some(fallback) => fallback,
                None => self.knockout_resources.fallback()?,
            };
            self.rerender_group_without_knockout(parent, group, backdrop, &replay)?;
            if !alpha_already_degraded {
                // A descendant can have degraded after earlier alpha paints. Replay the entire
                // source under the same ordinary compositing mode on both surfaces. ~keep
                drop(alpha_sidecar.take());
                alpha.data_mut().fill(0);
                let ordinary = GroupReplay {
                    knockout: false,
                    ..replay
                };
                alpha_sidecar = self.render_source_alpha(&mut alpha, &ordinary)?.0;
            }
        }
        recover_rgba(parent, group, &alpha);
        if let Some(backdrop) = backdrop {
            backdrop.recover_source(self, &alpha, alpha_sidecar.as_ref())?;
        }
        Ok(())
    }

    fn render_source_alpha(
        &mut self,
        alpha: &mut Pixmap,
        replay: &GroupReplay<'_>,
    ) -> Result<(Option<CmykSidecar>, Option<super::knockout_resources::WorkFallback>)> {
        let saved_sidecar = self.cmyk_sidecar.take();
        let saved_shape = self.knockout_shape.take();
        self.cmyk_sidecar = saved_sidecar
            .as_ref()
            .map(|sidecar| CmykSidecar::new(alpha.width(), alpha.height(), sidecar.spot_names().to_vec()));
        let rendered = if replay.knockout {
            self.execute_knockout_group(
                alpha,
                replay.transform,
                replay.operators,
                replay.doc,
                replay.page_num,
                replay.resources,
                None,
            )
        } else {
            self.execute_operators(
                alpha,
                replay.transform,
                replay.operators,
                replay.doc,
                replay.page_num,
                replay.resources,
            )
            .map(|_| None)
        };
        let alpha_sidecar = self.cmyk_sidecar.take();
        self.cmyk_sidecar = saved_sidecar;
        self.knockout_shape = saved_shape;
        Ok((alpha_sidecar, rendered?))
    }

    fn rerender_group_without_knockout(
        &mut self,
        parent: &Pixmap,
        group: &mut Pixmap,
        backdrop: Option<&GroupSidecar>,
        replay: &GroupReplay<'_>,
    ) -> Result<()> {
        group.data_mut().copy_from_slice(parent.data());
        if let Some(backdrop) = backdrop {
            backdrop.restore(self);
        }
        let shape = self.knockout_shape.take();
        let result = self.execute_operators(
            group,
            replay.transform,
            replay.operators,
            replay.doc,
            replay.page_num,
            replay.resources,
        );
        self.knockout_shape = shape;
        result
    }
}
