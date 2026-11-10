use super::*;
use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
use crate::rendering::render_png_page_with_budget_evidence;

const FULL_PAGE: &[u8] = b"q 32 0 0 32 0 0 cm /Im0 Do Q\n";

fn rgb_image(width: u32, height: u32) -> PdfImage {
    PdfImage::new(
        width,
        height,
        ColorSpace::DeviceRGB,
        8,
        ImageData::Jpeg(source::jpeg_of(width, height)),
    )
}

fn encoded_bytes(image: &PdfImage) -> u64 {
    let ImageData::Jpeg(bytes) = image.data() else {
        unreachable!()
    };
    (bytes.capacity() + bytes.len()) as u64
}

fn refused_as_over_budget(document: &PdfDocument) -> bool {
    match render_png_page_with_budget_evidence(document, 0, 72, 72, LIMIT) {
        Err((_, exceeded)) => exceeded,
        Ok(_) => false,
    }
}

#[test]
fn a_colour_key_masked_jpeg_is_refused_not_reduced() {
    let jpeg = source::jpeg(LARGE);
    let reducible = source::document_painting(&jpeg, LARGE, FULL_PAGE, "");
    assert!(
        render_png_page_with_budget_evidence(&reducible, 0, 72, 72, LIMIT).is_ok(),
        "without the /Mask the same image renders from a reduced decode"
    );
    let masked = source::document_painting(&jpeg, LARGE, FULL_PAGE, "/Mask [0 0 0 0 0 0]");
    assert!(
        refused_as_over_budget(&masked),
        "a colour-key /Mask indexes the full-size samples"
    );
}

#[test]
fn a_reduction_must_cover_the_extent_the_image_is_painted_over() {
    let jpeg = source::jpeg_of(1200, 1200);
    let fits = source::document_painting(&jpeg, 1200, FULL_PAGE, "");
    assert!(
        render_png_page_with_budget_evidence(&fits, 0, 72, 72, LIMIT).is_ok(),
        "painted over 32 px, a 1/2 decode covers the extent"
    );
    let stretched = source::document_painting(&jpeg, 1200, b"q 1200 0 0 1200 0 0 cm /Im0 Do Q\n", "");
    assert!(
        refused_as_over_budget(&stretched),
        "painted over 1200 px, a 600 px decode covers 50% of the extent"
    );
}

#[test]
fn the_full_decode_is_kept_at_exactly_the_limit_and_reduced_one_byte_under() {
    let image = rgb_image(1200, 1200);
    let canvas = 1000;
    let full = 1200 * 1200 * (3 + 4) + encoded_bytes(&image) + canvas;
    assert_eq!(full - encoded_bytes(&image) - canvas, 10_080_000);
    let admit = |limit: u64| image_budget::ImageBudget::new(limit as usize).admit_jpeg_reducing(&image, canvas, DEVICE);
    assert_eq!(admit(full).unwrap(), 1);
    assert_eq!(admit(full - 1).unwrap(), 2);
}

#[test]
fn a_reduced_decode_is_kept_at_exactly_the_limit_and_the_next_scale_one_byte_under() {
    let canvas = 1000;
    // Decode phase dominates: 2 * 360 000 output bytes of RGB, plus 150 MCU columns of coefficients
    // (3 components * 4 blocks * 2 rows * 128 bytes) beat the paint phase of 7 * 360 000.
    let square = rgb_image(1200, 1200);
    let required = 2_620_800 + encoded_bytes(&square) + canvas;
    // Paint phase dominates: 7 * 480 000 beats 6 * 480 000 + 460 800.
    let tall = rgb_image(1200, 1600);
    let tall_required = 3_360_000 + encoded_bytes(&tall) + canvas;
    for (image, required) in [(&square, required), (&tall, tall_required)] {
        let admit =
            |limit: u64| image_budget::ImageBudget::new(limit as usize).admit_jpeg_reducing(image, canvas, DEVICE);
        assert_eq!(admit(required).unwrap(), 2, "required {required}");
        assert_eq!(admit(required - 1).unwrap(), 4, "required {required}");
    }
}

#[test]
fn an_extended_sequential_jpeg_is_reduced_like_a_baseline_one() {
    let mut extended = source::jpeg_of(1200, 1200);
    let frame = source::sof_marker_position(&extended);
    assert_eq!(extended[frame + 1], 0xc0);
    extended[frame + 1] = 0xc1;
    let image = PdfImage::new(1200, 1200, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(extended.clone()));
    assert!(image_budget::reducible_jpeg(&image, &extended));
    assert_eq!(
        image_budget::ImageBudget::new(LIMIT)
            .admit_jpeg_reducing(&image, 0, DEVICE)
            .unwrap(),
        2
    );
    let decoded = image_budget::decode_reduced_jpeg(&extended, 2).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (600, 600));
}

#[test]
fn a_decode_refuses_a_frame_the_decoder_scales_to_another_size() {
    // Asked for 1x2048, `jpeg-decoder` reaches the one-pixel width at 1/8 and returns 1x512.
    let encoded = source::jpeg_of(1, 4096);
    let error = image_budget::decode_reduced_jpeg(&encoded, 2).unwrap_err();
    assert_eq!(
        error.to_string(),
        "Stream decoding error: JPEG decoded at 1x512, not the admitted 1x2048"
    );
}

#[test]
fn a_jpeg_image_never_carries_an_in_data_soft_mask() {
    use crate::object::ObjectRef;
    let jpeg = source::jpeg(32);
    for filter in ["/DCTDecode", "[/DCTDecode /JPXDecode]"] {
        let entries = format!("/SMaskInData 1 /Filter {filter}");
        let document = source::document_painting(&jpeg, 32, FULL_PAGE, &entries);
        let xobject = document.load_object(ObjectRef::new(5, 0)).unwrap();
        let extracted = crate::extractors::images::extract_image_from_xobject(Some(&document), &xobject, None, None);
        match extracted {
            Ok(image) => {
                assert!(image.soft_mask_in_data().is_none(), "{filter}");
                assert!(filter != "/DCTDecode" || matches!(image.data(), ImageData::Jpeg(_)));
            }
            Err(error) => assert_ne!(filter, "/DCTDecode", "a plain DCT image decodes: {error}"),
        }
    }
}
