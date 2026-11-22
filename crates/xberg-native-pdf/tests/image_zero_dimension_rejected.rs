//! An image XObject whose /Width or /Height is zero (or negative) is invalid
//! (ISO 32000-1:2008 Table 89 requires positive integers) and must be rejected
//! with an error rather than flowing into allocation and sampling arithmetic.

use bytes::Bytes;
use std::collections::HashMap;
use xberg_native_pdf::extractors::extract_image_from_xobject;
use xberg_native_pdf::object::Object;

fn gray_image(width: i64, height: i64) -> Object {
    let mut dict = HashMap::new();
    dict.insert("Subtype".to_string(), Object::Name("Image".to_string()));
    dict.insert("Width".to_string(), Object::Integer(width));
    dict.insert("Height".to_string(), Object::Integer(height));
    dict.insert("ColorSpace".to_string(), Object::Name("DeviceGray".to_string()));
    dict.insert("BitsPerComponent".to_string(), Object::Integer(8));
    Object::Stream {
        dict,
        data: Bytes::from(vec![0u8; 16]),
    }
}

#[test]
fn zero_width_is_rejected() {
    assert!(extract_image_from_xobject(None, &gray_image(0, 4), None, None).is_err());
}

#[test]
fn zero_height_is_rejected() {
    assert!(extract_image_from_xobject(None, &gray_image(4, 0), None, None).is_err());
}

#[test]
fn negative_dimension_is_rejected() {
    assert!(extract_image_from_xobject(None, &gray_image(-1, 4), None, None).is_err());
}

#[test]
fn positive_dimensions_still_decode() {
    let image = extract_image_from_xobject(None, &gray_image(4, 4), None, None).expect("valid image decodes");
    assert_eq!((image.width(), image.height()), (4, 4));
}
