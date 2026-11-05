use super::{XObjectRecoveryContext, XObjectRecoveryOutcome, recover_image_xobjects};
use crate::extractors::security::SecurityLimits;
use xberg_native_pdf::content::{Matrix, Operator, parse_content_stream};
use xberg_native_pdf::{PdfDocument, PdfImageHandle};

pub(in crate::extractors::pdf::ocr) fn can_project_stored_jpeg_page(
    doc: &PdfDocument,
    page: usize,
    limits: &SecurityLimits,
    context: &mut XObjectRecoveryContext<'_>,
) -> crate::Result<bool> {
    context.check_cancelled()?;
    let Ok(handles) = doc.page_image_handles(page) else {
        return Ok(false);
    };
    let mut stored = false;
    for handle in &handles {
        context.check_cancelled()?;
        context.budget.step()?;
        stored |= handle.plain_stored_jpeg_for_ocr_recovery().is_some();
    }
    // Other codecs keep their existing recovery behavior. A plain stored JPEG is
    // a complete page projection only when no other visible content is omitted.
    // Recovery does not map structured image geometry through arbitrary page CTMs,
    // so only a whole-page upright image is certified here. ~keep
    if !stored {
        return Ok(true);
    }
    if handles.len() != 1 || !unclipped_page_without_annotations(doc, page) {
        return Ok(false);
    }
    certify_content(doc, page, &handles[0], limits, context)
}

fn unclipped_page_without_annotations(doc: &PdfDocument, page: usize) -> bool {
    let Ok(page_object) = doc.get_page(page) else {
        return false;
    };
    let Some(dict) = page_object.as_dict() else {
        return false;
    };
    if let Some(annotations) = dict.get("Annots") {
        let Ok(annotations) = doc.resolve_object(annotations) else {
            return false;
        };
        if !annotations.as_array().is_some_and(|array| array.is_empty()) {
            return false;
        }
    }
    let Ok(info) = doc.get_page_info(page) else {
        return false;
    };
    info.crop_box.is_none_or(|crop| crop == info.media_box)
        && matches!(
            doc.get_page_rotation_status(page),
            Ok(xberg_native_pdf::document::PageRotation::Absent | xberg_native_pdf::document::PageRotation::Valid(0))
        )
}

fn certify_content(
    doc: &PdfDocument,
    page: usize,
    handle: &PdfImageHandle<'_>,
    limits: &SecurityLimits,
    context: &mut XObjectRecoveryContext<'_>,
) -> crate::Result<bool> {
    let Ok(bytes) = doc.get_page_content_data(page) else {
        return Ok(false);
    };
    if bytes.len() > limits.max_content_size {
        return Ok(false);
    }
    let Ok(operators) = parse_content_stream(&bytes) else {
        return Ok(false);
    };
    let Ok(bounds) = doc.get_page_media_box(page) else {
        return Ok(false);
    };
    let mut current = Matrix::identity();
    let mut stack = Vec::new();
    let mut painted = false;
    for operator in operators {
        context.check_cancelled()?;
        context.budget.step()?;
        match operator {
            Operator::SaveState if stack.len() < limits.max_nesting_depth => stack.push(current),
            Operator::RestoreState => match stack.pop() {
                Some(matrix) => current = matrix,
                None => return Ok(false),
            },
            Operator::Cm { a, b, c, d, e, f } => current = Matrix { a, b, c, d, e, f }.multiply(&current),
            Operator::Do { name } if !painted => {
                if handle.is_inline || !direct_image(doc, page, &name) || !complete_upright_image(current, bounds) {
                    return Ok(false);
                }
                painted = true;
            }
            _ => return Ok(false),
        }
    }
    Ok(painted && stack.is_empty())
}

fn direct_image(doc: &PdfDocument, page: usize, name: &str) -> bool {
    let Ok(resources) = doc.get_page_resources(page) else {
        return false;
    };
    let Some(resources) = resources.as_dict() else {
        return false;
    };
    let Some(xobjects) = resources.get("XObject") else {
        return false;
    };
    let Ok(xobjects) = doc.resolve_object(xobjects) else {
        return false;
    };
    let Some(image) = xobjects.as_dict().and_then(|dict| dict.get(name)) else {
        return false;
    };
    let Ok(image) = doc.resolve_object(image) else {
        return false;
    };
    image.as_dict().is_some_and(|dict| {
        dict.get("Subtype").and_then(xberg_native_pdf::object::Object::as_name) == Some("Image")
            && !dict.contains_key("OC")
    })
}

fn complete_upright_image(matrix: Matrix, (left, bottom, right, top): (f32, f32, f32, f32)) -> bool {
    [
        matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f, left, bottom, right, top,
    ]
    .iter()
    .all(|value| value.is_finite())
        && matrix.a > 0.0
        && matrix.d > 0.0
        && matrix.b == 0.0
        && matrix.c == 0.0
        && matrix.e == left
        && matrix.f == bottom
        && f64::from(matrix.e) + f64::from(matrix.a) == f64::from(right)
        && f64::from(matrix.f) + f64::from(matrix.d) == f64::from(top)
}

#[cfg(all(any(feature = "ocr", feature = "ocr-pipeline"), feature = "pdf"))]
pub(in crate::extractors::pdf::ocr) async fn recover_page_text_from_bounded_image_xobjects(
    backend: &std::sync::Arc<dyn crate::plugins::OcrBackend>,
    render_doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
    total_pages: usize,
    ocr_config: &crate::core::config::OcrConfig,
    mut context: XObjectRecoveryContext<'_>,
) -> crate::Result<Option<XObjectRecoveryOutcome>> {
    context.check_cancelled()?;
    let limits = ocr_config.security_limits.clone().unwrap_or_default();
    if !can_project_stored_jpeg_page(render_doc, page_idx, &limits, &mut context)? {
        return Ok(None);
    }
    let fallback_images = crate::pdf::native::images::page_ocr_fallback_image_bytes_with_stored_copy_limit(
        render_doc,
        page_idx,
        limits.max_content_size,
    );
    context.check_cancelled()?;
    if fallback_images.is_empty() {
        return Ok(None);
    }
    let whole_page_raster = crate::pdf::scan_detect::full_page_raster_density(render_doc, page_idx).is_some();
    let ocr_config = super::super::pipeline::ocr_config_with_page_rotation_hint(
        ocr_config,
        0,
        None,
        whole_page_raster,
        false,
        false,
    );
    let mut recovery =
        recover_image_xobjects(backend, &fallback_images, page_idx, total_pages, &ocr_config, context).await?;
    for (image, fallback) in recovery.images.iter_mut().zip(&fallback_images) {
        image.description = Some(format!(
            "recovered from raw image XObject ({}) after all page OCR stages failed",
            fallback.recovery.as_str()
        ));
    }
    Ok(Some(recovery))
}
#[cfg(all(test, feature = "ocr"))]
#[path = "composition/tests.rs"]
mod tests;
