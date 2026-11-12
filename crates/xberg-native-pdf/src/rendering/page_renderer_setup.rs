//! Construction, render entry points and inspection methods for the page renderer.

use super::*;

impl PageRenderer {
    /// Create a new page renderer with the specified options.
    pub fn new(options: RenderOptions) -> Self {
        Self {
            options,
            source_image_budget: None,
            reported_source_budget_exceeded: false,
            text_contrast_probe: None,
            path_rasterizer: PathRasterizer::new(),
            text_rasterizer: TextRasterizer::new(),
            text_ink_suppressed: false,
            images_suppressed: false,
            fonts: HashMap::new(),
            color_spaces: HashMap::new(),
            excluded_layers_snapshot: None,
            icc_transform_cache: IccTransformCache::new(),
            smask_depth: 0,
            cmyk_sidecar: None,
            force_cmyk_sidecar: false,
            k_zero_warning_emitted: false,
            type3_depth: 0,
            type3_fill_lock: None,
            knockout_shape: None,
            knockout_resources: knockout_resources::KnockoutResources::default(),
            #[cfg(test)]
            paint_operator_calls: 0,
            #[cfg(test)]
            rgba_snapshot_bytes: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Take ownership of the per-page CMYK + spot-ink sidecar produced
    /// by the most recent [`Self::render_page_with_options`] call.
    /// Leaves the renderer's slot empty so a subsequent render starts
    /// fresh.
    ///
    /// Used by the separation entry point
    /// ([`super::separation_renderer::render_separations`]) to harvest
    /// the populated process + spot lanes after a composite render and
    /// decompose them into per-plate output (ISO 32000-1 §10.5 plates,
    /// §11.7.3 spot lanes, §11.7.4.2 BM split).
    pub(crate) fn take_cmyk_sidecar(&mut self) -> Option<CmykSidecar> {
        self.cmyk_sidecar.take()
    }

    /// Number of qcms transform constructions the per-page cache has
    /// observed since the last `render_page_with_options` call. Exposes
    /// renderer internals for assertions and is not part of the
    /// supported surface. Lets the integration suite assert "1000
    /// same-colour CMYK paints built 1 transform" without racing
    /// concurrent tests that might also trigger `Transform::new_srgb_target`
    /// via the global counter.
    #[doc(hidden)]
    pub fn icc_transform_cache_build_count(&self) -> usize {
        self.icc_transform_cache.build_count()
    }

    /// Total `IccTransformCache::get_or_build` calls (hits + misses)
    /// observed since the last `render_page_with_options` call. Exposes
    /// renderer internals for assertions and is not part of the
    /// supported surface. Distinguishes a properly-hoisted per-paint
    /// lookup from a per-pixel regression: the cache returns a cached
    /// `Arc<Transform>` on every hit so `build_count` stays at 1
    /// either way, but the `content_hash` SipHash over the whole
    /// profile blob runs on every call, hit or miss. A correctly
    /// hoisted hot loop therefore yields lookup_count ≈ paint count;
    /// a per-pixel regression yields lookup_count proportional to
    /// painted pixels.
    #[doc(hidden)]
    pub fn icc_transform_cache_lookup_count(&self) -> usize {
        self.icc_transform_cache.lookup_count()
    }

    /// Number of CMYK→CMYK retarget cache misses observed since the
    /// last `render_page_with_options` call. Exposes renderer internals
    /// for assertions and is not part of the supported surface. Pins
    /// the M2 retarget cache: a page with many DeviceN /Process
    /// /ICCBased N=4 paints under one OutputIntent must build the
    /// retarget transform exactly once per unique `(src_profile,
    /// dst_profile, intent)` tuple, not once per paint.
    #[doc(hidden)]
    pub fn icc_transform_cache_cmyk_retarget_build_count(&self) -> usize {
        self.icc_transform_cache.cmyk_retarget_build_count()
    }

    /// Pixmap dimensions of the per-page compositing sidecar, or
    /// `None` when the sidecar was not allocated for the most recent
    /// `render_page_with_options` call (detection-OFF).
    ///
    /// Exposes renderer internals for assertions and is not part of
    /// the supported surface — gates round-1 spot-ink discovery probes
    /// and round-4 CMYK plane shape probes.
    #[doc(hidden)]
    pub fn cmyk_sidecar_dims(&self) -> Option<(u32, u32)> {
        self.cmyk_sidecar.as_ref().map(CmykSidecar::dims)
    }

    /// Read-only view over the sidecar's packed `(C, M, Y, K)` plane.
    /// `None` when the sidecar is not allocated. Exposes renderer
    /// internals for assertions and is not part of the supported
    /// surface.
    #[doc(hidden)]
    pub fn cmyk_sidecar_cmyk_bytes(&self) -> Option<&[u8]> {
        self.cmyk_sidecar.as_ref().map(CmykSidecar::cmyk)
    }

    /// Ordered list of spot ink names the discovery pre-pass surfaced
    /// for the most recent render (sorted ASCII, deduped, `/All` and
    /// `/None` filtered out per ISO 32000-1 §8.6.6.4). `None` when
    /// the sidecar is not allocated. Exposes renderer internals for
    /// assertions and is not part of the supported surface.
    #[doc(hidden)]
    pub fn cmyk_sidecar_spot_names(&self) -> Option<&[String]> {
        self.cmyk_sidecar.as_ref().map(CmykSidecar::spot_names)
    }

    /// Read-only view over the tint plane for spot ink `index`,
    /// or `None` when the sidecar is not allocated or `index` is
    /// beyond the discovered spot set. Exposes renderer internals for
    /// assertions and is not part of the supported surface.
    #[doc(hidden)]
    pub fn cmyk_sidecar_spot_plane(&self, index: usize) -> Option<&[u8]> {
        self.cmyk_sidecar.as_ref().and_then(|s| s.spot_plane(index))
    }

    /// Render a page to a raster image.
    pub fn render_page(&mut self, doc: &PdfDocument, page_num: usize) -> Result<RenderedImage> {
        self.render_page_with_options(page_num, doc)
    }

    /// Render a page with specific options.
    pub fn render_page_with_options(&mut self, page_num: usize, doc: &PdfDocument) -> Result<RenderedImage> {
        self.render_page_with_budget(page_num, doc, None)
            .map(|(image, _)| image)
    }

    pub(in crate::rendering) fn without_text_ink(mut self) -> Self {
        self.text_ink_suppressed = true;
        self
    }

    pub(in crate::rendering) fn without_images(mut self) -> Self {
        self.images_suppressed = true;
        self
    }
}
