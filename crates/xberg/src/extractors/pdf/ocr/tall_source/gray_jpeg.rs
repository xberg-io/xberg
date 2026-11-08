use super::{
    Arc, Attempt, ExtractedDocument, ExtractionConfig, OcrBackend, OcrConfig, PdfDocument, apply_recovery, recover,
};
use crate::extractors::pdf::ocr::scoring;

pub(in crate::extractors::pdf::ocr) async fn recover_after_gray_jpeg_render_failure(
    document: &PdfDocument,
    backend: &Arc<dyn OcrBackend>,
    config: &ExtractionConfig,
    ocr: &OcrConfig,
) -> crate::Result<Option<(ExtractedDocument, i32)>> {
    let attempt = Attempt::new(document, 0, None, config, ocr);
    let Some((recovered, strips)) = recover(attempt, backend, ocr).await? else {
        return Ok(None);
    };
    let thresholds = ocr.quality_thresholds.clone().unwrap_or_default();
    if !recovered.complete_source_rows
        || recovered.lowest_strip_confidence < thresholds.min_ocr_mean_confidence
        || recovered.worst_dictionary_invalid > thresholds.max_ocr_output_dict_invalid_word_ratio
        || scoring::is_ocr_recognition_noise(&recovered.text, &thresholds)
    {
        return Ok(None);
    }
    let Some(psm) = recovered.psm else { return Ok(None) };
    let mut result = ExtractedDocument::default();
    apply_recovery(&mut result, recovered, 0, strips);
    super::note_flat_structure(&mut result, config, 0);
    Ok(Some((result, psm)))
}

#[cfg(test)]
mod tests {
    use super::super::certify;
    use super::PdfDocument;

    #[test]
    fn gray_jpeg_certificate_requires_exact_full_page_paint_codec_and_budget() {
        use lopdf::{Document, Object, Stream};

        const JPEG: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/repros/tall-scan-strips/jpeg-tall.pdf"
        ));
        const LIMIT: usize = 100 * 1024 * 1024;
        let source = PdfDocument::from_bytes(JPEG.to_vec()).unwrap();
        let recovered = certify(&source, 0, None, LIMIT).expect("certified original JPEG");
        assert_eq!((recovered.width, recovered.height, recovered.dpi), (1275, 12000, 150.0));
        assert_eq!(recovered.samples.len(), 1275 * 12000);
        assert!(certify(&source, 0, None, 16 * 1024 * 1024).is_none());
        for (key, value) in [
            ("Mask", Object::Array(vec![0.into(), 0.into()])),
            ("Filter", Object::Name(b"FlateDecode".to_vec())),
            ("Width", Object::Integer(1276)),
        ] {
            let mut edited = Document::load_mem(JPEG).unwrap();
            let image = edited.get_object_mut((5, 0)).unwrap().as_stream_mut().unwrap();
            image.dict.set(key, value);
            let mut bytes = Vec::new();
            edited.save_to(&mut bytes).unwrap();
            let pdf = PdfDocument::from_bytes(bytes).unwrap();
            assert!(certify(&pdf, 0, None, LIMIT).is_none(), "{key} must refuse projection");
        }
        let mut edited = Document::load_mem(JPEG).unwrap();
        let image = edited.get_object_mut((5, 0)).unwrap().as_stream_mut().unwrap();
        image.dict.set("Width", 1280);
        image.dict.set("Height", 12047);
        let mut bytes = Vec::new();
        edited.save_to(&mut bytes).unwrap();
        let pdf = PdfDocument::from_bytes(bytes).unwrap();
        assert!(
            certify(&pdf, 0, None, LIMIT).is_none(),
            "JPEG header must match PDF dimensions"
        );

        let mut edited = Document::load_mem(JPEG).unwrap();
        let content = edited.get_object_mut((4, 0)).unwrap();
        *content = Object::Stream(Stream::new(
            Default::default(),
            b"q 612 0 0 5760 0 0 cm /Scan Do Q 0 0 3 3 re f".to_vec(),
        ));
        let mut bytes = Vec::new();
        edited.save_to(&mut bytes).unwrap();
        let pdf = PdfDocument::from_bytes(bytes).unwrap();
        assert!(
            certify(&pdf, 0, None, LIMIT).is_none(),
            "additional paint must refuse projection"
        );
    }
}
