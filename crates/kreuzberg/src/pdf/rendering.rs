use super::bindings::{PdfiumHandle, bind_pdfium};
use super::error::{PdfError, Result};
use image::{DynamicImage, GenericImageView};
use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};

const PDF_POINTS_PER_INCH: f64 = 72.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRenderOptions {
    pub target_dpi: i32,
    pub max_image_dimension: i32,
    pub auto_adjust_dpi: bool,
    pub min_dpi: i32,
    pub max_dpi: i32,
}

impl Default for PageRenderOptions {
    fn default() -> Self {
        Self {
            target_dpi: 300,
            max_image_dimension: 65536,
            auto_adjust_dpi: true,
            min_dpi: 72,
            max_dpi: 600,
        }
    }
}

pub struct PdfRenderer<'a> {
    pdfium: PdfiumHandle<'a>,
}

impl PdfRenderer<'static> {
    pub fn new() -> Result<Self> {
        let pdfium = bind_pdfium(PdfError::RenderingFailed, "page rendering", None)?;
        Ok(Self { pdfium })
    }
}

impl PdfRenderer<'_> {
    /// Return the number of pages in the given PDF without rendering.
    pub fn page_count(&self, pdf_bytes: &[u8]) -> Result<usize> {
        let document = self
            .pdfium
            .load_pdf_from_byte_slice(pdf_bytes, None)
            .map_err(|e| PdfError::InvalidPdf(super::error::format_pdfium_error(e)))?;
        Ok(document.pages().len() as usize)
    }

    pub fn render_page_to_image(
        &self,
        pdf_bytes: &[u8],
        page_index: usize,
        options: &PageRenderOptions,
    ) -> Result<DynamicImage> {
        self.render_page_to_image_with_password(pdf_bytes, page_index, options, None)
    }

    pub fn render_page_to_image_with_password(
        &self,
        pdf_bytes: &[u8],
        page_index: usize,
        options: &PageRenderOptions,
        password: Option<&str>,
    ) -> Result<DynamicImage> {
        let document = self
            .pdfium
            .load_pdf_from_byte_slice(pdf_bytes, password)
            .map_err(|e| super::error::classify_pdfium_load_error(e, password))?;

        self.render_page_from_document(&document, page_index, options)
    }

    fn render_page_from_document(
        &self,
        document: &PdfDocument,
        page_index: usize,
        options: &PageRenderOptions,
    ) -> Result<DynamicImage> {
        let page_i32 = i32::try_from(page_index).map_err(|_| PdfError::PageNotFound(page_index))?;
        let page = document
            .pages()
            .get(page_i32)
            .map_err(|_| PdfError::PageNotFound(page_index))?;

        let width_points = page.width().value;
        let height_points = page.height().value;

        let dpi = if options.auto_adjust_dpi {
            calculate_optimal_dpi(
                width_points as f64,
                height_points as f64,
                options.target_dpi,
                options.max_image_dimension,
                options.min_dpi,
                options.max_dpi,
            )
        } else {
            options.target_dpi
        };

        let scale = dpi as f64 / PDF_POINTS_PER_INCH;

        let config = PdfRenderConfig::new()
            .set_target_width(((width_points * scale as f32) as i32).max(1))
            .set_target_height(((height_points * scale as f32) as i32).max(1))
            .rotate_if_landscape(PdfPageRenderRotation::None, false);

        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| PdfError::RenderingFailed(format!("Failed to render page: {}", e)))?;

        let image = bitmap
            .as_image()
            .map_err(|e| PdfError::RenderingFailed(format!("Failed to convert bitmap to image: {}", e)))?
            .into_rgb8();

        Ok(DynamicImage::ImageRgb8(image))
    }

    pub fn render_all_pages(&self, pdf_bytes: &[u8], options: &PageRenderOptions) -> Result<Vec<DynamicImage>> {
        self.render_all_pages_with_password(pdf_bytes, options, None)
    }

    pub fn render_all_pages_with_password(
        &self,
        pdf_bytes: &[u8],
        options: &PageRenderOptions,
        password: Option<&str>,
    ) -> Result<Vec<DynamicImage>> {
        let document = self
            .pdfium
            .load_pdf_from_byte_slice(pdf_bytes, password)
            .map_err(|e| super::error::classify_pdfium_load_error(e, password))?;

        let page_count = document.pages().len() as usize;
        let mut images = Vec::with_capacity(page_count);

        for page_index in 0..page_count {
            let image = self.render_page_from_document(&document, page_index, options)?;
            images.push(image);
        }

        Ok(images)
    }
}

pub fn render_page_to_image(pdf_bytes: &[u8], page_index: usize, options: &PageRenderOptions) -> Result<DynamicImage> {
    let renderer = PdfRenderer::new()?;
    renderer.render_page_to_image(pdf_bytes, page_index, options)
}

/// Default DPI for page rendering. 150 balances legibility for vision-model
/// OCR against memory and file size: a standard letter page renders to
/// ~1275x1650 px (~2 MB PNG), versus ~2550x3300 px (~6 MB) at 300 DPI.
const DEFAULT_RENDER_DPI: i32 = 150;

/// Render a single PDF page to a PNG-encoded byte buffer.
///
/// # Errors
///
/// Returns an error if the PDF is invalid, the page index is out of bounds,
/// or if the page fails to render.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::pdf::render_pdf_page_to_png;
///
/// # fn example() -> kreuzberg::pdf::error::Result<()> {
/// let pdf_bytes = std::fs::read("document.pdf")?;
/// let png = render_pdf_page_to_png(&pdf_bytes, 0, Some(150), None)?;
/// std::fs::write("page_0.png", png)?;
/// # Ok(())
/// # }
/// ```
pub fn render_pdf_page_to_png(
    pdf_bytes: &[u8],
    page_index: usize,
    dpi: Option<i32>,
    password: Option<&str>,
) -> Result<Vec<u8>> {
    let renderer = PdfRenderer::new()?;
    let options = PageRenderOptions {
        target_dpi: dpi.unwrap_or(DEFAULT_RENDER_DPI),
        ..PageRenderOptions::default()
    };
    let image = renderer.render_page_to_image_with_password(pdf_bytes, page_index, &options, password)?;
    encode_png(&image)
}

/// Lazy page-by-page PDF renderer.
///
/// Reads the file once at construction and yields one PNG-encoded page per
/// `next()` call. Only one rendered page is held in memory at a time.
///
/// The PDFium mutex is acquired and released per page, so other PDF
/// operations can proceed between iterations. This makes the iterator
/// safe to use in long-running loops (e.g., sending each page to a vision
/// model for OCR) without blocking all PDF processing.
///
/// Use the iterator when memory is a concern or when you want to process
/// pages as they are rendered.
///
/// # Example
///
/// ```rust,no_run
/// use kreuzberg::pdf::PdfPageIterator;
///
/// # fn example() -> kreuzberg::pdf::error::Result<()> {
/// let iter = PdfPageIterator::from_file("document.pdf", Some(150), None)?;
/// println!("Rendering {} pages", iter.page_count());
/// for result in iter {
///     let (page_index, png) = result?;
///     std::fs::write(format!("page_{page_index}.png"), png)?;
/// }
/// # Ok(())
/// # }
/// ```
pub struct PdfPageIterator {
    pdf_bytes: Vec<u8>,
    password: Option<String>,
    page_count: usize,
    current_page: usize,
    options: PageRenderOptions,
}

impl PdfPageIterator {
    /// Create an iterator from raw PDF bytes.
    ///
    /// Validates the PDF and determines the page count. The PDF bytes are
    /// owned by the iterator — the file is not re-read from disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the PDF is invalid or password-protected without
    /// the correct password.
    pub fn new(pdf_bytes: Vec<u8>, dpi: Option<i32>, password: Option<String>) -> Result<Self> {
        // Validate PDF and get page count (acquires + releases mutex)
        let renderer = PdfRenderer::new()?;
        let pw = password.as_deref();
        let document = renderer
            .pdfium
            .load_pdf_from_byte_slice(&pdf_bytes, pw)
            .map_err(|e| super::error::classify_pdfium_load_error(e, pw))?;
        let page_count = document.pages().len() as usize;
        drop(document);
        drop(renderer); // release mutex immediately
        Ok(Self {
            pdf_bytes,
            password,
            page_count,
            current_page: 0,
            options: PageRenderOptions {
                target_dpi: dpi.unwrap_or(DEFAULT_RENDER_DPI),
                ..PageRenderOptions::default()
            },
        })
    }

    /// Create an iterator from a file path.
    ///
    /// Reads the file into memory once. Subsequent iterations render from
    /// the owned bytes without re-reading the file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the PDF is invalid.
    pub fn from_file(path: impl AsRef<std::path::Path>, dpi: Option<i32>, password: Option<String>) -> Result<Self> {
        let pdf_bytes =
            std::fs::read(path.as_ref()).map_err(|e| PdfError::IOError(format!("Failed to read file: {}", e)))?;
        Self::new(pdf_bytes, dpi, password)
    }

    /// Number of pages in the PDF.
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    fn render_page(&self, page_index: usize) -> Result<Vec<u8>> {
        // Acquire mutex, load document from owned bytes, render, release
        let renderer = PdfRenderer::new()?;
        let pw = self.password.as_deref();
        let document = renderer
            .pdfium
            .load_pdf_from_byte_slice(&self.pdf_bytes, pw)
            .map_err(|e| super::error::classify_pdfium_load_error(e, pw))?;
        let image = renderer.render_page_from_document(&document, page_index, &self.options)?;
        encode_png(&image)
        // renderer dropped here → mutex released
    }
}

impl Iterator for PdfPageIterator {
    type Item = Result<(usize, Vec<u8>)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_page >= self.page_count {
            return None;
        }
        let page_index = self.current_page;
        self.current_page += 1;
        Some(self.render_page(page_index).map(|png| (page_index, png)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.page_count - self.current_page;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PdfPageIterator {}

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>> {
    let (w, h) = image.dimensions();
    // Raw RGB is w*h*3 bytes; PNG compresses ~50%, plus header overhead
    let estimated = (w as usize * h as usize * 3) / 2;
    let mut buf = std::io::Cursor::new(Vec::with_capacity(estimated));
    image
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| PdfError::RenderingFailed(format!("PNG encoding failed: {}", e)))?;
    Ok(buf.into_inner())
}

fn calculate_optimal_dpi(
    page_width: f64,
    page_height: f64,
    target_dpi: i32,
    max_dimension: i32,
    min_dpi: i32,
    max_dpi: i32,
) -> i32 {
    let width_inches = page_width / PDF_POINTS_PER_INCH;
    let height_inches = page_height / PDF_POINTS_PER_INCH;

    let width_at_target = (width_inches * target_dpi as f64) as i32;
    let height_at_target = (height_inches * target_dpi as f64) as i32;

    if width_at_target <= max_dimension && height_at_target <= max_dimension {
        return target_dpi.clamp(min_dpi, max_dpi);
    }

    let width_limited_dpi = (max_dimension as f64 / width_inches) as i32;
    let height_limited_dpi = (max_dimension as f64 / height_inches) as i32;

    width_limited_dpi.min(height_limited_dpi).clamp(min_dpi, max_dpi)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_renderer_creation() {
        let result = PdfRenderer::new();
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_render_invalid_pdf() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let result = renderer.render_page_to_image(b"not a pdf", 0, &options);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PdfError::InvalidPdf(_)));
    }

    #[test]
    #[serial]
    fn test_render_page_not_found() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let minimal_pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n";
        let result = renderer.render_page_to_image(minimal_pdf, 999, &options);

        if let Err(err) = result {
            assert!(matches!(
                err,
                PdfError::PageNotFound(_) | PdfError::InvalidPdf(_) | PdfError::PasswordRequired
            ));
        }
    }

    #[test]
    fn test_calculate_optimal_dpi_within_limits() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, 300, 65536, 72, 600);
        assert!((72..=600).contains(&dpi));
    }

    #[test]
    fn test_calculate_optimal_dpi_oversized_page() {
        let dpi = calculate_optimal_dpi(10000.0, 10000.0, 300, 4096, 72, 600);
        assert!(dpi >= 72);
        assert!(dpi < 300);
    }

    #[test]
    fn test_calculate_optimal_dpi_min_clamp() {
        let dpi = calculate_optimal_dpi(100.0, 100.0, 10, 65536, 72, 600);
        assert_eq!(dpi, 72);
    }

    #[test]
    fn test_calculate_optimal_dpi_max_clamp() {
        let dpi = calculate_optimal_dpi(100.0, 100.0, 1000, 65536, 72, 600);
        assert_eq!(dpi, 600);
    }

    #[test]
    fn test_page_render_options_default() {
        let options = PageRenderOptions::default();
        assert_eq!(options.target_dpi, 300);
        assert_eq!(options.max_image_dimension, 65536);
        assert!(options.auto_adjust_dpi);
        assert_eq!(options.min_dpi, 72);
        assert_eq!(options.max_dpi, 600);
    }

    #[test]
    fn test_renderer_size() {
        use std::mem::size_of;
        let _size = size_of::<PdfRenderer>();
    }

    #[test]
    #[serial]
    fn test_render_page_with_password_none() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let result = renderer.render_page_to_image_with_password(b"not a pdf", 0, &options, None);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_render_all_pages_with_password_none() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let result = renderer.render_all_pages_with_password(b"not a pdf", &options, None);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_render_page_to_image_function() {
        let options = PageRenderOptions::default();
        let result = render_page_to_image(b"not a pdf", 0, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_optimal_dpi_tall_page() {
        let dpi = calculate_optimal_dpi(612.0, 10000.0, 300, 4096, 72, 600);
        assert!((72..=600).contains(&dpi));
    }

    #[test]
    fn test_calculate_optimal_dpi_wide_page() {
        let dpi = calculate_optimal_dpi(10000.0, 612.0, 300, 4096, 72, 600);
        assert!((72..=600).contains(&dpi));
    }

    #[test]
    fn test_calculate_optimal_dpi_square_page() {
        let dpi = calculate_optimal_dpi(1000.0, 1000.0, 300, 65536, 72, 600);
        assert!((72..=600).contains(&dpi));
    }

    #[test]
    fn test_calculate_optimal_dpi_tiny_page() {
        let dpi = calculate_optimal_dpi(72.0, 72.0, 300, 65536, 72, 600);
        assert_eq!(dpi, 300);
    }

    #[test]
    fn test_calculate_optimal_dpi_target_equals_max() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, 600, 65536, 72, 600);
        assert_eq!(dpi, 600);
    }

    #[test]
    fn test_calculate_optimal_dpi_target_equals_min() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, 72, 65536, 72, 600);
        assert_eq!(dpi, 72);
    }

    #[test]
    fn test_calculate_optimal_dpi_exactly_at_limit() {
        let page_size = 65536.0 / 300.0 * PDF_POINTS_PER_INCH;
        let dpi = calculate_optimal_dpi(page_size, page_size, 300, 65536, 72, 600);
        assert!((72..=600).contains(&dpi));
    }

    #[test]
    fn test_page_render_options_custom() {
        let options = PageRenderOptions {
            target_dpi: 150,
            max_image_dimension: 8192,
            auto_adjust_dpi: false,
            min_dpi: 50,
            max_dpi: 400,
        };

        assert_eq!(options.target_dpi, 150);
        assert_eq!(options.max_image_dimension, 8192);
        assert!(!options.auto_adjust_dpi);
        assert_eq!(options.min_dpi, 50);
        assert_eq!(options.max_dpi, 400);
    }

    #[test]
    fn test_page_render_options_clone() {
        let options1 = PageRenderOptions::default();
        let options2 = options1.clone();

        assert_eq!(options1.target_dpi, options2.target_dpi);
        assert_eq!(options1.max_image_dimension, options2.max_image_dimension);
        assert_eq!(options1.auto_adjust_dpi, options2.auto_adjust_dpi);
    }

    #[test]
    fn test_pdf_points_per_inch_constant() {
        assert_eq!(PDF_POINTS_PER_INCH, 72.0);
    }

    #[test]
    #[serial]
    fn test_render_empty_bytes() {
        let renderer = PdfRenderer::new().unwrap();
        let options = PageRenderOptions::default();
        let result = renderer.render_page_to_image(&[], 0, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_optimal_dpi_zero_target() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, 0, 65536, 72, 600);
        assert_eq!(dpi, 72);
    }

    #[test]
    fn test_calculate_optimal_dpi_negative_target() {
        let dpi = calculate_optimal_dpi(612.0, 792.0, -100, 65536, 72, 600);
        assert_eq!(dpi, 72);
    }

    #[test]
    #[serial]
    fn test_render_pdf_page_to_png_invalid() {
        let result = render_pdf_page_to_png(b"not a pdf", 0, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_png() {
        let image = DynamicImage::new_rgb8(1, 1);
        let png_bytes = encode_png(&image).expect("encoding a 1x1 image should succeed");
        assert!(png_bytes.len() >= 8, "PNG output too short");
        assert_eq!(&png_bytes[..4], &[0x89, 0x50, 0x4E, 0x47], "missing PNG magic bytes");
    }

    fn load_test_pdf() -> Vec<u8> {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/ocr_test_rotated_90.pdf");
        std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read test PDF at {}: {}", path.display(), e))
    }

    const PNG_MAGIC: [u8; 4] = [0x89, 0x50, 0x4E, 0x47];

    #[test]
    #[serial]
    fn test_render_pdf_page_to_png_success() {
        let pdf_bytes = load_test_pdf();
        let png = render_pdf_page_to_png(&pdf_bytes, 0, None, None).expect("render page 0 should succeed");
        assert!(png.len() >= 4 && png[..4] == PNG_MAGIC, "missing PNG magic bytes");
    }

    #[test]
    #[serial]
    fn test_render_pdf_page_out_of_bounds() {
        let pdf_bytes = load_test_pdf();
        let result = render_pdf_page_to_png(&pdf_bytes, 999, None, None);
        assert!(result.is_err(), "page 999 should be out of bounds");
        assert!(
            matches!(result.unwrap_err(), PdfError::PageNotFound(999)),
            "expected PageNotFound(999)"
        );
    }

    #[test]
    #[serial]
    fn test_render_pdf_custom_dpi() {
        let pdf_bytes = load_test_pdf();
        let png_72 = render_pdf_page_to_png(&pdf_bytes, 0, Some(72), None).expect("DPI=72 should succeed");
        let png_300 = render_pdf_page_to_png(&pdf_bytes, 0, Some(300), None).expect("DPI=300 should succeed");
        assert!(
            png_72.len() >= 4 && png_72[..4] == PNG_MAGIC,
            "DPI=72 missing PNG magic bytes"
        );
        assert!(
            png_300.len() >= 4 && png_300[..4] == PNG_MAGIC,
            "DPI=300 missing PNG magic bytes"
        );
        assert_ne!(
            png_72.len(),
            png_300.len(),
            "different DPI should produce different sized output"
        );
    }

    #[test]
    #[serial]
    fn test_page_count() {
        let pdf_bytes = load_test_pdf();
        let renderer = PdfRenderer::new().unwrap();
        let count = renderer.page_count(&pdf_bytes).expect("page_count should succeed");
        assert!(count >= 1, "test PDF should have at least 1 page");
    }

    #[test]
    #[serial]
    fn test_render_pdf_page_very_large_index() {
        let pdf_bytes = load_test_pdf();
        let result = render_pdf_page_to_png(&pdf_bytes, usize::MAX, None, None);
        assert!(result.is_err(), "usize::MAX page index should fail");
    }

    #[test]
    #[serial]
    fn test_pdf_page_iterator_success() {
        let pdf_bytes = load_test_pdf();
        // Get expected count from the iterator itself to avoid holding two pdfium handles
        let iter = PdfPageIterator::new(pdf_bytes, None, None).expect("iterator creation should succeed");
        let expected_count = iter.page_count();
        let mut count = 0;
        for result in iter {
            let (page_index, png) = result.expect("each page should render successfully");
            assert_eq!(page_index, count, "page index should match iteration order");
            assert!(
                png.len() >= 4 && png[..4] == PNG_MAGIC,
                "page {} missing PNG magic bytes",
                page_index
            );
            count += 1;
        }
        assert_eq!(count, expected_count, "iterator should yield all pages");
    }

    #[test]
    #[serial]
    fn test_pdf_page_iterator_from_file() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf/ocr_test_rotated_90.pdf");
        let iter = PdfPageIterator::from_file(&path, None, None).expect("from_file should succeed");
        let mut count = 0;
        for result in iter {
            let (_page_index, png) = result.expect("each page should render successfully");
            assert!(
                png.len() >= 4 && png[..4] == PNG_MAGIC,
                "page {} missing PNG magic bytes",
                count
            );
            count += 1;
        }
        assert!(count >= 1, "should produce at least one page");
    }

    #[test]
    #[serial]
    fn test_pdf_page_iterator_page_count() {
        let pdf_bytes = load_test_pdf();
        let iter = PdfPageIterator::new(pdf_bytes, None, None).expect("iterator creation should succeed");
        let page_count = iter.page_count();
        let actual_count = iter.count();
        assert_eq!(
            page_count, actual_count,
            "page_count() should match actual iteration count"
        );
    }

    #[test]
    #[serial]
    fn test_pdf_page_iterator_size_hint() {
        let pdf_bytes = load_test_pdf();
        let mut iter = PdfPageIterator::new(pdf_bytes, None, None).expect("iterator creation should succeed");
        let total = iter.page_count();
        assert_eq!(
            iter.size_hint(),
            (total, Some(total)),
            "initial size_hint should equal page_count"
        );

        iter.next();
        let remaining = total - 1;
        assert_eq!(
            iter.size_hint(),
            (remaining, Some(remaining)),
            "size_hint should decrease after next()"
        );
    }

    #[test]
    #[serial]
    fn test_pdf_page_iterator_invalid_pdf() {
        let result = PdfPageIterator::new(b"not a pdf".to_vec(), None, None);
        assert!(result.is_err(), "invalid PDF bytes should return an error");
    }

    #[test]
    #[serial]
    fn test_pdf_page_iterator_early_drop() {
        let pdf_bytes = load_test_pdf();
        let mut iter = PdfPageIterator::new(pdf_bytes, None, None).expect("iterator creation should succeed");
        let first = iter.next();
        assert!(first.is_some(), "should yield at least one page");
        let (idx, png) = first.unwrap().expect("first page should render");
        assert_eq!(idx, 0);
        assert!(png.len() >= 4 && png[..4] == PNG_MAGIC);
        drop(iter); // should not crash or leak
    }
}
