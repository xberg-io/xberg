use std::borrow::Cow;
use std::io::{Cursor, Read};

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use xberg::extractors::security::SecurityLimits;
use xberg::pdf::render::PdfRenderSession;

use super::{DetectorMeasurement, benchmark_error};
use crate::corpus::CorpusDocument;
use crate::{Error, Result};

enum InputFormat {
    Pdf,
    Raster(ImageFormat),
}

impl InputFormat {
    fn parse(file_type: &str) -> Result<Self> {
        match file_type {
            "pdf" => Ok(Self::Pdf),
            "png" => Ok(Self::Raster(ImageFormat::Png)),
            "jpeg" | "jpg" => Ok(Self::Raster(ImageFormat::Jpeg)),
            _ => Err(Error::Config(
                "detector cohort supports only PDF, PNG and JPEG fixtures".to_string(),
            )),
        }
    }
}

pub(super) fn validate_file_type(file_type: &str) -> Result<()> {
    InputFormat::parse(file_type).map(|_| ())
}

pub(super) fn read_document(document: &CorpusDocument) -> Result<Vec<u8>> {
    if matches!(InputFormat::parse(&document.file_type)?, InputFormat::Pdf) {
        return Ok(std::fs::read(&document.document_path)?);
    }
    let limit = image_budget();
    let file = std::fs::File::open(&document.document_path)?;
    if file.metadata()?.len() > limit {
        return Err(image_budget_error());
    }
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(image_budget_error());
    }
    Ok(bytes)
}

pub(super) enum PreparedInput<'a> {
    Pdf(Box<PdfRenderSession>),
    Raster { bytes: &'a [u8], width: u32, height: u32 },
}

pub(super) struct InputPage<'a> {
    pub bytes: Cow<'a, [u8]>,
    pub width: u32,
    pub height: u32,
}

impl InputPage<'_> {
    pub(super) fn validate_measurements(&self, measurements: &[DetectorMeasurement; 2]) -> Result<()> {
        if measurements
            .iter()
            .any(|measurement| measurement.page_width != self.width || measurement.page_height != self.height)
        {
            return Err(Error::Benchmark(
                "detector result dimensions do not match input pixels".to_string(),
            ));
        }
        Ok(())
    }
}

impl<'a> PreparedInput<'a> {
    pub(super) fn open(bytes: &'a [u8], file_type: &str) -> Result<Self> {
        match InputFormat::parse(file_type)? {
            InputFormat::Pdf => PdfRenderSession::open(bytes, None)
                .map(Box::new)
                .map(Self::Pdf)
                .map_err(benchmark_error),
            InputFormat::Raster(format) => {
                let (width, height) = decode_raster_dimensions(bytes, format)?;
                Ok(Self::Raster { bytes, width, height })
            }
        }
    }

    pub(super) fn page_count(&self) -> usize {
        match self {
            Self::Pdf(session) => session.page_count(),
            Self::Raster { .. } => 1,
        }
    }

    pub(super) fn page(&self, page: usize, dpi: i32) -> Result<InputPage<'_>> {
        if page >= self.page_count() {
            return Err(Error::Benchmark(
                "detector page index exceeds prepared input".to_string(),
            ));
        }
        match self {
            Self::Pdf(session) => {
                let bytes = session.render_page_to_png(page, Some(dpi)).map_err(benchmark_error)?;
                let (width, height) = ImageReader::with_format(Cursor::new(&bytes), ImageFormat::Png)
                    .into_dimensions()
                    .map_err(benchmark_error)?;
                Ok(InputPage {
                    bytes: Cow::Owned(bytes),
                    width,
                    height,
                })
            }
            Self::Raster { bytes, width, height } => Ok(InputPage {
                bytes: Cow::Borrowed(bytes),
                width: *width,
                height: *height,
            }),
        }
    }
}

fn image_budget() -> u64 {
    SecurityLimits::default().max_content_size as u64
}

fn image_budget_error() -> Error {
    Error::Benchmark("detector raster exceeds security_limits.max_content_size".to_string())
}

fn raster_error(error: impl std::fmt::Display) -> Error {
    Error::Benchmark(format!("invalid detector raster: {error}"))
}

fn image_error(error: image::ImageError) -> Error {
    match error {
        image::ImageError::Limits(_) => image_budget_error(),
        _ => raster_error(error),
    }
}

fn decode_raster_dimensions(bytes: &[u8], expected_format: ImageFormat) -> Result<(u32, u32)> {
    let budget = image_budget();
    if bytes.len() as u64 > budget {
        return Err(image_budget_error());
    }
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(raster_error)?;
    let format = reader
        .format()
        .ok_or_else(|| raster_error("unknown encoded image format"))?;
    if format != expected_format {
        return Err(Error::Config(
            "detector raster format does not match fixture type".to_string(),
        ));
    }
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(budget);
    reader.limits(limits);
    let decoder = reader.into_decoder().map_err(image_error)?;
    let (width, height) = decoder.dimensions();
    let live_bytes = decoder.total_bytes().checked_add(bytes.len() as u64);
    if width == 0 || height == 0 || live_bytes.is_none_or(|count| count > budget) {
        return Err(image_budget_error());
    }
    let image = DynamicImage::from_decoder(decoder).map_err(image_error)?;
    Ok((image.width(), image.height()))
}
