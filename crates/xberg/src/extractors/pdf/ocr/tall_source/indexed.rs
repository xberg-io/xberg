//! Source and visible-frame evidence for a single painted indexed RGB raster.

use super::ordered::{BoxPx, SourceColumn, SourceLine};
use super::{FIXED_WORK_BYTES, MAX_SOURCE_PIXELS, MAX_TILE_SIDE, TILE_BYTES_PER_PIXEL};
use crate::extractors::pdf::ocr::hidden_layer::{
    StreamFilters, decode_exact_flate, decode_probe_stream, page_frame_and_contents, stream_filters,
};
use image::ImageEncoder;
use std::ops::Range;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::content::Operator;
use xberg_native_pdf::content::parser::visit_complete_content_stream;
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::object::Object;

const PALETTE_BYTES: usize = 256 * 3;

pub(super) struct IndexedSource {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) dpi: f64,
    pub(super) frame: Rect,
    pub(super) painted_height: f64,
    pub(super) painted_width: f64,
    indices: Vec<u8>,
    palette: [u8; PALETTE_BYTES],
    available: usize,
}

fn image_placement(paint: &[u8], frame: Rect) -> Option<(String, f64, f64)> {
    let mut name = None;
    let mut dimensions = None;
    let mut count = 0;
    let complete = visit_complete_content_stream(paint, 4, |operator| {
        match (count, operator) {
            (0, Operator::SaveState) | (3, Operator::RestoreState) => {}
            (1, Operator::Cm { a, b, c, d, e, f })
                if b == 0.0
                    && c == 0.0
                    && e == frame.x
                    && f == frame.y
                    && a.is_finite()
                    && d.is_finite()
                    && a > 0.0
                    && d > 0.0 =>
            {
                dimensions = Some((f64::from(a), f64::from(d)));
            }
            (2, Operator::Do { name: image }) => name = Some(image),
            _ => return false,
        }
        count += 1;
        true
    })
    .ok()?;
    if !complete || count != 4 {
        return None;
    }
    let (w, h) = dimensions?;
    Some((name?, w, h))
}

fn placement_fits(frame: Rect, samples: (u32, u32), painted: (f64, f64)) -> Option<f64> {
    let (width, height) = (f64::from(frame.width), f64::from(frame.height));
    let (w, h) = painted;
    let (cols, rows) = (f64::from(samples.0), f64::from(samples.1));
    let dpi = (72.0 * cols / w).round();
    // The admitted thin edge differences cannot reach the second source sample.
    // The first/last sample edges must also be white, checked after exact decoding.
    (frame.x == 0.0
        && frame.y == 0.0
        && width.is_finite()
        && height.is_finite()
        && (w - width).abs() < w / cols
        && (h - height).abs() < h / rows
        && dpi >= 1.0
        && dpi == (72.0 * rows / h).round())
    .then_some(dpi)
}

fn palette(source: &PdfDocument, resources: &Object, color: &Object, available: usize) -> Option<[u8; PALETTE_BYTES]> {
    let aliases = resources
        .as_dict()?
        .get("ColorSpace")
        .map(|value| source.resolve_object(value))
        .transpose()
        .ok()?;
    if let Some(aliases) = aliases.as_ref()
        && aliases.as_dict()?.contains_key("DefaultRGB")
    {
        return None;
    }
    let color = if let Some(name) = color.as_name() {
        source.resolve_object(aliases.as_ref()?.as_dict()?.get(name)?).ok()?
    } else {
        source.resolve_object(color).ok()?
    };
    let [kind, base, max, lookup] = color.as_array()?.as_slice() else {
        return None;
    };
    if kind.as_name()? != "Indexed" || base.as_name()? != "DeviceRGB" || max.as_integer()? != 255 {
        return None;
    }
    match source.resolve_object(lookup).ok()? {
        Object::String(data) => data.as_slice().try_into().ok(),
        Object::Stream { dict, data } if !dict.contains_key("DecodeParms") => {
            let filters = stream_filters(dict.get("Filter"))?;
            decode_probe_stream(&data, filters, PALETTE_BYTES, available)?
                .as_slice()
                .try_into()
                .ok()
        }
        _ => None,
    }
}

pub(super) fn is_indexed_rgb_scan(attempt: &super::Attempt<'_>) -> bool {
    indexed_image(attempt, None)
        .and_then(|image| image.admit_palette(attempt.document))
        .is_some()
}

struct IndexedImage {
    frame: Rect,
    painted_height: f64,
    painted_width: f64,
    available: usize,
    dict: std::collections::HashMap<String, Object>,
    data: bytes::Bytes,
    resources: Object,
    pixels: usize,
    width: u32,
    height: u32,
    dpi: f64,
}

fn indexed_image(attempt: &super::Attempt<'_>, selected_width: Option<u32>) -> Option<IndexedImage> {
    let (raster_w, raster_h, raster_png) = attempt.raster?;
    let raster = (raster_w as usize)
        .checked_mul(raster_h as usize)?
        .checked_mul(4)?
        .checked_add(raster_png)?;
    let available = attempt.byte_limit.checked_sub(raster)?.checked_sub(FIXED_WORK_BYTES)?;
    let (frame, contents) = page_frame_and_contents(attempt.document, attempt.page)?;
    let Object::Stream { dict, data } = contents else {
        return None;
    };
    if dict.contains_key("DecodeParms") {
        return None;
    }
    let paint = decode_probe_stream(&data, stream_filters(dict.get("Filter"))?, 4096, available)?;
    let (name, painted_w, painted_height) = image_placement(&paint, frame)?;
    let resources = attempt.document.get_page_resources(attempt.page).ok()?;
    let xobjects = attempt
        .document
        .resolve_object(resources.as_dict()?.get("XObject")?)
        .ok()?;
    let Object::Stream { dict, data } = attempt.document.resolve_object(xobjects.as_dict()?.get(&name)?).ok()? else {
        return None;
    };
    if dict.get("Subtype")?.as_name()? != "Image"
        || dict.get("BitsPerComponent")?.as_integer()? != 8
        || !matches!(stream_filters(dict.get("Filter")), Some(StreamFilters::Flate))
        || [
            "Mask",
            "SMask",
            "SMaskInData",
            "Decode",
            "DecodeParms",
            "ImageMask",
            "Alternates",
            "OC",
        ]
        .iter()
        .any(|key| dict.contains_key(*key))
    {
        return None;
    }
    let width = u32::try_from(dict.get("Width")?.as_integer()?).ok()?;
    let height = u32::try_from(dict.get("Height")?.as_integer()?).ok()?;
    let pixels = (width as usize).checked_mul(height as usize)?;
    if width > MAX_TILE_SIDE
        || height <= MAX_TILE_SIDE
        || pixels > MAX_SOURCE_PIXELS
        || width <= selected_width.unwrap_or(raster_w)
    {
        return None;
    }
    let dpi = placement_fits(frame, (width, height), (painted_w, painted_height))?;
    let available = available.checked_sub(data.len())?.checked_sub(pixels.checked_add(1)?)?;
    Some(IndexedImage {
        frame,
        painted_height,
        painted_width: painted_w,
        available,
        dict,
        data,
        resources,
        pixels,
        width,
        height,
        dpi,
    })
}

impl IndexedImage {
    fn admit_palette(&self, document: &PdfDocument) -> Option<[u8; PALETTE_BYTES]> {
        palette(document, &self.resources, self.dict.get("ColorSpace")?, self.available)
    }
}

impl IndexedSource {
    pub(super) fn load(attempt: &super::Attempt<'_>) -> Option<Self> {
        Self::load_with_selected_width(attempt, None)
    }

    pub(super) fn load_with_selected_width(attempt: &super::Attempt<'_>, selected_width: Option<u32>) -> Option<Self> {
        let image = indexed_image(attempt, selected_width)?;
        let palette = image.admit_palette(attempt.document)?;
        let IndexedImage {
            frame,
            painted_height,
            painted_width,
            available,
            data,
            pixels,
            width,
            height,
            dpi,
            ..
        } = image;
        let indices = decode_exact_flate(&data, pixels)?;
        let available = available.checked_sub(indices.capacity().saturating_sub(pixels + 1))?;
        let source = Self {
            width,
            height,
            dpi,
            frame,
            painted_height,
            painted_width,
            indices,
            palette,
            available,
        };
        source.white_edges().then_some(source)
    }

    fn white(&self, sample: u8) -> bool {
        let i = usize::from(sample) * 3;
        self.palette[i..i + 3] == [255; 3]
    }

    fn white_edges(&self) -> bool {
        let width = self.width as usize;
        let last = self.indices.len() - width;
        self.indices[..width]
            .iter()
            .chain(&self.indices[last..])
            .all(|sample| self.white(*sample))
            && self
                .indices
                .chunks_exact(width)
                .all(|row| self.white(row[0]) && self.white(row[width - 1]))
    }

    pub(super) fn source_y(&self, y: u32, raster_h: u32) -> Option<f64> {
        if raster_h == 0 || y > raster_h {
            return None;
        }
        let frame_height = f64::from(self.frame.height);
        Some(
            (f64::from(y) / f64::from(raster_h) * frame_height - (frame_height - self.painted_height))
                / self.painted_height
                * f64::from(self.height),
        )
    }

    pub(super) fn backend_limit(&self, png_capacity: usize) -> Option<usize> {
        self.available.checked_sub(png_capacity)
    }

    pub(super) fn max_tile_rows(&self) -> usize {
        (self.available / TILE_BYTES_PER_PIXEL / self.width as usize).min(MAX_TILE_SIDE as usize)
    }

    pub(super) fn rows(&self, rows: Range<u32>) -> impl Iterator<Item = &[u8]> {
        let width = self.width as usize;
        self.indices[rows.start as usize * width..rows.end as usize * width].chunks_exact(width)
    }

    pub(super) fn white_row(&self, row: &[u8]) -> bool {
        row.iter().all(|&sample| self.white(sample))
    }

    pub(super) fn seams(&self, cancel: impl Fn() -> bool) -> Option<Vec<u32>> {
        const MAX_SEAMS: usize = 256;
        let mut seams = Vec::new();
        let mut start = None;
        for (index, row) in self.rows(0..self.height).enumerate() {
            if cancel() {
                return None;
            }
            let index = index as u32;
            if self.white_row(row) {
                start.get_or_insert(index);
            } else if let Some(begin) = start.take()
                && index - begin >= super::MIN_BLANK_ROWS
            {
                if seams.len() == MAX_SEAMS {
                    return None;
                }
                seams.push(begin + (index - begin) / 2);
            }
        }
        Some(seams)
    }

    pub(super) fn right_column(&self, rows: Range<u32>) -> Option<SourceColumn> {
        let mut columns = vec![true; self.width as usize];
        for row in self.rows(rows.clone()) {
            for (white, &sample) in columns.iter_mut().zip(row) {
                *white &= self.white(sample);
            }
        }
        // Only the widest internal separators can certify a column. Trying narrower
        // word spaces would turn this bounded scan into an arbitrary partition search.
        let mut best = None::<Range<usize>>;
        let mut start = None;
        for (x, &white) in columns.iter().enumerate() {
            if white {
                start.get_or_insert(x);
            } else if let Some(from) = start.take()
                && from > 0
                && best.as_ref().is_none_or(|b| x - from >= b.len())
            {
                best = Some(from..x);
            }
        }
        let separator = best?;
        Some(SourceColumn {
            lines: self.distinct_hanging_prefixes(rows, separator.end)?,
            separator: separator.start as u32..separator.end as u32,
        })
    }

    fn distinct_hanging_prefixes(&self, rows: Range<u32>, first_x: usize) -> Option<Vec<SourceLine>> {
        let bands = ink_bands(self.rows(rows.clone()), first_x, |sample| self.white(sample));
        let marker = bands.iter().map(|(_, x)| *x).min()?;
        let body = bands.iter().filter_map(|(_, x)| (*x > marker).then_some(*x)).min()?;
        let mut heads = Vec::new();
        let mut lines = Vec::new();
        let mut after_marker = false;
        for (band, x) in bands {
            if x == marker {
                if after_marker {
                    return None;
                }
                heads.push(rows.start + band.start..rows.start + band.end);
                after_marker = true;
            } else {
                if heads.is_empty() {
                    return None;
                }
                after_marker = false;
            }
            let prefix = if x == marker {
                Some(self.prefix_bounds(rows.start, &band, marker..body)?)
            } else {
                None
            };
            lines.push(SourceLine { rows: band, prefix });
        }
        if after_marker || heads.len() < super::ordered::MIN_ORDERED_ENTRIES {
            return None;
        }
        (!heads.windows(2).any(|pair| {
            pair[0].len() == pair[1].len()
                && self
                    .rows(pair[0].clone())
                    .zip(self.rows(pair[1].clone()))
                    .all(|(a, b)| {
                        a[marker..body]
                            .iter()
                            .zip(&b[marker..body])
                            .all(|(&x, &y)| self.white(x) == self.white(y))
                    })
        }))
        .then_some(lines)
    }

    fn prefix_bounds(&self, tile_start: u32, band: &Range<u32>, columns: Range<usize>) -> Option<BoxPx> {
        let mut x0 = usize::MAX;
        let mut x1 = 0;
        let mut y0 = u32::MAX;
        let mut y1 = 0;
        for (i, row) in self.rows(tile_start + band.start..tile_start + band.end).enumerate() {
            let pixels = &row[columns.clone()];
            if let (Some(left), Some(right)) = (
                pixels.iter().position(|&p| !self.white(p)),
                pixels.iter().rposition(|&p| !self.white(p)),
            ) {
                x0 = x0.min(columns.start + left);
                x1 = x1.max(columns.start + right + 1);
                let y = band.start + i as u32;
                y0 = y0.min(y);
                y1 = y + 1;
            }
        }
        (x0 < x1 && y0 < y1).then_some(BoxPx {
            x0: x0 as u32,
            y0,
            x1: x1 as u32,
            y1,
        })
    }

    pub(super) fn native_bounds(
        &self,
        raster: (u32, u32),
        bbox: [u32; 4],
    ) -> Option<super::region_reconciliation::RegionBox> {
        let [left, top, right, bottom] = bbox;
        if left >= right || top >= bottom || right > raster.0 || bottom > raster.1 {
            return None;
        }
        let x = |value| {
            f64::from(value) / f64::from(raster.0) * f64::from(self.frame.width) / self.painted_width
                * f64::from(self.width)
        };
        let coordinates = [
            x(left).floor(),
            self.source_y(top, raster.1)?.floor(),
            x(right).ceil(),
            self.source_y(bottom, raster.1)?.ceil(),
        ];
        if coordinates.iter().any(|value| !value.is_finite() || *value < 0.0)
            || coordinates[2] > f64::from(self.width)
            || coordinates[3] > f64::from(self.height)
        {
            return None;
        }
        let [left, top, right, bottom] = coordinates.map(|value| value as u32);
        Some(super::region_reconciliation::RegionBox {
            left,
            top,
            right,
            bottom,
        })
    }

    pub(super) fn line_ink_components(
        &self,
        bounds: super::region_reconciliation::RegionBox,
    ) -> Option<Vec<Range<u32>>> {
        const MAX_SCAN_PIXELS: u32 = 64 * 1024;
        const BRIDGE: u32 = 2;
        let width = bounds.right.checked_sub(bounds.left)?;
        let height = bounds.bottom.checked_sub(bounds.top)?;
        if width == 0
            || height == 0
            || bounds.right > self.width
            || bounds.bottom > self.height
            || width.checked_mul(height)? > MAX_SCAN_PIXELS
        {
            return None;
        }

        let mut components: Vec<Range<u32>> = Vec::new();
        for x in bounds.left..bounds.right {
            if !self
                .rows(bounds.top..bounds.bottom)
                .any(|row| self.sample_has_ink(row[x as usize]))
            {
                continue;
            }
            if let Some(last) = components.last_mut()
                && x - last.end <= BRIDGE
            {
                last.end = x + 1;
            } else {
                components.push(x..x + 1);
            }
        }
        Some(components)
    }

    pub(super) fn source_line_column(
        &self,
        region: super::region_reconciliation::RegionBox,
    ) -> Option<super::region_reconciliation::RegionBox> {
        use super::region_reconciliation::RegionBox;
        let gutter_width = region.bottom.checked_sub(region.top)?.checked_mul(2)?.max(12);
        let scan = RegionBox {
            left: 0,
            right: self.width,
            top: region.top,
            bottom: region.bottom,
        };
        let components = self.line_ink_components(scan)?;
        let mut left = 0;
        let mut right = self.width;
        let mut previous = 0;
        for next in components
            .iter()
            .map(|ink| ink.start)
            .chain(std::iter::once(self.width))
            .zip(components.iter().map(|ink| ink.end).chain(std::iter::once(self.width)))
        {
            let (start, end) = next;
            if start - previous >= gutter_width {
                let center = previous + (start - previous) / 2;
                if region.left < center && center < region.right {
                    return None;
                }
                if center <= region.left {
                    left = center;
                }
                if center >= region.right {
                    right = right.min(center);
                }
            }
            previous = end;
        }
        let column = RegionBox { left, right, ..scan };
        (column.contains(region) && (left != 0 || right != self.width)).then_some(column)
    }

    fn sample_has_ink(&self, sample: u8) -> bool {
        const INK_THRESHOLD: u8 = 224;
        let offset = usize::from(sample) * 3;
        self.palette[offset..offset + 3]
            .iter()
            .any(|&channel| channel < INK_THRESHOLD)
    }

    pub(super) fn has_panel_gutter(&self, bounds: super::region_reconciliation::RegionBox) -> bool {
        const MAX_SCAN: u32 = 256 * 1024;
        const MIN_GUTTER: u32 = 16;
        if bounds.right > self.width
            || bounds.bottom > self.height
            || bounds.top >= bounds.bottom
            || bounds.left >= bounds.right
            || (bounds.right - bounds.left).saturating_mul(bounds.bottom - bounds.top) > MAX_SCAN
        {
            return false;
        }
        let mut white_columns = 0;
        for x in bounds.left..bounds.right {
            if self
                .rows(bounds.top..bounds.bottom)
                .all(|row| self.white(row[x as usize]))
            {
                white_columns += 1;
                if white_columns >= MIN_GUTTER {
                    return true;
                }
            } else {
                white_columns = 0;
            }
        }
        false
    }

    pub(super) fn panel_gap_label(
        &self,
        rows: Range<u32>,
    ) -> Option<(
        super::region_reconciliation::RegionBox,
        super::region_reconciliation::RegionBox,
    )> {
        use super::region_reconciliation::RegionBox;
        const MAX_GAP_PIXELS: u32 = 3 * 1024 * 1024;
        const CONTRAST: u8 = 8;
        if rows.end > self.height || self.width.checked_mul(rows.end.checked_sub(rows.start)?)? > MAX_GAP_PIXELS {
            return None;
        }
        let tone = |sample: u8| {
            let offset = usize::from(sample) * 3;
            *self.palette[offset..offset + 3].iter().max().unwrap()
        };
        let mut panel: Option<RegionBox> = None;
        for (offset, row) in self.rows(rows.clone()).enumerate() {
            let y = rows.start + offset as u32;
            let Some(left) = row.iter().position(|&sample| self.sample_has_ink(sample)) else {
                continue;
            };
            let right = row.iter().rposition(|&sample| self.sample_has_ink(sample))? + 1;
            if left == 0
                || right == row.len()
                || y == rows.start
                || y + 1 == rows.end
                || !row[left..right].iter().all(|&sample| tone(sample) <= 128)
            {
                return None;
            }
            if let Some(panel) = &mut panel {
                if (panel.left, panel.right, panel.bottom) != (left as u32, right as u32, y) {
                    return None;
                }
                panel.bottom += 1;
            } else {
                panel = Some(RegionBox {
                    left: left as u32,
                    right: right as u32,
                    top: y,
                    bottom: y + 1,
                });
            }
        }
        let panel = panel?;
        let panel_rows = self.rows(panel.top..panel.bottom);
        let mut background = u8::MAX;
        let slice = panel.left as usize..panel.right as usize;
        for (i, row) in panel_rows.enumerate() {
            let row = &row[slice.clone()];
            let border = if i == 0 || i + 1 == (panel.bottom - panel.top) as usize {
                row.iter().copied().map(tone).min()?
            } else {
                tone(row[0]).min(tone(*row.last()?))
            };
            background = background.min(border);
        }
        let threshold = background.checked_sub(CONTRAST)?;
        let mut label: Option<RegionBox> = None;
        for (i, row) in self.rows(panel.top..panel.bottom).enumerate() {
            let y = panel.top + i as u32;
            let row = &row[slice.clone()];
            let mut ink = row.iter().enumerate().filter(|&(_, sample)| tone(*sample) <= threshold);
            let Some((left, _)) = ink.next() else { continue };
            let right = ink.next_back().map_or(left, |(right, _)| right) + 1;
            let (left, right) = (panel.left + left as u32, panel.left + right as u32);
            label = Some(match label {
                Some(prior) if y == prior.bottom => RegionBox {
                    left: prior.left.min(left),
                    right: prior.right.max(right),
                    bottom: y + 1,
                    ..prior
                },
                Some(_) => return None,
                None => RegionBox {
                    left,
                    right,
                    top: y,
                    bottom: y + 1,
                },
            });
        }
        let label = label?;
        (label.left > panel.left && label.right < panel.right && label.top > panel.top && label.bottom < panel.bottom)
            .then_some((label, panel))
    }

    pub(super) fn select_full_line_crop(
        &self,
        stock: super::region_reconciliation::RegionBox,
        protected: &[super::region_reconciliation::RegionBox],
    ) -> Option<super::region_reconciliation::RegionBox> {
        use super::region_reconciliation::{RegionBox, select_full_line_crop};
        let frame = RegionBox {
            left: 0,
            top: 0,
            right: self.width,
            bottom: self.height,
        };
        select_full_line_crop(frame, stock, protected, |x, y| {
            self.sample_has_ink(self.indices[y as usize * self.width as usize + x as usize])
        })
    }

    pub(super) fn line_ink_fully_covered(
        &self,
        region: super::region_reconciliation::RegionBox,
        boxes: &[super::region_reconciliation::RegionBox],
    ) -> bool {
        const MAX_SCAN_PIXELS: u32 = 64 * 1024;
        if region.left >= region.right
            || region.top >= region.bottom
            || region.right > self.width
            || region.bottom > self.height
            || (region.right - region.left)
                .checked_mul(region.bottom - region.top)
                .is_none_or(|pixels| pixels > MAX_SCAN_PIXELS)
        {
            return false;
        }
        let mut found = false;
        for y in region.top..region.bottom {
            let row_start = y as usize * self.width as usize;
            for x in region.left..region.right {
                if !self.sample_has_ink(self.indices[row_start + x as usize]) {
                    continue;
                }
                if !boxes
                    .iter()
                    .any(|bounds| bounds.left <= x && x < bounds.right && bounds.top <= y && y < bounds.bottom)
                {
                    return false;
                }
                found = true;
            }
        }
        found
    }

    pub(super) fn certify_line_ink(
        &self,
        hull: super::region_reconciliation::RegionBox,
        column: super::region_reconciliation::RegionBox,
    ) -> Option<super::region_reconciliation::RegionBox> {
        const COLLAR: u32 = 4;
        let frame = super::region_reconciliation::RegionBox {
            left: 0,
            top: 0,
            right: self.width,
            bottom: self.height,
        };
        if !column.contains(hull)
            || !frame.contains(column)
            || (hull.right - hull.left).checked_mul(hull.bottom - hull.top)? > 64 * 1024
        {
            return None;
        }
        let mut found_ink = false;
        for y in hull.top.checked_sub(COLLAR)?..hull.bottom.checked_add(COLLAR)? {
            let left = hull.left.checked_sub(COLLAR)?;
            let right = hull.right.checked_add(COLLAR)?;
            if y >= self.height || right > self.width {
                return None;
            }
            let row_start = y as usize * self.width as usize;
            for x in left..right {
                if self.sample_has_ink(self.indices[row_start + x as usize]) {
                    if x < hull.left || x >= hull.right || y < hull.top || y >= hull.bottom {
                        return None;
                    }
                    found_ink = true;
                }
            }
        }
        found_ink.then_some(hull)
    }

    pub(super) fn encode_region(&self, region: super::region_reconciliation::RegionBox, pad: u32) -> Option<Vec<u8>> {
        let width = region.right.checked_sub(region.left)?;
        let height = region.bottom.checked_sub(region.top)?;
        if width == 0 || height == 0 || region.right > self.width || region.bottom > self.height {
            return None;
        }
        let margin = pad.checked_mul(2)?;
        let padded_width = width.checked_add(margin)?;
        let padded_height = height.checked_add(margin)?;
        let pixels = (padded_width as usize).checked_mul(padded_height as usize)?;
        let working = pixels.checked_mul(TILE_BYTES_PER_PIXEL)?;
        if padded_width > MAX_TILE_SIDE || padded_height > MAX_TILE_SIDE || working > self.available {
            return None;
        }
        let mut rgb = Vec::new();
        let rgb_len = pixels.checked_mul(3)?;
        rgb.try_reserve_exact(rgb_len).ok()?;
        rgb.resize(rgb_len, 255);
        for (y, row) in self.rows(region.top..region.bottom).enumerate() {
            let start = ((y + pad as usize) * padded_width as usize + pad as usize) * 3;
            for (x, &sample) in row[region.left as usize..region.right as usize].iter().enumerate() {
                let palette_offset = usize::from(sample) * 3;
                let dest = start + x * 3;
                rgb[dest..dest + 3].copy_from_slice(&self.palette[palette_offset..palette_offset + 3]);
            }
        }
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&rgb, padded_width, padded_height, image::ExtendedColorType::Rgb8)
            .ok()?;
        (working.checked_add(png.capacity())? <= self.available).then_some(png)
    }

    pub(super) fn encode(&self, rows: Range<u32>) -> Option<Vec<u8>> {
        let height = rows.end.checked_sub(rows.start)?;
        if rows.end > self.height || height == 0 || height as usize > self.max_tile_rows() {
            return None;
        }
        let pixels = self.width as usize * height as usize;
        let mut rgb = Vec::new();
        rgb.try_reserve_exact(pixels.checked_mul(3)?).ok()?;
        for sample in self.rows(rows).flatten() {
            let i = usize::from(*sample) * 3;
            rgb.extend_from_slice(&self.palette[i..i + 3]);
        }
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&rgb, self.width, height, image::ExtendedColorType::Rgb8)
            .ok()?;
        (pixels.checked_mul(TILE_BYTES_PER_PIXEL)?.checked_add(png.capacity())? <= self.available).then_some(png)
    }
}

fn ink_bands<'a>(
    rows: impl Iterator<Item = &'a [u8]>,
    start_x: usize,
    white: impl Fn(u8) -> bool,
) -> Vec<(Range<u32>, usize)> {
    let mut bands = Vec::new();
    let mut current = None::<(Range<u32>, usize)>;
    for (y, row) in rows.enumerate() {
        let y = y as u32;
        if let Some(x) = row[start_x..].iter().position(|&sample| !white(sample)) {
            let x = x + start_x;
            let (band, left) = current.get_or_insert((y..y + 1, x));
            band.end = y + 1;
            *left = (*left).min(x);
        } else if let Some(band) = current.take() {
            bands.push(band);
        }
    }
    bands.extend(current);
    bands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agreed_collapsed_reads_cannot_omit_source_ink_at_an_edge() {
        use super::super::{
            region_probe::{RegionWord, collapsed_read_covers_ink},
            region_reconciliation::RegionBox,
        };
        let mut palette = [255; PALETTE_BYTES];
        palette[..3].fill(0);
        let mut indices = vec![1; 32 * 16];
        for y in 5..11 {
            indices[y * 32 + 4..y * 32 + 11].fill(0);
            indices[y * 32 + 15..y * 32 + 24].fill(0);
        }
        let source = IndexedSource {
            width: 32,
            height: 16,
            dpi: 72.0,
            frame: Rect::new(0.0, 0.0, 32.0, 16.0),
            painted_height: 16.0,
            painted_width: 32.0,
            indices,
            palette,
            available: FIXED_WORK_BYTES,
        };
        let region = RegionBox {
            left: 3,
            top: 4,
            right: 25,
            bottom: 12,
        };
        let read = |left, top| {
            [
                RegionWord {
                    text: "First".into(),
                    confidence: 99.0,
                    bounds: RegionBox {
                        left,
                        top,
                        right: 11,
                        bottom: 11,
                    },
                },
                RegionWord {
                    text: "Second".into(),
                    confidence: 99.0,
                    bounds: RegionBox {
                        left: 15,
                        top: 5,
                        right: 24,
                        bottom: 11,
                    },
                },
            ]
        };
        for words in [read(5, 6), read(5, 6)] {
            assert!(collapsed_read_covers_ink(&source, region, &words));
        }
        for words in [read(10, 5), read(10, 5), read(4, 8)] {
            assert!(!collapsed_read_covers_ink(&source, region, &words));
        }
    }

    #[test]
    fn owned_gap_scan_rejects_same_height_sidebar_panel() {
        use super::super::{region_probe::owned_gap_crop, region_reconciliation::RegionBox};
        let width = 240;
        let height = 160;
        let anchors = [
            RegionBox {
                left: 36,
                right: 68,
                top: 12,
                bottom: 24,
            },
            RegionBox {
                left: 37,
                right: 69,
                top: 126,
                bottom: 138,
            },
        ];
        let make_source = |panel_left: u32| {
            let mut palette = [255; PALETTE_BYTES];
            palette[3..6].fill(64);
            palette[6..9].fill(0);
            let mut indices = vec![0; (width * height) as usize];
            for y in 32..120 {
                let row = (y * width + panel_left) as usize;
                indices[row..row + 96].fill(1);
            }
            for y in 62..70 {
                let row = (y * width + panel_left + 30) as usize;
                indices[row..row + 24].fill(2);
            }
            IndexedSource {
                width,
                height,
                dpi: 72.0,
                frame: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: width as _,
                    height: height as _,
                },
                painted_height: f64::from(height),
                painted_width: f64::from(width),
                indices,
                palette,
                available: 4 * 1024 * 1024,
            }
        };
        let sidebar = make_source(124);
        assert!(sidebar.panel_gap_label(24..126).is_some());
        assert!(owned_gap_crop(&sidebar, 24..126, anchors).is_none());
        let aligned = make_source(28);
        assert!(owned_gap_crop(&aligned, 24..126, anchors).is_some());
    }

    const SCAN_WIDTH: usize = 4;
    const SCAN_HEIGHT: usize = 4100;

    fn filtered_image(filter: lopdf::Object) -> lopdf::Stream {
        use lopdf::{Object as LopdfObject, Stream, dictionary};
        use std::io::Write;

        let mut samples = vec![1; SCAN_WIDTH * SCAN_HEIGHT];
        samples[SCAN_WIDTH + 1] = 0;
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&samples).unwrap();
        let mut palette = vec![255; PALETTE_BYTES];
        palette[..3].fill(0);
        Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => SCAN_WIDTH as i64, "Height" => SCAN_HEIGHT as i64,
                "BitsPerComponent" => 8, "Filter" => filter,
                "ColorSpace" => vec![LopdfObject::Name(b"Indexed".to_vec()),
                    LopdfObject::Name(b"DeviceRGB".to_vec()), LopdfObject::Integer(255),
                    LopdfObject::String(palette, lopdf::StringFormat::Hexadecimal)],
            },
            encoder.finish().unwrap(),
        )
    }

    fn filtered_scan(filter: lopdf::Object, extra_paint: bool) -> lopdf::Document {
        use lopdf::{Document, Object as LopdfObject, Stream, dictionary};

        let mut document = Document::with_version("1.7");
        let pages = document.new_object_id();
        let image = document.add_object(filtered_image(filter));
        let mut paint = format!("q {SCAN_WIDTH} 0 0 {SCAN_HEIGHT} 0 0 cm /Scan Do Q").into_bytes();
        if extra_paint {
            paint.extend_from_slice(b" 0 0 m 1 1 l S");
        }
        let contents = document.add_object(Stream::new(dictionary! {}, paint));
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages,
            "MediaBox" => LopdfObject::Array(vec![0.into(), 0.into(),
                (SCAN_WIDTH as i64).into(), (SCAN_HEIGHT as i64).into()]),
            "Resources" => dictionary! {"XObject" => dictionary! {"Scan" => image}}, "Contents" => contents,
        });
        document.objects.insert(
            pages,
            dictionary! {
                "Type" => "Pages", "Kids" => vec![LopdfObject::Reference(page)], "Count" => 1,
            }
            .into(),
        );
        let root = document.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages});
        document.trailer.set("Root", root);
        document
    }

    fn load_filtered_scan(filter: lopdf::Object, extra_paint: bool) -> Option<IndexedSource> {
        load_scan(filtered_scan(filter, extra_paint))
    }

    fn load_scan(mut document: lopdf::Document) -> Option<IndexedSource> {
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        let document = PdfDocument::from_bytes(bytes).unwrap();
        IndexedSource::load(&super::super::Attempt {
            document: &document,
            page: 0,
            raster: Some((1, 100, 0)),
            read_width: Some(1),
            byte_limit: 8 * FIXED_WORK_BYTES,
            max_strips: 4,
            cancel: None,
        })
    }

    #[test]
    fn singleton_flate_filter_loads_the_same_indexed_source_samples() {
        let filter = lopdf::Object::Name(b"FlateDecode".to_vec());
        let scalar = load_filtered_scan(filter.clone(), false).expect("certified scalar filter source");
        let singleton =
            load_filtered_scan(lopdf::Object::Array(vec![filter]), false).expect("equivalent singleton filter source");
        assert_eq!((singleton.width, singleton.height, singleton.dpi), (4, 4100, 72.0));
        assert_eq!(singleton.indices, scalar.indices);
        assert_eq!(singleton.palette, scalar.palette);
        assert_eq!(singleton.available, scalar.available);
    }

    #[test]
    fn singleton_filter_keeps_the_exact_filter_chain_and_single_paint_proofs() {
        let filter = lopdf::Object::Name(b"FlateDecode".to_vec());
        for filters in [
            vec![],
            vec![filter.clone(), filter.clone()],
            vec![filter.clone(), lopdf::Object::Name(b"DCTDecode".to_vec())],
            vec![lopdf::Object::Name(b"ASCII85Decode".to_vec()), filter.clone()],
            vec![lopdf::Object::Name(b"DCTDecode".to_vec())],
        ] {
            assert!(load_filtered_scan(lopdf::Object::Array(filters), false).is_none());
        }
        assert!(load_filtered_scan(lopdf::Object::Array(vec![filter]), true).is_none());
    }

    fn scan_image_mut(document: &mut lopdf::Document) -> &mut lopdf::Stream {
        document
            .objects
            .values_mut()
            .filter_map(|object| object.as_stream_mut().ok())
            .find(|stream| stream.dict.get(b"Subtype").and_then(lopdf::Object::as_name).ok() == Some(b"Image"))
            .expect("one source image")
    }

    fn singleton_flate() -> lopdf::Object {
        lopdf::Object::Array(vec![lopdf::Object::Name(b"FlateDecode".to_vec())])
    }

    fn wrap_content_and_palette(document: &mut lopdf::Document) -> lopdf::ObjectId {
        use lopdf::{Stream, dictionary};

        let mut palette = vec![255; PALETTE_BYTES];
        palette[..3].fill(0);
        let mut stream = Stream::new(dictionary! {}, palette);
        stream.compress().unwrap();
        stream.dict.set("Filter", singleton_flate());
        let palette = document.add_object(stream);
        scan_image_mut(document)
            .dict
            .get_mut(b"ColorSpace")
            .unwrap()
            .as_array_mut()
            .unwrap()[3] = palette.into();
        let content = document
            .objects
            .values_mut()
            .filter_map(|object| object.as_stream_mut().ok())
            .find(|stream| !stream.dict.has(b"Subtype") && !stream.dict.has(b"Filter"))
            .expect("one uncompressed content stream");
        use std::io::Write;
        let original = content.content.clone();
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&original).unwrap();
        content.set_content(encoder.finish().unwrap());
        content.dict.set("Filter", singleton_flate());
        assert_eq!(content.decompressed_content().unwrap(), original);
        palette
    }

    #[test]
    fn indexed_stream_palette_requires_budget_after_reserving_source_indices() {
        let mut document = filtered_scan(singleton_flate(), false);
        wrap_content_and_palette(&mut document);
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        let document = PdfDocument::from_bytes(bytes).unwrap();
        let mut attempt = super::super::Attempt {
            document: &document,
            page: 0,
            raster: Some((1, 100, 0)),
            read_width: Some(1),
            byte_limit: 8 * FIXED_WORK_BYTES,
            max_strips: 4,
            cancel: None,
        };
        let image = indexed_image(&attempt, None).expect("single supported indexed frame");
        assert!(IndexedSource::load(&attempt).is_some());
        assert!(is_indexed_rgb_scan(&attempt));
        attempt.byte_limit -= image.available;
        assert_eq!(indexed_image(&attempt, None).unwrap().available, 0);
        assert!(IndexedSource::load(&attempt).is_none());
        assert!(!is_indexed_rgb_scan(&attempt));
    }

    #[test]
    fn should_decode_singleton_flate_in_content_and_palette_like_the_scalar_form() {
        let scalar = load_filtered_scan(lopdf::Object::Name(b"FlateDecode".to_vec()), false).unwrap();
        let mut document = filtered_scan(singleton_flate(), false);
        wrap_content_and_palette(&mut document);
        let singleton = load_scan(document).expect("same certified samples with every filter wrapped");
        assert_eq!(singleton.indices, scalar.indices);
        assert_eq!(singleton.palette, scalar.palette);
        assert_eq!(
            (singleton.width, singleton.height, singleton.dpi),
            (scalar.width, scalar.height, scalar.dpi)
        );
    }

    #[test]
    fn should_reject_all_decode_parms_forms_on_singleton_image_content_and_palette() {
        use lopdf::{Object as LopdfObject, dictionary};

        let mut accepted = Vec::new();
        for params in [
            LopdfObject::Dictionary(dictionary! {}),
            LopdfObject::Array(vec![LopdfObject::Dictionary(dictionary! {})]),
            LopdfObject::Array(vec![LopdfObject::Null]),
        ] {
            for target in ["image", "content", "palette"] {
                let mut document = filtered_scan(singleton_flate(), false);
                let palette = wrap_content_and_palette(&mut document);
                let stream = scan_stream_mut(&mut document, palette, target);
                stream.dict.set("DecodeParms", params.clone());
                if load_scan(document).is_some() {
                    accepted.push((target, params.clone()));
                }
            }
        }
        assert!(accepted.is_empty(), "unsupported DecodeParms accepted: {accepted:?}");
    }

    fn scan_stream_mut<'a>(
        document: &'a mut lopdf::Document,
        palette: lopdf::ObjectId,
        target: &str,
    ) -> &'a mut lopdf::Stream {
        match target {
            "image" => scan_image_mut(document),
            "palette" => document.get_object_mut(palette).unwrap().as_stream_mut().unwrap(),
            "content" => document
                .objects
                .iter_mut()
                .filter(|(id, _)| **id != palette)
                .filter_map(|(_, object)| object.as_stream_mut().ok())
                .find(|stream| !stream.dict.has(b"Subtype"))
                .unwrap(),
            _ => panic!("unknown fixture stream: {target}"),
        }
    }

    #[test]
    fn should_reject_an_indirect_filter_element_in_image_content_and_palette_even_when_it_resolves_to_flate() {
        let mut accepted = Vec::new();
        for target in ["image", "content", "palette"] {
            let mut document = filtered_scan(singleton_flate(), false);
            let palette = wrap_content_and_palette(&mut document);
            let filter = document.add_object(lopdf::Object::Name(b"FlateDecode".to_vec()));
            scan_stream_mut(&mut document, palette, target)
                .dict
                .set("Filter", vec![lopdf::Object::Reference(filter)]);
            if load_scan(document).is_some() {
                accepted.push(target);
            }
        }
        assert!(accepted.is_empty(), "indirect filter elements accepted: {accepted:?}");
    }

    #[test]
    fn rejects_placement_that_can_clip_beyond_one_edge_sample() {
        let frame = Rect::new(0.0, 0.0, 100.0, 1000.0);
        assert_eq!(placement_fits(frame, (100, 1000), (100.01, 999.99)), Some(72.0));
        assert!(placement_fits(frame, (100, 1000), (101.2, 999.99)).is_none());
        assert!(placement_fits(frame, (100, 1000), (100.01, 998.9)).is_none());
    }
    #[test]
    fn heading_planner_rejects_native_gutter_after_same_line_admission() {
        use super::super::{
            region_probe::{discover_isolated_headings, same_line},
            region_reconciliation::RegionBox,
        };
        let word = |left| RegionBox {
            left,
            right: left + 10,
            top: 10,
            bottom: 20,
        };
        let plan = |second| {
            let words = [word(20), word(second)];
            assert!(same_line(words[0], words[1]));
            let mut palette = [255; PALETTE_BYTES];
            palette[..3].fill(0);
            let mut indices = vec![1; 100 * 30];
            for y in 10..20 {
                for word in words {
                    indices[y * 100 + word.left as usize..y * 100 + word.right as usize].fill(0);
                }
            }
            let source = IndexedSource {
                width: 100,
                height: 30,
                dpi: 72.0,
                frame: Rect::new(0.0, 0.0, 100.0, 30.0),
                painted_height: 30.0,
                painted_width: 100.0,
                indices,
                palette,
                available: FIXED_WORK_BYTES,
            };
            discover_isolated_headings(&source, &words, 2).unwrap()
        };
        assert_eq!(plan(36).len(), 1);
        assert!(plan(54).is_empty());
    }

    #[test]
    fn clipped_edge_must_be_white_even_if_placement_is_subpixel() {
        let mut palette = [255; PALETTE_BYTES];
        palette[..3].fill(0);
        let mut source = IndexedSource {
            width: 3,
            height: 3,
            dpi: 72.0,
            frame: Rect::new(0.0, 0.0, 3.0, 3.0),
            painted_height: 3.0,
            painted_width: 3.0,
            indices: vec![1, 1, 1, 1, 0, 1, 1, 1, 1],
            palette,
            available: FIXED_WORK_BYTES,
        };
        assert!(source.white_edges());
        source.indices[5] = 0;
        assert!(!source.white_edges());
    }
    #[test]
    fn refuses_rgb_encoding_before_allocating_when_tile_rows_exceed_budget() {
        let source = IndexedSource {
            width: 3,
            height: 3,
            dpi: 72.0,
            frame: Rect::new(0.0, 0.0, 3.0, 3.0),
            painted_height: 3.0,
            painted_width: 3.0,
            indices: vec![0; 9],
            palette: [255; PALETTE_BYTES],
            available: 8 * TILE_BYTES_PER_PIXEL,
        };
        assert_eq!(source.max_tile_rows(), 2);
        assert!(source.encode(0..3).is_none());
        assert!(source.backend_limit(source.available + 1).is_none());
    }

    #[test]
    fn refuses_inherited_indirect_default_rgb_authority_for_indexed_samples() {
        use lopdf::{Document, Object as LopdfObject, dictionary};

        let mut document = Document::with_version("1.7");
        let pages = document.new_object_id();
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages,
        });
        let default = document.add_object(LopdfObject::Name(b"DeviceRGB".to_vec()));
        let aliases = document.add_object(dictionary! { "DefaultRGB" => default });
        let resources = document.add_object(dictionary! { "ColorSpace" => aliases });
        document.objects.insert(
            pages,
            dictionary! { "Type" => "Pages", "Kids" => vec![LopdfObject::Reference(page)],
            "Count" => 1, "Resources" => resources }
            .into(),
        );
        let root = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        document.trailer.set("Root", root);
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        let source = PdfDocument::from_bytes(bytes).unwrap();
        let effective = source.get_page_resources(0).unwrap();
        let color = Object::Array(vec![
            Object::Name("Indexed".into()),
            Object::Name("DeviceRGB".into()),
            Object::Integer(255),
            Object::String(vec![255; PALETTE_BYTES]),
        ]);
        assert!(
            palette(
                &source,
                &Object::Dictionary(Default::default()),
                &color,
                FIXED_WORK_BYTES
            )
            .is_some()
        );
        assert!(palette(&source, &effective, &color, FIXED_WORK_BYTES).is_none());
    }
}
