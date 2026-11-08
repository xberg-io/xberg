use crate::error::{Error, Result};
use crate::extractors::images::{ImageData, PdfImage};

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

impl ImageBudget {
    pub(super) fn new(limit: usize) -> Self {
        Self { limit, failure: None }
    }

    pub(super) fn admit_jpeg(&mut self, image: &PdfImage, retained_canvas: u64) -> Result<()> {
        let ImageData::Jpeg(bytes) = image.data() else {
            return Ok(());
        };
        self.admit_required(jpeg_image_bytes(image, bytes, bytes.capacity(), retained_canvas, 4))
    }

    /// Admit `image`, or the IDCT-scaled decode that keeps its rasters within the budget when the
    /// full-size decode does not. Returns the scale denominator to decode at (1, 2, 4 or 8): the
    /// finest scale that fits, provided the reduced raster still covers the `device` extent (in canvas
    /// pixels) the image is painted at, so reducing never costs rendered resolution. An image that
    /// cannot be reduced that way is refused exactly as [`Self::admit_jpeg`] refuses it.
    pub(super) fn admit_jpeg_reducing(
        &mut self,
        image: &PdfImage,
        retained_canvas: u64,
        device: (f32, f32),
    ) -> Result<u8> {
        let ImageData::Jpeg(bytes) = image.data() else {
            return Ok(1);
        };
        let capacity = bytes.capacity();
        let full = jpeg_image_bytes(image, bytes, capacity, retained_canvas, 4);
        if full.as_ref().is_ok_and(|required| *required <= self.limit as u64) {
            return Ok(1);
        }
        if reducible_jpeg(image, bytes) {
            for denominator in REDUCED_DENOMINATORS {
                if !covers_device(image, denominator, device) {
                    break;
                }
                let reduced = reduced_jpeg_image_bytes(bytes, capacity, retained_canvas, denominator);
                if reduced.is_ok_and(|required| required <= self.limit as u64) {
                    return Ok(denominator);
                }
            }
        }
        self.admit_required(full).map(|()| 1)
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
        self.admit_required(retained.and_then(|retained| jpeg_image_bytes(image, bytes, bytes.capacity(), retained, 1)))
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
    let rgba_pixels = if cmyk {
        u64::from(image.width()) * u64::from(image.height())
    } else {
        pixels
    };
    decoded
        .checked_add(converted)
        .and_then(|sum| sum.checked_add(rgba_pixels.checked_mul(paint_components)?))
        .and_then(|sum| sum.checked_add(encoded_capacity as u64))
        .and_then(|sum| sum.checked_add(bytes.len() as u64))
        .and_then(|sum| sum.checked_add(retained_canvas))
        .ok_or_else(|| "JPEG source image workspace arithmetic overflow".into())
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

/// Whether `bytes` is an 8-bit sequential RGB or grayscale JPEG whose dictionary agrees with its
/// frame. A progressive scan keeps every coefficient of the full-resolution image, so a reduced
/// decode would not lower its peak; CMYK takes a separate conversion path.
fn reducible_jpeg(image: &PdfImage, bytes: &[u8]) -> bool {
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

/// Whether the raster of `image` at `denominator` has at least the canvas pixels it is painted over.
fn covers_device(image: &PdfImage, denominator: u8, (width, height): (f32, f32)) -> bool {
    let covers = |pixels: u32, painted: f32| {
        painted.is_finite() && reduced_dimension(pixels, denominator) as f32 >= painted - 1.0
    };
    covers(image.width(), width) && covers(image.height(), height)
}

fn reduced_dimension(dimension: u32, denominator: u8) -> u64 {
    u64::from(dimension).div_ceil(u64::from(denominator))
}

/// The JPEG working set at `denominator`: the larger of the paint phase (scaled raster plus its
/// RGBA conversion) and the decode phase (component planes, output and one MCU row of
/// coefficients), on top of the encoded bytes and the retained canvas.
fn reduced_jpeg_image_bytes(
    bytes: &[u8],
    encoded_capacity: usize,
    retained_canvas: u64,
    denominator: u8,
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
    let pixels = reduced_dimension(u32::from(info.width), denominator)
        .checked_mul(reduced_dimension(u32::from(info.height), denominator))
        .ok_or("reduced JPEG raster arithmetic overflow")?;
    let components = info.pixel_format.pixel_bytes() as u64;
    let decoded = pixels * components;
    let paint = decoded.checked_add(pixels * 4);
    let coefficients = u64::from(info.width)
        .div_ceil(8)
        .checked_mul(components * MAX_VERTICAL_BLOCKS * 2 * COEFFICIENT_BLOCK_BYTES);
    let decoding = decoded
        .checked_mul(2)
        .and_then(|planes_and_output| planes_and_output.checked_add(coefficients?));
    paint
        .zip(decoding)
        .map(|(paint, decoding)| paint.max(decoding))
        .and_then(|sum| sum.checked_add(encoded_capacity as u64))
        .and_then(|sum| sum.checked_add(bytes.len() as u64))
        .and_then(|sum| sum.checked_add(retained_canvas))
        .ok_or_else(|| "reduced JPEG source image workspace arithmetic overflow".into())
}

/// Decode a JPEG admitted by [`ImageBudget::admit_jpeg_reducing`] at `denominator`, never holding
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
    let output = usize::from(width) * usize::from(height) * info.pixel_format.pixel_bytes();
    decoder.set_max_decoding_buffer_size(output);
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
