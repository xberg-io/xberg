use crate::error::{Error, Result};
use crate::extractors::images::{ImageData, PdfImage};
use tiny_skia::Transform;

#[derive(Clone)]
enum Failure {
    Exceeded { required: u64, limit: usize },
    Unqualified(String),
}

impl Failure {
    fn error(&self) -> Error {
        Error::Image(match self {
            Self::Exceeded { required, limit } => format!(
                "JPEG source image rasters and retained canvas require {required} bytes, exceeding image working budget {limit}"
            ),
            Self::Unqualified(reason) => {
                format!("JPEG source image workspace cannot be admitted within the image budget: {reason}")
            }
        })
    }
}

pub(super) struct ImageBudget {
    limit: usize,
    failure: Option<Failure>,
}

/// How an admitted JPEG is decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum JpegDecode {
    /// At full size through `PdfImage::to_dynamic_image`, then converted to RGBA.
    Full,
    /// At full size straight into the RGBA paint raster, with [`decode_jpeg_into_rgba`].
    InPlace,
    /// At 1/n with [`decode_reduced_jpeg`].
    Reduced(u8),
}

/// How `render_image` draws an axis-aligned downscale (`fast_resize_target`); every other image is
/// drawn by tiny-skia's scaled pattern draw either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Blit {
    /// Pre-resize with `fast_image_resize`, then copy 1:1.
    PreResize,
    /// Let tiny-skia scale while drawing, bicubic, holding no resized copy. Admitted only when the
    /// pre-resize does not fit.
    Direct,
}

/// An admitted JPEG paint: how it is decoded and how it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct JpegPaint {
    pub(super) decode: JpegDecode,
    pub(super) blit: Blit,
}

/// Bytes a tiny-skia `draw_pixmap` allocates beyond the pixmaps it reads and writes: the pattern
/// borrows the source and the raster pipeline lives on the stack, leaving a 64-entry edge list
/// (at most 64 x 96 bytes), the transformed rect path copies (under 100 bytes each, a few alive at
/// once, one per tile on a canvas wider or taller than 8191 px) and allocation rounding.
pub(super) const TINY_SKIA_DRAW_BYTES: u64 = 16 * 1024;

/// Smallest scale, on either axis, at which a [`Blit::Direct`] draw is admitted. tiny-skia's bicubic
/// kernel reads only the 4x4 source pixels around each sample, so below half scale it skips source
/// pixels and aliases; such an image is refused rather than drawn that way.
const MIN_DIRECT_DRAW_SCALE: f32 = 0.5;

/// Where and how an image is painted: the device transform of its unit square and the [`Blit`].
#[derive(Clone, Copy)]
pub(super) struct Placement {
    transform: Transform,
    blit: Blit,
}

impl Placement {
    pub(super) fn new(transform: Transform) -> Self {
        Self {
            transform,
            blit: Blit::PreResize,
        }
    }

    /// The unit square painted upright over `width` x `height` device pixels.
    #[cfg(test)]
    pub(super) fn extent(width: f32, height: f32) -> Self {
        Self::new(Transform::from_row(width, 0.0, 0.0, -height, 0.0, height))
    }

    fn with_blit(self, blit: Blit) -> Self {
        Self { blit, ..self }
    }

    /// The device extent, in canvas pixels, the image is painted over on each axis.
    fn painted(self) -> (f32, f32) {
        let transform = self.transform;
        (transform.sx.hypot(transform.ky), transform.kx.hypot(transform.sy))
    }

    /// Bytes drawing a `source` RGBA raster holds beside that raster: the larger of a pre-resize's
    /// working set and its resized copy during the draw, or the draw alone. `None` for a direct draw
    /// that would shrink either axis below [`MIN_DIRECT_DRAW_SCALE`].
    fn paint_bytes(self, source: (u32, u32)) -> Option<u64> {
        let image_transform = super::image_unit_square_transform(self.transform, source.0, source.1);
        match (super::fast_resize_target(image_transform, source), self.blit) {
            (Some(target), Blit::PreResize) => {
                let resized = u64::from(target.0)
                    .saturating_mul(u64::from(target.1))
                    .saturating_mul(4);
                Some(resize_working_bytes(source, target).max(resized.saturating_add(TINY_SKIA_DRAW_BYTES)))
            }
            (Some(_), Blit::Direct)
                if image_transform.sx < MIN_DIRECT_DRAW_SCALE || image_transform.sy < MIN_DIRECT_DRAW_SCALE =>
            {
                None
            }
            _ => Some(TINY_SKIA_DRAW_BYTES),
        }
    }
}

impl ImageBudget {
    pub(super) fn new(limit: usize) -> Self {
        Self { limit, failure: None }
    }

    /// [`Self::admit_jpeg_at`] for an image painted at its own pixel size, where no pre-resize runs.
    #[cfg(test)]
    pub(super) fn admit_jpeg(&mut self, image: &PdfImage, retained_canvas: u64) -> Result<()> {
        let own_size = Placement::extent(image.width() as f32, image.height() as f32);
        self.admit_jpeg_at(image, retained_canvas, own_size).map(drop)
    }

    /// Admit `image`'s full-size decode, drawn with the pre-resize when that fits and directly
    /// otherwise.
    pub(super) fn admit_jpeg_at(
        &mut self,
        image: &PdfImage,
        retained_canvas: u64,
        placement: Placement,
    ) -> Result<Blit> {
        let ImageData::Jpeg(bytes) = image.data() else {
            return Ok(Blit::PreResize);
        };
        let full = |blit| {
            let placement = placement.with_blit(blit);
            jpeg_image_bytes(image, bytes, bytes.capacity(), retained_canvas, 4, Some(placement))
        };
        for blit in [Blit::PreResize, Blit::Direct] {
            if full(blit).is_ok_and(|required| required <= self.limit as u64) {
                return Ok(blit);
            }
        }
        self.admit_required(smaller(full(Blit::PreResize), full(Blit::Direct)))
            .map(|()| Blit::PreResize)
    }

    /// [`Self::admit_jpeg_decode`] without the in-place decode, as the scale denominator, for an
    /// image painted upright over `device` pixels.
    #[cfg(test)]
    pub(super) fn admit_jpeg_reducing(
        &mut self,
        image: &PdfImage,
        retained_canvas: u64,
        device: (f32, f32),
    ) -> Result<u8> {
        self.admit_jpeg_decode(image, retained_canvas, Placement::extent(device.0, device.1), false)
            .map(|paint| match paint.decode {
                JpegDecode::Reduced(denominator) => denominator,
                JpegDecode::Full | JpegDecode::InPlace => 1,
            })
    }

    /// Admit `image`, or a decode that keeps its rasters within the budget when the full-size decode
    /// does not. The first that fits wins:
    ///
    /// 1. the full-size decode;
    /// 2. the finest IDCT-scaled decode (1/2, 1/4 or 1/8) whose reduced raster spans at least
    ///    [`MIN_PAINTED_COVERAGE`] of the device extent (in canvas pixels) `placement` paints the image
    ///    over on each axis, less one pixel of slack (`reduced >= 0.9 * device - 1`);
    /// 3. when `unmasked` (no /Mask, /SMask or in-data soft mask reads the decoded samples), the
    ///    full-size decode straight into the RGBA paint raster, which never holds the decoded raster
    ///    beside a converted copy.
    ///
    /// Each charge covers drawing the RGBA raster, including the pre-resize `render_image` runs on an
    /// axis-aligned downscale. Only when none of them fits with that pre-resize does the same ladder
    /// run again with a [`Blit::Direct`] draw. An image neither ladder fits is refused exactly as
    /// [`Self::admit_jpeg_at`] refuses it.
    pub(super) fn admit_jpeg_decode(
        &mut self,
        image: &PdfImage,
        retained_canvas: u64,
        placement: Placement,
        unmasked: bool,
    ) -> Result<JpegPaint> {
        let ImageData::Jpeg(bytes) = image.data() else {
            return Ok(JpegPaint {
                decode: JpegDecode::Full,
                blit: Blit::PreResize,
            });
        };
        for blit in [Blit::PreResize, Blit::Direct] {
            let placement = placement.with_blit(blit);
            if let Some(decode) =
                self.fitting_decode(image, (bytes, bytes.capacity()), retained_canvas, placement, unmasked)
            {
                return Ok(JpegPaint { decode, blit });
            }
        }
        let full = |blit| {
            let placement = placement.with_blit(blit);
            jpeg_image_bytes(image, bytes, bytes.capacity(), retained_canvas, 4, Some(placement))
        };
        self.admit_required(smaller(full(Blit::PreResize), full(Blit::Direct)))
            .map(|()| JpegPaint {
                decode: JpegDecode::Full,
                blit: Blit::PreResize,
            })
    }

    /// The first decode of [`Self::admit_jpeg_decode`]'s ladder that fits, drawn as `placement` says.
    /// `capacity` is that of the buffer holding the encoded `bytes`.
    fn fitting_decode(
        &self,
        image: &PdfImage,
        (bytes, capacity): (&[u8], usize),
        retained_canvas: u64,
        placement: Placement,
        unmasked: bool,
    ) -> Option<JpegDecode> {
        let device = placement.painted();
        let full = jpeg_image_bytes(image, bytes, capacity, retained_canvas, 4, Some(placement));
        if full.is_ok_and(|required| required <= self.limit as u64) {
            return Some(JpegDecode::Full);
        }
        // The rayon worker's batched `append_rows` path serves progressive scans only, and
        // `sequential_frame` excludes those. A frame that cannot be read, or that disagrees with the
        // dictionary, is not reduced and keeps the `Exceeded` refusal the OCR path keys on. ~keep
        let frame = reducible_jpeg(image, bytes)
            .then(|| jpeg_frame(bytes).ok())
            .flatten()
            .filter(|frame| *frame == (image.width(), image.height()));
        if let Some((frame_width, frame_height)) = frame {
            for denominator in REDUCED_DENOMINATORS {
                if !covers_device((frame_width, frame_height), denominator, device) {
                    break;
                }
                let reduced = reduced_jpeg_image_bytes(bytes, capacity, retained_canvas, denominator, placement);
                if reduced.is_ok_and(|required| required <= self.limit as u64) {
                    return Some(JpegDecode::Reduced(denominator));
                }
            }
            if unmasked
                && single_scan_frame(bytes)
                && in_place_jpeg_image_bytes(bytes, capacity, retained_canvas, placement)
                    .is_ok_and(|required| required <= self.limit as u64)
            {
                return Some(JpegDecode::InPlace);
            }
        }
        None
    }

    pub(super) fn admit_referenced_mask(
        &mut self,
        image: &PdfImage,
        parent: &PdfImage,
        dynamic: &image::DynamicImage,
        paint: &image::RgbaImage,
        retained_canvas: u64,
    ) -> Result<()> {
        let ImageData::Jpeg(bytes) = image.data() else {
            return Ok(());
        };
        let retained = parent_image_bytes(parent, dynamic, paint, retained_canvas);
        self.admit_required(
            retained.and_then(|retained| jpeg_image_bytes(image, bytes, bytes.capacity(), retained, 1, None)),
        )
    }

    /// Record the refusal for an image whose reduced or in-place decode failed after admission, which
    /// reads headers only. The page is then refused as the full-size decode would have been, not rendered
    /// without the image.
    pub(super) fn refuse_failed_reduction(&mut self, image: &PdfImage, retained_canvas: u64, placement: Placement) {
        if let ImageData::Jpeg(bytes) = image.data() {
            let full = jpeg_image_bytes(image, bytes, bytes.capacity(), retained_canvas, 4, Some(placement));
            let _ = self.admit_required(full);
            if self.failure.is_none() {
                self.failure = Some(Failure::Unqualified(
                    "the reduced or in-place JPEG decode failed".into(),
                ));
            }
        }
    }

    fn admit_required(&mut self, required: std::result::Result<u64, String>) -> Result<()> {
        let failure = match required {
            Ok(required) if required <= self.limit as u64 => return Ok(()),
            Ok(required) => Failure::Exceeded {
                required,
                limit: self.limit,
            },
            Err(reason) => Failure::Unqualified(reason),
        };
        self.failure.get_or_insert_with(|| failure.clone());
        Err(failure.error())
    }

    pub(super) fn exceeded(&self) -> bool {
        matches!(self.failure, Some(Failure::Exceeded { .. }))
    }

    pub(super) fn error(&self) -> Option<Error> {
        self.failure.as_ref().map(Failure::error)
    }
}

fn jpeg_image_bytes(
    image: &PdfImage,
    bytes: &[u8],
    encoded_capacity: usize,
    retained_canvas: u64,
    paint_components: u64,
    placement: Option<Placement>,
) -> std::result::Result<u64, String> {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    decoder.read_info().map_err(|error| error.to_string())?;
    let info = decoder.info().ok_or_else(|| "missing JPEG frame header".to_string())?;
    if info.pixel_format == jpeg_decoder::PixelFormat::L16 {
        return Err("only 8-bit JPEG source rasters have a known rendering workspace".into());
    }
    let pixels = u64::from(info.width) * u64::from(info.height);
    let decoded = pixels * info.pixel_format.pixel_bytes() as u64;
    let cmyk = image.color_space().components() == 4;
    if cmyk && info.pixel_format != jpeg_decoder::PixelFormat::CMYK32 {
        return Err("CMYK image dictionary disagrees with the JPEG frame components".into());
    }
    // CMYK conversion holds the decoded CMYK and RGB vectors. Other JPEGs retain
    // their actual decoded raster while to_rgba8 or to_luma8 allocates the paint raster. The
    // canvas and separation planes stay live throughout the image paint. Encoded
    // bytes retain their capacity and JPEG metadata can copy at most the input. ~keep
    let converted = if info.pixel_format == jpeg_decoder::PixelFormat::CMYK32 {
        pixels * 3
    } else {
        0
    };
    let paint_size = if cmyk {
        (image.width(), image.height())
    } else {
        (u32::from(info.width), u32::from(info.height))
    };
    let rgba_pixels = u64::from(paint_size.0) * u64::from(paint_size.1);
    // `render_image` frees the decoded raster before it draws the paint raster, so the draw's working
    // set shares the paint raster with it but not the decoded one. ~keep
    let draw = paint_bytes(placement, paint_size)?;
    Ok(decoded
        .saturating_add(converted)
        .max(draw)
        .saturating_add(rgba_pixels.saturating_mul(paint_components))
        .saturating_add(encoded_capacity as u64)
        .saturating_add(bytes.len() as u64)
        .saturating_add(retained_canvas))
}

/// What drawing a `source` raster as `placement` says holds beside it (nothing without a
/// placement), or why that draw is not admitted.
fn paint_bytes(placement: Option<Placement>, source: (u32, u32)) -> std::result::Result<u64, String> {
    placement
        .map_or(Some(0), |placement| placement.paint_bytes(source))
        .ok_or_else(|| format!("a direct draw below {MIN_DIRECT_DRAW_SCALE} scale is not admitted"))
}

/// The smaller of two charges, so a refusal reports the cheapest draw that still did not fit. Every
/// charge saturates instead of overflowing, so it stays a definite `Exceeded` refusal.
fn smaller(
    first: std::result::Result<u64, String>,
    second: std::result::Result<u64, String>,
) -> std::result::Result<u64, String> {
    match (first, second) {
        (Ok(first), Ok(second)) => Ok(first.min(second)),
        (Ok(charge), Err(_)) | (Err(_), Ok(charge)) => Ok(charge),
        (Err(reason), Err(_)) => Err(reason),
    }
}

fn parent_image_bytes(
    parent: &PdfImage,
    dynamic: &image::DynamicImage,
    paint: &image::RgbaImage,
    retained_canvas: u64,
) -> std::result::Result<u64, String> {
    // These are the Vec capacities still live across referenced-mask decoding.
    // The mask's own dynamic raster and luma conversion are admitted separately. ~keep
    let dynamic_capacity = match dynamic {
        image::DynamicImage::ImageLuma8(image) => image.as_raw().capacity(),
        image::DynamicImage::ImageLumaA8(image) => image.as_raw().capacity(),
        image::DynamicImage::ImageRgb8(image) => image.as_raw().capacity(),
        image::DynamicImage::ImageRgba8(image) => image.as_raw().capacity(),
        _ => return Err("referenced JPEG mask with unqualified parent raster type".into()),
    };
    parent
        .retained_image_buffer_bytes()
        .and_then(|bytes| bytes.checked_add(dynamic_capacity as u64))
        .and_then(|bytes| bytes.checked_add(paint.as_raw().capacity() as u64))
        .and_then(|bytes| bytes.checked_add(retained_canvas))
        .ok_or_else(|| "referenced JPEG mask parent workspace arithmetic overflow".into())
}

/// Scales `jpeg-decoder` can decode at without holding the full-resolution raster.
const REDUCED_DENOMINATORS: [u8; 3] = [2, 4, 8];
/// Bytes of one 8x8 block of `i16` coefficients.
const COEFFICIENT_BLOCK_BYTES: u64 = 64 * 2;
/// Largest vertical sampling factor, in blocks per MCU column.
const MAX_VERTICAL_BLOCKS: u64 = 4;

/// Whether `bytes` is a sequential, non-CMYK JPEG. A progressive scan keeps every coefficient of the
/// full-resolution image, so a reduced decode would not lower its peak; CMYK takes a separate
/// conversion path.
pub(super) fn reducible_jpeg(image: &PdfImage, bytes: &[u8]) -> bool {
    image.color_space().components() != 4 && sequential_frame(bytes)
}

fn sequential_frame(bytes: &[u8]) -> bool {
    let mut position = 2;
    while bytes.get(position) == Some(&0xff) {
        let Some(&marker) = bytes.get(position + 1) else {
            return false;
        };
        match marker {
            0xc0 | 0xc1 => return true,
            // DHT, JPG and DAC are not frame headers; every other SOFn and SOS is not sequential
            // Huffman, and SOS ends the headers. ~keep
            0xc4 | 0xc8 | 0xcc => {}
            0xc2..=0xcf | 0xda => return false,
            _ => {}
        }
        let Some(length) = bytes.get(position + 2..position + 4) else {
            return false;
        };
        position += 2 + usize::from(u16::from_be_bytes([length[0], length[1]]));
    }
    false
}

/// The frame dimensions of the JPEG itself, which the decoder allocates from, not the image dictionary's.
pub(super) fn jpeg_frame(bytes: &[u8]) -> std::result::Result<(u32, u32), String> {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    decoder.read_info().map_err(|error| error.to_string())?;
    let info = decoder.info().ok_or("missing JPEG frame header")?;
    Ok((u32::from(info.width), u32::from(info.height)))
}

/// Fraction of the painted canvas extent a reduced raster must still span, before the one pixel of
/// slack `covers_device` allows. It keeps a page whose canvas sits just above a 1/2 scale from being
/// refused.
const MIN_PAINTED_COVERAGE: f32 = 0.9;

/// Whether a frame of `frame` pixels at `denominator` still spans [`MIN_PAINTED_COVERAGE`] of the
/// canvas pixels it is painted over on both axes: `reduced >= painted * 0.9 - 1`. For a 600 px reduced
/// axis that accepts a painted extent up to 667 px and refuses 668 px.
fn covers_device(frame: (u32, u32), denominator: u8, (width, height): (f32, f32)) -> bool {
    let covers = |pixels: u32, painted: f32| {
        painted.is_finite() && reduced_dimension(pixels, denominator) as f32 >= painted * MIN_PAINTED_COVERAGE - 1.0
    };
    covers(frame.0, width) && covers(frame.1, height)
}

fn reduced_dimension(dimension: u32, denominator: u8) -> u64 {
    u64::from(dimension).div_ceil(u64::from(denominator))
}

/// The JPEG working set at `denominator`: the largest of the paint phase (scaled raster plus its
/// RGBA conversion), the decode phase (component planes, output and one MCU row of coefficients)
/// and the draw phase (RGBA raster plus what drawing it as `placement` says needs), on top of the encoded
/// bytes and the retained canvas.
///
/// The one-MCU-row bound holds because `jpeg-decoder` converts each coefficient row as it is read:
/// its `rayon` worker (enabled for this crate on every target that supports it) and its immediate
/// worker (the only one on wasm32) both do. The default thread-per-component worker queues
/// full-resolution rows without a bound, so this crate must not build `jpeg-decoder` without
/// `rayon` on a threaded target.
fn reduced_jpeg_image_bytes(
    bytes: &[u8],
    encoded_capacity: usize,
    retained_canvas: u64,
    denominator: u8,
    placement: Placement,
) -> std::result::Result<u64, String> {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    decoder.read_info().map_err(|error| error.to_string())?;
    let info = decoder.info().ok_or_else(|| "missing JPEG frame header".to_string())?;
    if !matches!(
        info.pixel_format,
        jpeg_decoder::PixelFormat::L8 | jpeg_decoder::PixelFormat::RGB24
    ) {
        return Err("only 8-bit grayscale and RGB JPEG sources can be decoded at a reduced scale".into());
    }
    let (width, height) = (
        reduced_dimension(u32::from(info.width), denominator),
        reduced_dimension(u32::from(info.height), denominator),
    );
    // `scale` picks the finest size reaching the request on either axis, so a thin frame can land on a
    // size other than the one admitted here; such a scale is not admitted. ~keep
    let chosen = u16::try_from(width)
        .ok()
        .zip(u16::try_from(height).ok())
        .and_then(|(width, height)| decoder.scale(width, height).ok());
    if chosen.map(|(w, h)| (u64::from(w), u64::from(h))) != Some((width, height)) {
        return Err("the decoder would not decode this frame at the admitted reduced size".into());
    }
    // Both sides fit u16, which `chosen` checked, so the raster arithmetic below cannot overflow. ~keep
    let pixels = width * height;
    let components = info.pixel_format.pixel_bytes() as u64;
    let decoded = pixels * components;
    let paint = decoded + pixels * 4;
    let coefficients =
        u64::from(info.width).div_ceil(8) * components * MAX_VERTICAL_BLOCKS * 2 * COEFFICIENT_BLOCK_BYTES;
    let decoding = decoded * 2 + coefficients;
    let drawing = (pixels * 4).saturating_add(paint_bytes(Some(placement), (width as u32, height as u32))?);
    Ok(paint
        .max(decoding)
        .max(drawing)
        .saturating_add(encoded_capacity as u64)
        .saturating_add(bytes.len() as u64)
        .saturating_add(retained_canvas))
}

/// Decode a JPEG admitted by [`ImageBudget::admit_jpeg_decode`] at `denominator`, never holding
/// the full-resolution raster.
pub(super) fn decode_reduced_jpeg(bytes: &[u8], denominator: u8) -> Result<image::DynamicImage> {
    let failed =
        |error: jpeg_decoder::Error| Error::Decode(format!("Failed to decode JPEG at 1/{denominator}: {error}"));
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    decoder.read_info().map_err(failed)?;
    let info = decoder
        .info()
        .ok_or_else(|| Error::Decode("missing JPEG frame header".into()))?;
    let width = u16::try_from(reduced_dimension(u32::from(info.width), denominator))
        .map_err(|_| Error::Decode("reduced JPEG width exceeds the frame limit".into()))?;
    let height = u16::try_from(reduced_dimension(u32::from(info.height), denominator))
        .map_err(|_| Error::Decode("reduced JPEG height exceeds the frame limit".into()))?;
    let actual = decoder.scale(width, height).map_err(failed)?;
    if actual != (width, height) {
        return Err(Error::Decode(format!(
            "JPEG decoded at {}x{}, not the admitted {width}x{height}",
            actual.0, actual.1
        )));
    }
    let pixels = decoder.decode().map_err(failed)?;
    let (width, height) = (u32::from(width), u32::from(height));
    match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => {
            image::GrayImage::from_raw(width, height, pixels).map(image::DynamicImage::ImageLuma8)
        }
        jpeg_decoder::PixelFormat::RGB24 => {
            image::RgbImage::from_raw(width, height, pixels).map(image::DynamicImage::ImageRgb8)
        }
        _ => None,
    }
    .ok_or_else(|| Error::Decode("reduced JPEG decoded to an unexpected pixel layout".into()))
}

/// Bytes per output pixel of one resized axis that hold its convolution coefficients beyond the
/// window itself: the `i16` chunk's struct, its allocation rounding and the pixel's bounds.
const COEFFICIENT_BYTES_PER_OUTPUT: u64 = 64;
/// Bytes per window slot per output pixel: the `f64` weight and the `i16` normalized copy, which
/// coexist while the normalizer builds its chunks.
const COEFFICIENT_BYTES_PER_SLOT: u64 = 8 + 2;

/// Bytes `resize_rgba` allocates to resize a `source` RGBA raster to `target` with
/// `fast_image_resize` (no `rayon`, bilinear convolution, alpha untouched): the output raster; for a
/// resize on both axes, the vertical-first intermediate of at most the source width by the target
/// height plus one alignment pixel; and each resized axis's coefficient table, whose bilinear window
/// is `2 * ceil(in / out) + 1` slots (3 for an enlarged axis) per output pixel. Saturates rather
/// than wrapping, so an absurd target is refused.
pub(super) fn resize_working_bytes(source: (u32, u32), target: (u32, u32)) -> u64 {
    let (source_width, source_height) = (u64::from(source.0), u64::from(source.1));
    let (target_width, target_height) = (u64::from(target.0), u64::from(target.1));
    let output = target_width.saturating_mul(target_height).saturating_mul(4);
    let horizontal = target_width != source_width;
    let vertical = target_height != source_height;
    let intermediate = if horizontal && vertical {
        source_width
            .saturating_mul(target_height)
            .saturating_mul(4)
            .saturating_add(4)
    } else {
        0
    };
    let coefficients = |resized: bool, input: u64, output: u64| {
        if !resized || output == 0 {
            return 0;
        }
        let window = input.div_ceil(output).max(1) * 2 + 1;
        output.saturating_mul(window * COEFFICIENT_BYTES_PER_SLOT + COEFFICIENT_BYTES_PER_OUTPUT)
    };
    output
        .saturating_add(intermediate)
        .saturating_add(coefficients(horizontal, source_width, target_width))
        .saturating_add(coefficients(vertical, source_height, target_height))
}

/// Widest MCU, in pixels: 8 times the largest horizontal sampling factor.
const MAX_MCU_COLUMNS: u64 = 32;
/// Bytes per padded frame column, per component, bounding zune-jpeg's row buffers when the first
/// scan holds every component. With sampling factors up to 4, its raw coefficient rows (32),
/// upsampling destination (256, doubled for misaligned factors), first-row copy (64),
/// `row`/`row_up` (8) and upsampler scratch (8) total at most 368 `i16` per column of the frame
/// padded to whole MCUs, which bounds every component's stride.
const SINGLE_SCAN_ROW_BYTES_PER_COLUMN: u64 = 1024;

/// Whether the first scan of `bytes` carries every frame component. zune-jpeg then converts each
/// MCU row into output pixels as it decodes it; a frame split over several scans holds the
/// coefficients of the whole image until its last scan.
pub(super) fn single_scan_frame(bytes: &[u8]) -> bool {
    let mut components = None;
    let mut position = 2;
    while bytes.get(position) == Some(&0xff) {
        let Some(&marker) = bytes.get(position + 1) else {
            return false;
        };
        match marker {
            0xc0 | 0xc1 => components = bytes.get(position + 9).copied(),
            0xda => return components.is_some() && bytes.get(position + 4).copied() == components,
            _ => {}
        }
        let Some(length) = bytes.get(position + 2..position + 4) else {
            return false;
        };
        position += 2 + usize::from(u16::from_be_bytes([length[0], length[1]]));
    }
    false
}

/// The frame size, RGBA raster bytes and zune-jpeg row-buffer bound of a JPEG [`decode_jpeg_into_rgba`]
/// can decode.
fn in_place_frame(bytes: &[u8]) -> std::result::Result<((u32, u32), u64, u64), String> {
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    decoder.read_info().map_err(|error| error.to_string())?;
    let info = decoder.info().ok_or_else(|| "missing JPEG frame header".to_string())?;
    if !matches!(
        info.pixel_format,
        jpeg_decoder::PixelFormat::L8 | jpeg_decoder::PixelFormat::RGB24
    ) {
        return Err("only 8-bit grayscale and RGB JPEG sources can be decoded in place".into());
    }
    let raster = u64::from(info.width) * u64::from(info.height) * 4;
    let rows = u64::from(info.width).div_ceil(MAX_MCU_COLUMNS)
        * MAX_MCU_COLUMNS
        * info.pixel_format.pixel_bytes() as u64
        * SINGLE_SCAN_ROW_BYTES_PER_COLUMN;
    Ok(((u32::from(info.width), u32::from(info.height)), raster, rows))
}

/// Bytes [`decode_jpeg_into_rgba`] itself allocates, the share of [`in_place_jpeg_image_bytes`]
/// beside the caller's encoded buffer and canvas: the RGBA raster, the decoder's row buffers, its
/// copy of the encoded bytes and the metadata it may copy out of them (at most the input).
pub(super) fn in_place_decode_bytes(bytes: &[u8]) -> std::result::Result<u64, String> {
    let (_, raster, rows) = in_place_frame(bytes)?;
    Ok(raster + rows + bytes.len() as u64 * 2)
}

/// The working set of [`decode_jpeg_into_rgba`] and the paint after it: the RGBA raster the samples
/// are decoded into, the larger of zune-jpeg's row buffers and what drawing it as `placement` says
/// needs, the encoded bytes, the decoder's copy of them and the metadata it may copy out of them (at
/// most the input), and the retained canvas.
fn in_place_jpeg_image_bytes(
    bytes: &[u8],
    encoded_capacity: usize,
    retained_canvas: u64,
    placement: Placement,
) -> std::result::Result<u64, String> {
    let (frame, raster, rows) = in_place_frame(bytes)?;
    let draw = paint_bytes(Some(placement), frame)?;
    Ok(raster
        .saturating_add(rows.max(draw))
        .saturating_add(encoded_capacity as u64)
        .saturating_add(bytes.len() as u64 * 2)
        .saturating_add(retained_canvas))
}

/// Decode a JPEG admitted as [`JpegDecode::InPlace`] at its full `frame` size straight into its RGBA
/// paint raster. The decoder writes its gray or RGB samples to the front of the RGBA buffer, which
/// is then widened from the back, so the decoded raster never coexists with a converted copy. The
/// decoder, its allocation limit and so the pixels are those of `image::load_from_memory` followed
/// by `to_rgba8`.
pub(super) fn decode_jpeg_into_rgba(bytes: &[u8], (width, height): (u32, u32)) -> Result<image::RgbaImage> {
    use image::ImageDecoder;
    let failed = |error: image::ImageError| Error::Decode(format!("Failed to decode JPEG in place: {error}"));
    let mut decoder = image::codecs::jpeg::JpegDecoder::new(std::io::Cursor::new(bytes)).map_err(failed)?;
    let mut limits = image::Limits::default();
    limits.reserve(decoder.total_bytes()).map_err(failed)?;
    decoder.set_limits(limits).map_err(failed)?;
    let channels = match decoder.color_type() {
        image::ColorType::L8 => 1,
        image::ColorType::Rgb8 => 3,
        other => return Err(Error::Decode(format!("JPEG decodes to {other:?}, not gray or RGB"))),
    };
    if decoder.dimensions() != (width, height) {
        let (decoded_width, decoded_height) = decoder.dimensions();
        return Err(Error::Decode(format!(
            "JPEG decodes at {decoded_width}x{decoded_height}, not the admitted {width}x{height}"
        )));
    }
    let pixels = usize::try_from(u64::from(width) * u64::from(height))
        .map_err(|_| Error::Decode("JPEG raster exceeds the address space".into()))?;
    let rgba_len = pixels
        .checked_mul(4)
        .ok_or_else(|| Error::Decode("JPEG raster exceeds the address space".into()))?;
    let mut raster = vec![0; rgba_len];
    decoder.read_image(&mut raster[..pixels * channels]).map_err(failed)?;
    widen_to_rgba(&mut raster, channels);
    image::RgbaImage::from_raw(width, height, raster)
        .ok_or_else(|| Error::Decode("in-place JPEG decoded to an unexpected pixel layout".into()))
}

/// Widen `channels`-byte gray or RGB samples packed at the front of `raster` into opaque RGBA over
/// the whole of it. Back to front, pixel `i` lands at `4 * i`, at or after every sample of pixels
/// `0..=i`, so no sample is overwritten before it is read.
fn widen_to_rgba(raster: &mut [u8], channels: usize) {
    for index in (0..raster.len() / 4).rev() {
        let source = index * channels;
        let pixel = if channels == 1 {
            [raster[source], raster[source], raster[source], u8::MAX]
        } else {
            [raster[source], raster[source + 1], raster[source + 2], u8::MAX]
        };
        raster[index * 4..index * 4 + 4].copy_from_slice(&pixel);
    }
}
