/// Fraction of the page a painted image must cover for its scale to stand for the page's scan.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const PAGE_COVERING_MIN: f32 = 0.80;
/// Largest relative difference between the horizontal and vertical resolution of an image
/// whose placement still counts as proven.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const SOURCE_DPI_ANISOTROPY_MAX: f64 = 0.01;

/// Points per inch: the unit of a placed extent, and the resolution below which the unknown-DPI
/// estimator never goes (it caps its upscale at `target_dpi / 72`), so a smaller one is not a scan
/// resolution but a thumbnail stretched over the page.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const POINTS_PER_INCH: f64 = 72.0;

/// A page-covering image whose placement fixes its resolution, with the pixel size the
/// dictionary declared for it.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProvenPlacement {
    dpi: f64,
    declared_width: u32,
    declared_height: u32,
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
impl ProvenPlacement {
    /// The proven resolution, only when the bytes handed to the backend have the pixel size the
    /// dictionary declared. A dictionary that disagrees with its stream proves nothing about the
    /// stream the backend decodes. JPEG 2000 is not read here, so it never confirms.
    pub(crate) fn confirmed_by(&self, format: &str, bytes: &[u8]) -> Option<f64> {
        let format = match format {
            "jpeg" => image::ImageFormat::Jpeg,
            "png" => image::ImageFormat::Png,
            _ => return None,
        };
        let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes));
        reader.set_format(format);
        let (width, height) = reader.into_dimensions().ok()?;
        (width == self.declared_width && height == self.declared_height).then_some(self.dpi)
    }
}

/// The resolution an image of `width` x `height` pixels painted at `width_pt` x `height_pt`
/// points has, when both axes agree within [`SOURCE_DPI_ANISOTROPY_MAX`] and it is at least
/// [`POINTS_PER_INCH`].
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn placement_dpi(width: u32, height: u32, width_pt: f64, height_pt: f64) -> Option<f64> {
    let across = f64::from(width) * POINTS_PER_INCH / width_pt;
    let down = f64::from(height) * POINTS_PER_INCH / height_pt;
    let usable = across.is_finite()
        && down.is_finite()
        && across > 0.0
        && down > 0.0
        && (across - down).abs() <= across.max(down) * SOURCE_DPI_ANISOTROPY_MAX;
    let dpi = (across + down) / 2.0;
    (usable && dpi >= POINTS_PER_INCH).then_some(dpi)
}

/// Placement-proven resolution, by paint order, of each page-covering image whose placement is
/// exactly known.
///
/// An image counts only when its matrix is an unsheared quarter-turn multiple (see
/// `PdfImageHandle::placed_extent_points`), its pixel size over its painted size gives the same
/// resolution along both axes (see [`placement_dpi`]), and its painted area covers at least
/// [`PAGE_COVERING_MIN`] of the media box. Any other image is absent, and the caller then leaves
/// the resolution unknown. `paint_order` is unique per image on a page.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn proven_source_dpis(
    doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
) -> std::collections::HashMap<usize, ProvenPlacement> {
    let mut proven = std::collections::HashMap::new();
    // Placed extents are in default user space units; only a unit of exactly one point makes
    // them comparable to the 72-per-inch constant below, so other pages prove nothing. ~keep
    if !doc
        .get_page_user_unit(page_idx)
        .is_ok_and(|unit| (unit - 1.0).abs() < f64::EPSILON)
    {
        return proven;
    }
    let Ok((x0, y0, x1, y1)) = doc.get_page_media_box(page_idx) else {
        return proven;
    };
    let page_area = ((x1 - x0) * (y1 - y0)).abs();
    let Ok(handles) = doc.page_image_handles(page_idx) else {
        return proven;
    };
    if !page_area.is_finite() || page_area <= f32::EPSILON {
        return proven;
    }
    let (left, right) = (x0.min(x1), x0.max(x1));
    let (bottom, top) = (y0.min(y1), y0.max(y1));
    for handle in &handles {
        let Some((width_pt, height_pt)) = handle.placed_extent_points() else {
            continue;
        };
        let bbox = &handle.bbox;
        let visible_width = ((bbox.x + bbox.width).min(right) - bbox.x.max(left)).max(0.0);
        let visible_height = ((bbox.y + bbox.height).min(top) - bbox.y.max(bottom)).max(0.0);
        if visible_width * visible_height < page_area * PAGE_COVERING_MIN {
            continue;
        }
        let Some(dpi) = placement_dpi(handle.width, handle.height, width_pt, height_pt) else {
            continue;
        };
        proven.insert(
            handle.paint_order,
            ProvenPlacement {
                dpi,
                declared_width: handle.width,
                declared_height: handle.height,
            },
        );
    }
    proven
}

#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
mod proven_source_dpi_tests {
    use super::{placement_dpi, proven_source_dpis};

    const PLACEMENT: &[u8] = b"q 612 0 0 5760 0 0 cm /Scan Do Q";
    const SCAN: &[u8] = include_bytes!("../../../../../fixtures/repros/tall-scan-strips/jpeg-tall.pdf");

    fn patched(placement: &[u8]) -> xberg_native_pdf::PdfDocument {
        let mut padded = placement.to_vec();
        assert!(
            padded.len() <= PLACEMENT.len(),
            "the stream length is fixed by the fixture"
        );
        padded.resize(PLACEMENT.len(), b' ');
        let start = SCAN
            .windows(PLACEMENT.len())
            .position(|window| window == PLACEMENT)
            .expect("the fixture paints its scan with a known matrix");
        let mut bytes = SCAN.to_vec();
        bytes[start..start + PLACEMENT.len()].copy_from_slice(&padded);
        xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("patched fixture opens")
    }

    fn dpis_with_placement(placement: &[u8]) -> std::collections::HashMap<usize, super::ProvenPlacement> {
        proven_source_dpis(&patched(placement), 0)
    }

    fn extent(placement: &[u8]) -> Option<(f64, f64)> {
        let document = patched(placement);
        let handles = document.page_image_handles(0).expect("one painted image");
        let [handle] = handles.as_slice() else {
            panic!("one image")
        };
        handle.placed_extent_points()
    }

    #[test]
    fn placed_extent_follows_the_image_axes_through_turns_mirrors_and_tolerance() {
        assert_eq!(extent(PLACEMENT), Some((612.0, 5760.0)));
        assert_eq!(extent(b"q 0 612 5760 0 0 0 cm /Scan Do Q"), Some((612.0, 5760.0)));
        assert_eq!(extent(b"-612 0 0 5760 612 0 cm /Scan Do"), Some((612.0, 5760.0)));
        // Shear is tolerated up to 1e-4 of the largest scale (0.576 here) and no further.
        assert_eq!(extent(b"612 0 0.5 5760 0 0 cm /Scan Do"), Some((612.0, 5760.0)));
        assert_eq!(extent(b"612 0 0.6 5760 0 0 cm /Scan Do"), None);
    }

    #[test]
    fn page_covering_scan_resolution_comes_from_its_placement() {
        let dpis = dpis_with_placement(PLACEMENT);
        assert_eq!(dpis.len(), 1, "{dpis:?}");
        assert!((dpis[&0].dpi - 150.0).abs() < 1e-6, "{dpis:?}");
    }

    #[test]
    fn a_page_with_a_user_unit_proves_nothing() {
        const PARENT: &[u8] = b"/Parent 2 0 R";
        let start = SCAN.windows(PARENT.len()).position(|w| w == PARENT).unwrap();
        let mut bytes = SCAN.to_vec();
        bytes[start..start + PARENT.len()].copy_from_slice(b"/UserUnit 2  ");
        let document = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("patched fixture opens");
        assert_eq!(document.get_page_user_unit(0).unwrap(), 2.0);
        assert!(proven_source_dpis(&document, 0).is_empty());
    }

    #[test]
    fn sheared_anisotropic_or_partial_placements_prove_nothing() {
        for (name, placement) in [
            ("shear", &b"q 612 0 9 5760 0 0 cm /Scan Do Q"[..]),
            ("anisotropic", &b"q 612 0 0 5000 0 0 cm /Scan Do Q"[..]),
            ("quarter of the page", &b"q 153 0 0 1440 0 0 cm /Scan Do Q"[..]),
        ] {
            assert!(dpis_with_placement(placement).is_empty(), "{name}");
        }
    }

    #[test]
    fn a_dictionary_that_disagrees_with_its_stream_is_not_confirmed() {
        const DECLARED: &[u8] = b"/Width 1275";
        let fallback = |bytes: Vec<u8>| {
            let document = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("fixture opens");
            let proven = proven_source_dpis(&document, 0);
            let images = crate::pdf::native::images::page_ocr_fallback_image_bytes(&document, 0, None, None).0;
            (proven, images)
        };
        let (proven, images) = fallback(SCAN.to_vec());
        let [image] = images.as_slice() else {
            panic!("one image")
        };
        assert_eq!(
            proven[&0].confirmed_by(image.format, &image.bytes),
            Some(proven[&0].dpi)
        );

        let start = SCAN.windows(DECLARED.len()).position(|w| w == DECLARED).unwrap();
        let mut patched = SCAN.to_vec();
        patched[start..start + DECLARED.len()].copy_from_slice(b"/Width 1276");
        let (proven, images) = fallback(patched);
        let [image] = images.as_slice() else {
            panic!("one image")
        };
        assert!(
            proven.contains_key(&0),
            "the placement alone still proves a resolution: {proven:?}"
        );
        assert_eq!(proven[&0].confirmed_by(image.format, &image.bytes), None);
        assert_eq!(proven[&0].confirmed_by("jpeg2000", &image.bytes), None);
    }

    #[test]
    fn a_stretched_thumbnail_is_not_a_scan_resolution() {
        let points = (612.0, 792.0);
        assert_eq!(placement_dpi(100, 130, points.0, points.1).map(|d| d.round()), None);
        assert!(placement_dpi(612, 792, points.0, points.1).is_some_and(|d| (d - 72.0).abs() < 1e-9));
        assert_eq!(placement_dpi(611, 791, points.0, points.1), None);
        assert!(placement_dpi(2550, 3300, points.0, points.1).is_some_and(|d| (d - 300.0).abs() < 1e-9));
    }
}
