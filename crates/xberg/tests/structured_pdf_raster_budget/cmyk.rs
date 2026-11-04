use super::*;

const WIDTH: usize = 100;
const BASE_PNG_CHARGE: usize = WIDTH * WIDTH * 12 + 256 * 1024;
const CMYK_PNG_CHARGE: usize = BASE_PNG_CHARGE + WIDTH * WIDTH * 4;
const SPOT_PNG_CHARGE: usize = CMYK_PNG_CHARGE + WIDTH * WIDTH;

fn synthetic_cmyk_profile() -> Vec<u8> {
    // Constant-Lab ICC v2 CMYK input LUT, like the native compositor fixtures.
    let mut lut = b"mft1\0\0\0\0\x04\x03\x02\0".to_vec();
    for component in [0x10000_u32, 0, 0, 0, 0x10000, 0, 0, 0, 0x10000] {
        lut.extend(component.to_be_bytes());
    }
    for _ in 0..4 {
        lut.extend(0..=u8::MAX);
    }
    for _ in 0..16 {
        lut.extend([135, 128, 128]);
    }
    for _ in 0..3 {
        lut.extend(0..=u8::MAX);
    }

    let mut profile = vec![0_u8; 128];
    profile[..4].copy_from_slice(&(144_u32 + lut.len() as u32).to_be_bytes());
    profile[8..12].copy_from_slice(&0x0240_0000_u32.to_be_bytes());
    profile[12..24].copy_from_slice(b"prtrCMYKLab ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[68..72].copy_from_slice(&0x0000_F6D6_u32.to_be_bytes());
    profile[72..76].copy_from_slice(&0x0001_0000_u32.to_be_bytes());
    profile[76..80].copy_from_slice(&0x0000_D32D_u32.to_be_bytes());
    profile.extend(1_u32.to_be_bytes());
    profile.extend(b"A2B0");
    profile.extend(144_u32.to_be_bytes());
    profile.extend((lut.len() as u32).to_be_bytes());
    profile.extend(lut);
    profile
}

fn spot_color_space() -> Vec<Object> {
    let tint_transform = dictionary! {
        "FunctionType" => 2,
        "Domain" => vec![Object::Integer(0), Object::Integer(1)],
        "C0" => vec![Object::Integer(0); 4],
        "C1" => vec![Object::Integer(1), Object::Integer(0), Object::Integer(0), Object::Integer(0)],
        "N" => 1,
    };
    vec![
        Object::Name(b"Separation".to_vec()),
        Object::Name(b"SpotInk".to_vec()),
        Object::Name(b"DeviceCMYK".to_vec()),
        Object::Dictionary(tint_transform),
    ]
}

fn cmyk_page(spot: bool) -> Vec<u8> {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let profile = pdf.add_object(Stream::new(dictionary! { "N" => 4 }, synthetic_cmyk_profile()));
    let mut resources = dictionary! {
        "ExtGState" => dictionary! { "Half" => dictionary! { "Type" => "ExtGState", "ca" => 0.5_f32 } },
    };
    let mut commands = b"0 0 0 1 k /Half gs 0 0 100 100 re f".to_vec();
    if spot {
        resources.set("ColorSpace", dictionary! { "Spot" => spot_color_space() });
        commands.extend(b" /Spot cs 1 scn 10 10 80 80 re f");
    }
    let content = pdf.add_object(Stream::new(dictionary! {}, commands));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content, "Resources" => resources,
        "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Integer(100), Object::Integer(100)],
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1,
        }),
    );
    let output_intent = dictionary! {
        "Type" => "OutputIntent", "S" => "GTS_PDFX", "DestOutputProfile" => profile,
        "OutputConditionIdentifier" => Object::string_literal("synthetic CMYK"),
    };
    let root = pdf.add_object(dictionary! {
        "Type" => "Catalog", "Pages" => pages, "OutputIntents" => vec![Object::Dictionary(output_intent)],
    });
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("serialize synthetic press-target PDF");
    bytes
}

fn native_png_and_sidecar(pdf: &[u8], spot: bool) -> Vec<u8> {
    use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

    let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf.to_vec()).expect("open press-target PDF");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(DPI));
    let rendered = renderer.render_page(&doc, 0).expect("unbounded native PNG render");
    assert_eq!((rendered.width, rendered.height), (100, 100));
    assert_eq!(renderer.cmyk_sidecar_cmyk_bytes().map(<[u8]>::len), Some(40_000));
    assert_eq!(renderer.cmyk_sidecar_spot_names().unwrap().len(), usize::from(spot));
    if spot {
        assert_eq!(renderer.cmyk_sidecar_spot_names().unwrap(), ["SpotInk"]);
        assert_eq!(renderer.cmyk_sidecar_spot_plane(0).map(<[u8]>::len), Some(10_000));
    }
    rendered.data
}

async fn structured_pdf_at_budget(pdf: &[u8], max_content_size: usize) -> Result<Vec<u8>, RasterizeError> {
    let mut pages = pages_for_call_with_security_limits(
        pdf,
        PDF_MIME,
        StructuredCallMode::VisionOnly,
        DPI,
        &limits(max_content_size),
    )
    .await?;
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    Ok(pages.remove(0).png_bytes)
}

fn assert_budget_error(error: RasterizeError, max_content_size: usize) {
    assert!(matches!(error, RasterizeError::Pdf(_)), "{error}");
    let message = error.to_string();
    assert!(message.contains("page 1"), "{message}");
    assert!(
        message.contains(&format!("security_limits.max_content_size ({max_content_size} bytes)")),
        "{message}"
    );
}

#[tokio::test]
async fn cmyk_and_spot_planes_are_charged_at_their_exact_boundaries() {
    for (has_spot, budget) in [(false, CMYK_PNG_CHARGE), (true, SPOT_PNG_CHARGE)] {
        let pdf = cmyk_page(has_spot);
        let expected = native_png_and_sidecar(&pdf, has_spot);
        let admitted = structured_pdf_at_budget(&pdf, budget)
            .await
            .expect("exact charge admits PNG");
        assert_eq!(admitted, expected);
        let denied = structured_pdf_at_budget(&pdf, budget - 1)
            .await
            .expect_err("one byte below the live raster and sidecar charge must reject");
        assert_budget_error(denied, budget - 1);
    }
}

#[tokio::test]
async fn spot_plane_alone_exceeds_the_same_budget_that_admits_cmyk() {
    let cmyk = cmyk_page(false);
    let spot = cmyk_page(true);
    let expected = native_png_and_sidecar(&cmyk, false);
    native_png_and_sidecar(&spot, true);
    assert_eq!(
        structured_pdf_at_budget(&cmyk, CMYK_PNG_CHARGE).await.unwrap(),
        expected
    );
    let denied = structured_pdf_at_budget(&spot, CMYK_PNG_CHARGE)
        .await
        .expect_err("one extra spot plane must exceed the same live raster budget");
    assert_budget_error(denied, CMYK_PNG_CHARGE);
}

#[test]
fn fitted_native_dpi_accounts_for_spot_planes_before_rendering() {
    use xberg_native_pdf::rendering::render_png_page_with_dpi_budget;

    let pdf = cmyk_page(true);
    native_png_and_sidecar(&pdf, true);
    let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf).expect("open press-target PDF");
    let (image, dpi) = render_png_page_with_dpi_budget(&doc, 0, DPI, 69, CMYK_PNG_CHARGE).expect("96px with spot fits");
    assert_eq!((image.width, image.height, dpi), (96, 96, 69));

    let error = render_png_page_with_dpi_budget(&doc, 0, DPI, 70, CMYK_PNG_CHARGE)
        .err()
        .expect("98px with spot exceeds the budget");
    assert!(
        error
            .to_string()
            .contains("PDF raster 98x98 exceeds security_limits.max_content_size"),
        "{error}"
    );
}
