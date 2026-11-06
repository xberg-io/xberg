/// Fraction of the page a painted image must cover for its scale to stand for the page's scan.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const PAGE_COVERING_MIN: f32 = 0.80;
/// Largest relative difference between the horizontal and vertical resolution of an image
/// whose placement still counts as proven.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
const SOURCE_DPI_ANISOTROPY_MAX: f64 = 0.01;

/// Resolution, by paint order, of each page-covering image whose placement is exactly known.
///
/// An image counts only when its matrix is an unsheared quarter-turn multiple (see
/// `PdfImageHandle::placed_extent_points`), its pixel size over its painted size gives the same
/// resolution along both axes, and its painted area covers at least [`PAGE_COVERING_MIN`] of the
/// media box. Any other image is absent, and the caller then leaves the resolution unknown.
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn proven_source_dpis(
    doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
) -> std::collections::HashMap<usize, f64> {
    const POINTS_PER_INCH: f64 = 72.0;
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
        let across = f64::from(handle.width) * POINTS_PER_INCH / width_pt;
        let down = f64::from(handle.height) * POINTS_PER_INCH / height_pt;
        if !(across.is_finite() && down.is_finite() && across > 0.0 && down > 0.0)
            || (across - down).abs() > across.max(down) * SOURCE_DPI_ANISOTROPY_MAX
        {
            continue;
        }
        if proven.insert(handle.paint_order, (across + down) / 2.0).is_some() {
            proven.remove(&handle.paint_order);
        }
    }
    proven
}

#[cfg(all(test, any(feature = "ocr", feature = "ocr-pipeline")))]
mod proven_source_dpi_tests {
    use super::proven_source_dpis;

    const PLACEMENT: &[u8] = b"q 612 0 0 5760 0 0 cm /Scan Do Q";
    const SCAN: &[u8] = include_bytes!("../../../../../fixtures/repros/tall-scan-strips/jpeg-tall.pdf");

    fn dpis_with_placement(placement: &[u8]) -> std::collections::HashMap<usize, f64> {
        assert_eq!(placement.len(), PLACEMENT.len());
        let start = SCAN
            .windows(PLACEMENT.len())
            .position(|window| window == PLACEMENT)
            .expect("the fixture paints its scan with a known matrix");
        let mut bytes = SCAN.to_vec();
        bytes[start..start + PLACEMENT.len()].copy_from_slice(placement);
        let document = xberg_native_pdf::PdfDocument::from_bytes(bytes).expect("patched fixture opens");
        proven_source_dpis(&document, 0)
    }

    #[test]
    fn page_covering_scan_resolution_comes_from_its_placement() {
        let dpis = dpis_with_placement(PLACEMENT);
        assert_eq!(dpis.len(), 1, "{dpis:?}");
        assert!((dpis[&0] - 150.0).abs() < 1e-6, "{dpis:?}");
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
}
