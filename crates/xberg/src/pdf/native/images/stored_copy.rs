use super::*;

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(super) fn collect_legacy_page_fallback_images(
    doc: &xberg_native_pdf::PdfDocument,
    page_idx: usize,
    mut remaining: Option<usize>,
    mut on_decode: impl FnMut(usize),
) -> Vec<PageFallbackImage> {
    let handles = match doc.page_image_handles(page_idx) {
        Ok(handles) => handles,
        Err(error) => {
            tracing::debug!(
                page = page_idx,
                "force_ocr fallback: enumerating image handles failed: {error}"
            );
            return Vec::new();
        }
    };
    let mut out = Vec::with_capacity(handles.len());
    for handle in &handles {
        let next_remaining = if let Some(limit) = remaining {
            let Ok(length) = handle.stored_stream_len() else {
                continue;
            };
            let Some(next) = limit.checked_sub(length) else {
                continue;
            };
            Some(next)
        } else {
            None
        };
        on_decode(handle.paint_order);
        if let Some(recovered) = recover_legacy_page_image(handle, page_idx) {
            out.push(recovered);
            remaining = next_remaining;
        }
    }
    out
}

#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
fn recover_legacy_page_image(
    handle: &xberg_native_pdf::extractors::images::PdfImageHandle<'_>,
    page_idx: usize,
) -> Option<PageFallbackImage> {
    let decoded = handle.decode().map(|image| {
        let dimensions = (image.width(), image.height());
        (dimensions, image.into_data())
    });
    match decoded {
        Ok((_, xberg_native_pdf::extractors::ImageData::Jpeg(bytes))) => Some(PageFallbackImage {
            bytes: Bytes::from(bytes),
            format: "jpeg",
            recovery: XObjectRecovery::EmbeddedJpeg,
            paint_order: handle.paint_order,
        }),
        Ok(((width, height), xberg_native_pdf::extractors::ImageData::Raw { pixels, format })) => {
            match raw_pixels_to_png(width, height, &format, &pixels) {
                Ok(bytes) => Some(PageFallbackImage {
                    bytes,
                    format: "png",
                    recovery: XObjectRecovery::ReencodedPixels,
                    paint_order: handle.paint_order,
                }),
                Err(error) => {
                    tracing::debug!(page = page_idx, "force_ocr fallback: raw re-encode failed: {error}");
                    None
                }
            }
        }
        Err(decode_err) => {
            if matches!(
                handle.filter_chain.last(),
                Some(xberg_native_pdf::PdfFilter::DCTDecode) | Some(xberg_native_pdf::PdfFilter::JPXDecode)
            ) {
                match handle.raw_compressed_bytes() {
                    Ok(raw) if matches!(detect_image_format_from_bytes(&raw), "jpeg" | "jpeg2000") => {
                        Some(PageFallbackImage {
                            format: detect_image_format_from_bytes(&raw),
                            bytes: Bytes::from(raw),
                            recovery: XObjectRecovery::RawCompressedStream,
                            paint_order: handle.paint_order,
                        })
                    }
                    Ok(_) => {
                        tracing::debug!(
                            page = page_idx,
                            "force_ocr fallback: raw bytes not a recognizable JPEG/JP2"
                        );
                        None
                    }
                    Err(error) => {
                        tracing::debug!(
                            page = page_idx,
                            "force_ocr fallback: raw_compressed_bytes failed: {error}"
                        );
                        None
                    }
                }
            } else {
                tracing::debug!(
                    page = page_idx,
                    "force_ocr fallback: undecodable image, non-JPEG codec: {decode_err}"
                );
                None
            }
        }
    }
}
