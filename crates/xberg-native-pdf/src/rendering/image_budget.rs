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
        let failure = match jpeg_image_bytes(image, bytes, bytes.capacity(), retained_canvas) {
            Ok(required) if required <= self.limit as u64 => return Ok(()),
            Ok(required) => Failure::Exceeded {
                required,
                limit: self.limit,
            },
            Err(error) => Failure::Unqualified(error),
        };
        self.failure.get_or_insert_with(|| failure.clone());
        Err(failure.error())
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
    // their actual decoded raster while to_rgba8 allocates the paint raster. The
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
        .and_then(|sum| sum.checked_add(rgba_pixels.checked_mul(4)?))
        .and_then(|sum| sum.checked_add(encoded_capacity as u64))
        .and_then(|sum| sum.checked_add(bytes.len() as u64))
        .and_then(|sum| sum.checked_add(retained_canvas))
        .ok_or_else(|| "JPEG source image workspace arithmetic overflow".into())
}
