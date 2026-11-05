use super::{fixture, model, pixel_memory};
use lopdf::{Document, Stream, dictionary};

const CHILD: &str = "XBERG_JPEG_RECOVERY_COMPOSITION_CHILD";

fn raster_annotation() -> xberg_native_pdf::rendering::RenderedImage {
    let mut pdf = Document::with_version("1.7");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica"});
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        b"BT /Fextra 10 Tf 0 g 2 3 Td (OVERLAY) Tj ET\n".to_vec(),
    ));
    let page = pdf.add_object(dictionary! {
        "Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),57.into(),15.into()],
        "Resources"=>dictionary! {"Font"=>dictionary! {"Fextra"=>font}},"Contents"=>content
    });
    pdf.objects.insert(
        pages,
        lopdf::Object::Dictionary(dictionary! {"Type"=>"Pages","Count"=>1,"Kids"=>vec![page.into()]}),
    );
    let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let native = xberg_native_pdf::PdfDocument::from_bytes(bytes).unwrap();
    let rendered = xberg_native_pdf::rendering::render_page(
        &native,
        0,
        &xberg_native_pdf::rendering::RenderOptions::with_dpi(300).as_raw(),
    )
    .unwrap();
    assert!(rendered.data.chunks_exact(4).filter(|pixel| pixel[0] < 64).count() > 100);
    rendered
}

fn annotated_pdf(bytes: &[u8]) -> Vec<u8> {
    let mut pdf = Document::load_mem(bytes).unwrap();
    let raster = raster_annotation();
    let pixels: Vec<_> = raster
        .data
        .chunks_exact(4)
        .flat_map(|pixel| pixel[..3].iter().copied())
        .collect();
    let image = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject","Subtype"=>"Image","Width"=>raster.width,"Height"=>raster.height,
            "ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8
        },
        pixels,
    ));
    let appearance = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject","Subtype"=>"Form","BBox"=>vec![0.into(),0.into(),57.into(),15.into()],
            "Resources"=>dictionary! {"XObject"=>dictionary! {"Glyph"=>image}}
        },
        b"q 57 0 0 15 0 0 cm /Glyph Do Q\n".to_vec(),
    ));
    let annotation = pdf.add_object(dictionary! {
        "Type"=>"Annot","Subtype"=>"Stamp","Rect"=>vec![70.into(),1.into(),127.into(),16.into()],
        "Contents"=>lopdf::Object::string_literal(""),"AP"=>dictionary! {"N"=>appearance}
    });
    let page = pdf.get_pages()[&1];
    pdf.get_object_mut(page)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Annots", vec![annotation.into()]);
    let mut output = Vec::new();
    pdf.save_to(&mut output).unwrap();
    let native = xberg_native_pdf::PdfDocument::from_bytes(output.clone()).unwrap();
    assert_eq!(native.get_annotations(0).unwrap().len(), 1);
    assert!(
        native.extract_text_auto(0).unwrap().trim().is_empty(),
        "appearance has raster-only text"
    );
    assert_eq!(
        native.page_image_handles(0).unwrap().len(),
        1,
        "page image recovery excludes annotation appearance"
    );
    output
}

fn config(mib: usize, model: &std::path::Path) -> crate::core::config::ExtractionConfig {
    let mut config = super::config(mib, model);
    config.ocr.as_mut().unwrap().pipeline.as_mut().unwrap().stages[0].backend_options =
        Some(serde_json::json!({"source_dpi":600,"use_cache":false}));
    config
}

async fn public_text(bytes: &[u8], config: &crate::core::config::ExtractionConfig) -> crate::Result<String> {
    let result = crate::extract(
        crate::ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
        config,
    )
    .await?;
    if !result.errors.is_empty() {
        return Err(crate::XbergError::validation(format!("{:?}", result.errors)));
    }
    assert_eq!(result.results.len(), 1);
    Ok(result.results.into_iter().next().unwrap().content)
}

fn jpeg_masked_pdf(bytes: &[u8], dimension: u32, sample: u8) -> Vec<u8> {
    let mut pdf = Document::load_mem(bytes).unwrap();
    let mut encoded = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut encoded)
        .encode(
            &vec![sample; dimension as usize * dimension as usize],
            dimension,
            dimension,
            image::ExtendedColorType::L8,
        )
        .unwrap();
    let mask = pdf.add_object(Stream::new(
        dictionary! {
            "Type"=>"XObject","Subtype"=>"Image","Width"=>dimension,"Height"=>dimension,
            "ColorSpace"=>"DeviceGray","BitsPerComponent"=>8,"Filter"=>"DCTDecode"
        },
        encoded,
    ));
    let mut images = 0;
    for (_, object) in pdf.objects.iter_mut() {
        if let lopdf::Object::Stream(stream) = object
            && stream
                .dict
                .get(b"Subtype")
                .is_ok_and(|value| value.as_name().is_ok_and(|name| name == b"Image"))
            && !stream.dict.has(b"SMask")
            && stream
                .dict
                .get(b"ColorSpace")
                .is_ok_and(|value| value.as_name().is_ok_and(|name| name == b"DeviceRGB"))
        {
            stream.dict.set("SMask", mask);
            images += 1;
        }
    }
    assert_eq!(images, 1);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

async fn qualify_affordable_jpeg_mask(bytes: &[u8], model: &std::path::Path) {
    let masked = jpeg_masked_pdf(bytes, 32, 255);
    let mut default = crate::core::config::ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        ocr: Some(crate::core::config::OcrConfig {
            tessdata_path: Some(model.to_path_buf()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let text = public_text(&masked, &default)
        .await
        .expect("default OCR with affordable JPEG SMask");
    fixture::assert_words(&text);
    default.security_limits = Some(crate::extractors::security::SecurityLimits {
        max_content_size: 512 * 1024 * 1024,
        ..Default::default()
    });
    let large_cap = public_text(&masked, &default).await.expect("512MiB JPEG SMask");
    assert_eq!(large_cap, text, "budget preserves exact default OCR text");
    println!("AFFORDABLE_JPEG_SMASK_DEFAULT_TEXT={text:?}");
}

async fn qualify_oversized_jpeg_mask(bytes: &[u8], model: &std::path::Path) {
    let masked = jpeg_masked_pdf(bytes, 9000, 0);
    let document = xberg_native_pdf::PdfDocument::from_bytes(masked.clone()).unwrap();
    let visible = xberg_native_pdf::rendering::render_page(
        &document,
        0,
        &xberg_native_pdf::rendering::RenderOptions::with_dpi(72).as_raw(),
    )
    .unwrap();
    assert!(
        visible.data.chunks_exact(4).all(|pixel| pixel[..3] == [255, 255, 255]),
        "opaque-black SMask hides every base-image word"
    );
    let mut wrongly_recovered = Vec::new();
    for mib in [40, 64] {
        let result = public_text(&masked, &config(mib, model)).await;
        println!("OVERSIZE_JPEG_SMASK_RESULT={mib}:{result:?}");
        if let Ok(text) = result {
            wrongly_recovered.push((mib, text));
        }
    }
    assert!(
        wrongly_recovered.is_empty(),
        "masked base JPEG cannot represent this page: {wrongly_recovered:?}"
    );
}

async fn qualify() {
    let model = model::verified_model_directory();
    let fixture = fixture::fixture();
    qualify_affordable_jpeg_mask(&fixture.pdf, &model).await;
    qualify_oversized_jpeg_mask(&fixture.pdf, &model).await;
    let annotated = annotated_pdf(&fixture.pdf);
    pixel_memory::install();
    let complete = public_text(&annotated, &config(40, &model)).await.unwrap();
    fixture::assert_words(&complete);
    assert!(
        complete.contains("OVERLAY"),
        "actual affordable annotation OCR: {complete}"
    );
    let pure = public_text(&fixture.pdf, &config(5, &model))
        .await
        .expect("bounded pure-image recovery");
    fixture::assert_words(&pure);
    let before = pixel_memory::observation().allocations;
    let result = public_text(&annotated, &config(5, &model)).await;
    println!(
        "BUDGET_COMPOSITION_RESULT={}",
        serde_json::json!({
            "affordable_annotation_text":complete,"pure_budget_text":pure,
        "annotated_pdf_sha256":fixture::sha(&annotated),
            "unsafe_result":result.as_ref().ok(),"error":result.as_ref().err().map(ToString::to_string),
            "new_native_allocations":pixel_memory::observation().allocations-before,
        })
    );
    assert!(
        result.is_err(),
        "stored JPEG alone cannot recover the visible annotation: {result:?}"
    );
    assert_eq!(
        pixel_memory::observation().allocations,
        before,
        "do not OCR an incomplete page projection"
    );
    crate::plugins::clear_ocr_backends().unwrap();
    model.verify();
}

#[tokio::test]
#[ignore = "model-backed composition recovery; covered by rust:test:jpeg-recovery"]
async fn image_only_recovery_never_discards_a_visible_annotation_after_budget_failure() {
    if std::env::var(CHILD).as_deref() == Ok("1") {
        qualify().await;
        return;
    }
    let selector = concat!(
        module_path!(),
        "::image_only_recovery_never_discards_a_visible_annotation_after_budget_failure"
    )
    .strip_prefix("xberg::")
    .unwrap();
    let cache = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", selector, "--ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .env("XBERG_CACHE_DIR", cache.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success()
            && stdout.contains("1 passed; 0 failed; 0 ignored;")
            && stdout.contains("BUDGET_COMPOSITION_RESULT="),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{stdout}");
}
