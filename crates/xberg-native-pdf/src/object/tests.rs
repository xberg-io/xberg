use super::*;
use std::collections::HashMap;

#[test]
fn test_object_integer() {
    let obj = Object::Integer(42);
    assert_eq!(obj.as_integer(), Some(42));
    assert!(obj.as_name().is_none());
    assert!(!obj.is_null());
}

#[test]
fn test_object_name() {
    let obj = Object::Name("Type".to_string());
    assert_eq!(obj.as_name(), Some("Type"));
    assert!(obj.as_integer().is_none());
}

#[test]
fn test_object_bool() {
    let obj = Object::Boolean(true);
    assert_eq!(obj.as_bool(), Some(true));
}

#[test]
#[allow(clippy::approx_constant)]
fn test_object_real() {
    let obj = Object::Real(3.14);
    assert_eq!(obj.as_real(), Some(3.14));
}

#[test]
fn test_object_string() {
    let obj = Object::String(b"Hello".to_vec());
    assert_eq!(obj.as_string(), Some(&b"Hello"[..]));
}

#[test]
fn test_object_null() {
    let obj = Object::Null;
    assert!(obj.is_null());
    assert!(obj.as_integer().is_none());
}

#[test]
fn test_object_array() {
    let obj = Object::Array(vec![Object::Integer(1), Object::Integer(2)]);
    let arr = obj.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].as_integer(), Some(1));
}

#[test]
fn test_object_dictionary() {
    let mut dict = HashMap::new();
    dict.insert("Type".to_string(), Object::Name("Page".to_string()));
    let obj = Object::Dictionary(dict);

    let d = obj.as_dict().unwrap();
    assert_eq!(d.get("Type").unwrap().as_name(), Some("Page"));
}

#[test]
fn test_object_stream_dict_access() {
    let mut dict = HashMap::new();
    dict.insert("Length".to_string(), Object::Integer(100));
    let obj = Object::Stream {
        dict,
        data: bytes::Bytes::from_static(b"stream data"),
    };

    let d = obj.as_dict().unwrap();
    assert_eq!(d.get("Length").unwrap().as_integer(), Some(100));
}

#[test]
fn test_object_reference() {
    let obj_ref = ObjectRef::new(10, 0);
    let obj = Object::Reference(obj_ref);

    assert_eq!(obj.as_reference(), Some(obj_ref));
    assert_eq!(obj_ref.id, 10);
    assert_eq!(obj_ref.generation, 0);
}

#[test]
fn test_object_ref_display() {
    let obj_ref = ObjectRef::new(10, 0);
    assert_eq!(format!("{}", obj_ref), "10 0 R");
}

#[test]
fn test_object_clone() {
    let obj = Object::Integer(42);
    let cloned = obj.clone();
    assert_eq!(obj, cloned);
}

#[test]
fn test_object_ref_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(ObjectRef::new(1, 0));
    set.insert(ObjectRef::new(2, 0));
    set.insert(ObjectRef::new(1, 0));

    assert_eq!(set.len(), 2);
}

#[test]
fn test_decode_stream_no_filter() {
    let mut dict = HashMap::new();
    dict.insert("Length".to_string(), Object::Integer(5));
    let obj = Object::Stream {
        dict,
        data: bytes::Bytes::from_static(b"Hello"),
    };

    let decoded = obj.decode_stream_data().unwrap();
    assert_eq!(decoded, b"Hello");
}

#[test]
fn test_decode_stream_single_filter() {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name("ASCIIHexDecode".to_string()));
    let obj = Object::Stream {
        dict,
        data: bytes::Bytes::from_static(b"48656C6C6F"),
    };

    let decoded = obj.decode_stream_data().unwrap();
    assert_eq!(decoded, b"Hello");
}

#[test]
fn test_decode_stream_filter_array() {
    let mut dict = HashMap::new();
    dict.insert(
        "Filter".to_string(),
        Object::Array(vec![Object::Name("ASCIIHexDecode".to_string())]),
    );
    let obj = Object::Stream {
        dict,
        data: bytes::Bytes::from_static(b"48656C6C6F"),
    };

    let decoded = obj.decode_stream_data().unwrap();
    assert_eq!(decoded, b"Hello");
}

#[test]
fn test_decode_stream_not_a_stream() {
    let obj = Object::Integer(42);
    let result = obj.decode_stream_data();
    assert!(result.is_err());
    match result {
        Err(Error::InvalidObjectType { expected, found }) => {
            assert_eq!(expected, "Stream");
            assert_eq!(found, "Integer");
        }
        _ => panic!("Expected InvalidObjectType error"),
    }
}

#[test]
fn test_decode_dictionary_as_stream() {
    let mut dict = HashMap::new();
    dict.insert("Length".to_string(), Object::Integer(0));
    let obj = Object::Dictionary(dict);

    let decoded = obj.decode_stream_data().unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn test_decode_dictionary_as_stream_with_filter() {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name("ASCIIHexDecode".to_string()));
    let obj = Object::Dictionary(dict);

    let decoded = obj.decode_stream_data().unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn test_extract_filter_names_single() {
    let filter = Object::Name("FlateDecode".to_string());
    let names = extract_filter_names(&filter);
    assert_eq!(names, vec!["FlateDecode"]);
}

#[test]
fn test_extract_filter_names_array() {
    let filter = Object::Array(vec![
        Object::Name("ASCII85Decode".to_string()),
        Object::Name("FlateDecode".to_string()),
    ]);
    let names = extract_filter_names(&filter);
    assert_eq!(names, vec!["ASCII85Decode", "FlateDecode"]);
}

#[test]
fn test_extract_filter_names_invalid() {
    let filter = Object::Integer(42);
    let names = extract_filter_names(&filter);
    assert!(names.is_empty());
}

#[test]
fn test_type_name_all_variants() {
    assert_eq!(Object::Null.type_name(), "Null");
    assert_eq!(Object::Boolean(true).type_name(), "Boolean");
    assert_eq!(Object::Integer(0).type_name(), "Integer");
    assert_eq!(Object::Real(0.0).type_name(), "Real");
    assert_eq!(Object::String(vec![]).type_name(), "String");
    assert_eq!(Object::Name("X".to_string()).type_name(), "Name");
    assert_eq!(Object::Array(vec![]).type_name(), "Array");
    assert_eq!(Object::Dictionary(HashMap::new()).type_name(), "Dictionary");
    assert_eq!(
        Object::Stream {
            dict: HashMap::new(),
            data: bytes::Bytes::new()
        }
        .type_name(),
        "Stream"
    );
    assert_eq!(Object::Reference(ObjectRef::new(1, 0)).type_name(), "Reference");
}

#[test]
fn test_as_integer_returns_none_for_non_integer() {
    assert!(Object::Null.as_integer().is_none());
    assert!(Object::Boolean(true).as_integer().is_none());
    assert!(Object::Real(1.0).as_integer().is_none());
    assert!(Object::String(vec![]).as_integer().is_none());
    assert!(Object::Name("X".to_string()).as_integer().is_none());
    assert!(Object::Array(vec![]).as_integer().is_none());
    assert!(Object::Dictionary(HashMap::new()).as_integer().is_none());
    assert!(Object::Reference(ObjectRef::new(1, 0)).as_integer().is_none());
}

#[test]
fn test_as_name_returns_none_for_non_name() {
    assert!(Object::Null.as_name().is_none());
    assert!(Object::Integer(1).as_name().is_none());
    assert!(Object::Boolean(true).as_name().is_none());
    assert!(Object::Real(1.0).as_name().is_none());
    assert!(Object::String(vec![]).as_name().is_none());
    assert!(Object::Array(vec![]).as_name().is_none());
}

#[test]
fn test_as_dict_returns_none_for_non_dict() {
    assert!(Object::Null.as_dict().is_none());
    assert!(Object::Integer(1).as_dict().is_none());
    assert!(Object::Boolean(true).as_dict().is_none());
    assert!(Object::Real(1.0).as_dict().is_none());
    assert!(Object::String(vec![]).as_dict().is_none());
    assert!(Object::Name("X".to_string()).as_dict().is_none());
    assert!(Object::Array(vec![]).as_dict().is_none());
    assert!(Object::Reference(ObjectRef::new(1, 0)).as_dict().is_none());
}

#[test]
fn test_as_array_returns_none_for_non_array() {
    assert!(Object::Null.as_array().is_none());
    assert!(Object::Integer(1).as_array().is_none());
    assert!(Object::Dictionary(HashMap::new()).as_array().is_none());
    assert!(Object::Name("X".to_string()).as_array().is_none());
}

#[test]
fn test_as_reference_returns_none_for_non_reference() {
    assert!(Object::Null.as_reference().is_none());
    assert!(Object::Integer(1).as_reference().is_none());
    assert!(Object::Name("X".to_string()).as_reference().is_none());
    assert!(Object::Dictionary(HashMap::new()).as_reference().is_none());
}

#[test]
fn test_as_bool_returns_none_for_non_bool() {
    assert!(Object::Null.as_bool().is_none());
    assert!(Object::Integer(1).as_bool().is_none());
    assert!(Object::Real(1.0).as_bool().is_none());
    assert!(Object::String(vec![]).as_bool().is_none());
}

#[test]
fn test_as_real_returns_none_for_non_real() {
    assert!(Object::Null.as_real().is_none());
    assert!(Object::Integer(1).as_real().is_none());
    assert!(Object::Boolean(true).as_real().is_none());
    assert!(Object::String(vec![]).as_real().is_none());
}

#[test]
fn test_as_string_returns_none_for_non_string() {
    assert!(Object::Null.as_string().is_none());
    assert!(Object::Integer(1).as_string().is_none());
    assert!(Object::Boolean(true).as_string().is_none());
    assert!(Object::Real(1.0).as_string().is_none());
    assert!(Object::Name("X".to_string()).as_string().is_none());
}

#[test]
fn test_is_null_returns_false_for_non_null() {
    assert!(!Object::Integer(0).is_null());
    assert!(!Object::Boolean(false).is_null());
    assert!(!Object::Real(0.0).is_null());
    assert!(!Object::String(vec![]).is_null());
    assert!(!Object::Name("".to_string()).is_null());
    assert!(!Object::Array(vec![]).is_null());
    assert!(!Object::Dictionary(HashMap::new()).is_null());
}

#[test]
fn test_object_ref_display_with_non_zero_gen() {
    let obj_ref = ObjectRef::new(42, 3);
    assert_eq!(format!("{}", obj_ref), "42 3 R");
}

#[test]
fn test_object_ref_equality() {
    let a = ObjectRef::new(5, 0);
    let b = ObjectRef::new(5, 0);
    let c = ObjectRef::new(5, 1);
    let d = ObjectRef::new(6, 0);
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
}

#[test]
fn test_object_ref_copy() {
    let a = ObjectRef::new(10, 0);
    let b = a;
    assert_eq!(a, b);
    assert_eq!(a.id, 10);
}

#[test]
fn test_extract_filter_names_array_with_non_names() {
    let filter = Object::Array(vec![
        Object::Name("FlateDecode".to_string()),
        Object::Integer(42),
        Object::Name("LZWDecode".to_string()),
    ]);
    let names = extract_filter_names(&filter);
    assert_eq!(names, vec!["FlateDecode", "LZWDecode"]);
}

#[test]
fn test_extract_filter_names_empty_array() {
    let filter = Object::Array(vec![]);
    let names = extract_filter_names(&filter);
    assert!(names.is_empty());
}

#[test]
fn test_extract_filter_names_null() {
    let filter = Object::Null;
    let names = extract_filter_names(&filter);
    assert!(names.is_empty());
}

#[test]
fn test_extract_filter_names_boolean() {
    let filter = Object::Boolean(true);
    let names = extract_filter_names(&filter);
    assert!(names.is_empty());
}

#[test]
fn test_extract_decode_params_none() {
    let result = extract_decode_params(None);
    assert!(result.is_none());
}

#[test]
fn test_extract_decode_params_defaults() {
    let dict = Object::Dictionary(HashMap::new());
    let result = extract_decode_params(Some(&dict)).unwrap();
    assert_eq!(result.predictor, 1);
    assert_eq!(result.columns, 1);
    assert_eq!(result.colors, 1);
    assert_eq!(result.bits_per_component, 8);
}

#[test]
fn test_extract_decode_params_custom_values() {
    let mut d = HashMap::new();
    d.insert("Predictor".to_string(), Object::Integer(12));
    d.insert("Columns".to_string(), Object::Integer(800));
    d.insert("Colors".to_string(), Object::Integer(3));
    d.insert("BitsPerComponent".to_string(), Object::Integer(16));
    let dict = Object::Dictionary(d);
    let result = extract_decode_params(Some(&dict)).unwrap();
    assert_eq!(result.predictor, 12);
    assert_eq!(result.columns, 800);
    assert_eq!(result.colors, 3);
    assert_eq!(result.bits_per_component, 16);
}

#[test]
fn test_extract_decode_params_from_array() {
    let mut d = HashMap::new();
    d.insert("Predictor".to_string(), Object::Integer(15));
    d.insert("Columns".to_string(), Object::Integer(640));
    let dict = Object::Dictionary(d);
    let arr = Object::Array(vec![dict]);
    let result = extract_decode_params(Some(&arr)).unwrap();
    assert_eq!(result.predictor, 15);
    assert_eq!(result.columns, 640);
}

#[test]
fn test_extract_decode_params_invalid_type() {
    let obj = Object::Integer(42);
    let result = extract_decode_params(Some(&obj));
    assert!(result.is_none());
}

#[test]
fn test_extract_decode_params_null_object() {
    let obj = Object::Null;
    let result = extract_decode_params(Some(&obj));
    assert!(result.is_none());
}

#[test]
fn test_extract_decode_params_empty_array() {
    let arr = Object::Array(vec![]);
    let result = extract_decode_params(Some(&arr));
    assert!(result.is_none());
}

#[test]
fn test_extract_decode_params_array_with_only_non_dicts() {
    let arr = Object::Array(vec![Object::Integer(1), Object::Null]);
    let result = extract_decode_params(Some(&arr));
    assert!(result.is_none());
}

#[test]
fn test_extract_ccitt_params_none() {
    let result = extract_ccitt_params(None);
    assert!(result.is_none());
}

#[test]
fn test_extract_ccitt_params_defaults() {
    let dict = Object::Dictionary(HashMap::new());
    let result = extract_ccitt_params(Some(&dict)).unwrap();
    assert_eq!(result.k, 0);
    assert_eq!(result.columns, 1);
    assert!(result.rows.is_none());
    assert!(!result.black_is_1);
    assert!(!result.end_of_line);
    assert!(!result.encoded_byte_align);
    assert!(result.end_of_block);
}

#[test]
fn test_extract_ccitt_params_custom_values() {
    let mut d = HashMap::new();
    d.insert("K".to_string(), Object::Integer(0));
    d.insert("Columns".to_string(), Object::Integer(1728));
    d.insert("Rows".to_string(), Object::Integer(2376));
    d.insert("BlackIs1".to_string(), Object::Boolean(true));
    d.insert("EndOfLine".to_string(), Object::Boolean(true));
    d.insert("EncodedByteAlign".to_string(), Object::Boolean(true));
    d.insert("EndOfBlock".to_string(), Object::Boolean(false));
    let dict = Object::Dictionary(d);
    let result = extract_ccitt_params(Some(&dict)).unwrap();
    assert_eq!(result.k, 0);
    assert_eq!(result.columns, 1728);
    assert_eq!(result.rows, Some(2376));
    assert!(result.black_is_1);
    assert!(result.end_of_line);
    assert!(result.encoded_byte_align);
    assert!(!result.end_of_block);
}

#[test]
fn test_extract_ccitt_params_invalid_type() {
    let obj = Object::Integer(42);
    let result = extract_ccitt_params(Some(&obj));
    assert!(result.is_none());
}

#[test]
fn test_extract_ccitt_params_from_array() {
    let mut d = HashMap::new();
    d.insert("K".to_string(), Object::Integer(-1));
    d.insert("Columns".to_string(), Object::Integer(612));
    let dict = Object::Dictionary(d);
    let arr = Object::Array(vec![dict]);
    let result = extract_ccitt_params(Some(&arr)).unwrap();
    assert_eq!(result.k, -1);
    assert_eq!(result.columns, 612);
}

#[test]
fn test_extract_ccitt_params_skips_unrelated_filter_dictionary() {
    let mut ccitt = HashMap::new();
    ccitt.insert("K".to_string(), Object::Integer(-1));
    ccitt.insert("Columns".to_string(), Object::Integer(2542));
    ccitt.insert("Rows".to_string(), Object::Integer(4222));
    ccitt.insert("BlackIs1".to_string(), Object::Boolean(true));
    let params = Object::Array(vec![Object::Dictionary(HashMap::new()), Object::Dictionary(ccitt)]);

    let result = extract_ccitt_params_with_width_at_index(Some(&params), 1, Some(2542)).unwrap();

    assert_eq!(
        result.k, -1,
        "CCITT parameters must not come from the preceding ASCII85 filter"
    );
    assert_eq!(result.columns, 2542);
    assert_eq!(result.rows, Some(4222));
    assert!(result.black_is_1);
}

#[test]
fn test_extract_ccitt_params_with_width_override() {
    let dict = Object::Dictionary(HashMap::new());
    let result = extract_ccitt_params_with_width(Some(&dict), Some(2550)).unwrap();
    assert_eq!(result.columns, 2550);
}

#[test]
fn test_extract_ccitt_params_with_width_columns_takes_precedence() {
    let mut d = HashMap::new();
    d.insert("Columns".to_string(), Object::Integer(1000));
    let dict = Object::Dictionary(d);
    let result = extract_ccitt_params_with_width(Some(&dict), Some(2550)).unwrap();
    assert_eq!(result.columns, 1000);
}

#[test]
fn test_extract_ccitt_params_with_width_none_params() {
    let result = extract_ccitt_params_with_width(None, Some(200));
    assert!(result.is_none());
}

#[test]
fn test_extract_ccitt_params_with_width_no_override_no_columns() {
    let dict = Object::Dictionary(HashMap::new());
    let result = extract_ccitt_params_with_width(Some(&dict), None).unwrap();
    assert_eq!(result.columns, 1);
}

#[test]
fn test_decode_stream_with_decryption_no_decrypt() {
    let mut dict = HashMap::new();
    dict.insert("Length".to_string(), Object::Integer(5));
    let obj = Object::Stream {
        dict,
        data: bytes::Bytes::from_static(b"Hello"),
    };
    let decoded = obj.decode_stream_data_with_decryption(None, 0, 0).unwrap();
    assert_eq!(decoded, b"Hello");
}

#[test]
fn test_decode_stream_with_decryption_fn() {
    let mut dict = HashMap::new();
    dict.insert("Length".to_string(), Object::Integer(5));
    let obj = Object::Stream {
        dict,
        data: bytes::Bytes::from_static(b"Hello"),
    };
    let decrypt_fn = |data: &[u8]| -> Result<Vec<u8>> { Ok(data.to_vec()) };
    let decoded = obj.decode_stream_data_with_decryption(Some(&decrypt_fn), 1, 0).unwrap();
    assert_eq!(decoded, b"Hello");
}

#[test]
fn test_decode_stream_with_decryption_fn_that_transforms() {
    let mut dict = HashMap::new();
    dict.insert("Length".to_string(), Object::Integer(5));
    let obj = Object::Stream {
        dict,
        data: bytes::Bytes::from_static(b"\x01\x02\x03"),
    };
    let decrypt_fn = |data: &[u8]| -> Result<Vec<u8>> { Ok(data.iter().map(|b| b ^ 0xFF).collect()) };
    let decoded = obj.decode_stream_data_with_decryption(Some(&decrypt_fn), 1, 0).unwrap();
    assert_eq!(decoded, vec![0xFE, 0xFD, 0xFC]);
}

#[test]
fn test_decode_stream_with_decryption_fn_error() {
    let mut dict = HashMap::new();
    dict.insert("Length".to_string(), Object::Integer(5));
    let obj = Object::Stream {
        dict,
        data: bytes::Bytes::from_static(b"Hello"),
    };
    let decrypt_fn = |_data: &[u8]| -> Result<Vec<u8>> { Err(Error::InvalidPdf("decrypt fail".into())) };
    let result = obj.decode_stream_data_with_decryption(Some(&decrypt_fn), 1, 0);
    assert!(result.is_err());
}

#[test]
fn test_decode_stream_not_a_stream_with_decryption() {
    let obj = Object::Name("NotAStream".to_string());
    let result = obj.decode_stream_data_with_decryption(None, 0, 0);
    assert!(result.is_err());
    if let Err(Error::InvalidObjectType { expected, found }) = result {
        assert_eq!(expected, "Stream");
        assert_eq!(found, "Name");
    } else {
        panic!("Expected InvalidObjectType error");
    }
}

/// Stream data must be returned verbatim. The parser already
/// consumes the single EOL that follows the `stream` keyword per
/// ISO 32000-1:2008 §7.3.8.1, so `decode_stream_data` must not
/// further strip leading CR/LF bytes — that would corrupt binary
/// streams whose first byte is legitimately 0x0A or 0x0D.
#[test]
fn test_decode_stream_preserves_leading_cr_lf() {
    let mut dict = HashMap::new();
    dict.insert("Length".to_string(), Object::Integer(7));
    let obj = Object::Stream {
        dict,
        data: bytes::Bytes::from_static(b"\r\nHello"),
    };
    let decoded = obj.decode_stream_data().unwrap();
    assert_eq!(decoded, b"\r\nHello");
}

#[test]
fn test_decode_dictionary_as_stream_empty() {
    let dict = HashMap::new();
    let obj = Object::Dictionary(dict);
    let decoded = obj.decode_stream_data().unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn test_decode_stream_with_decode_params() {
    let mut dict = HashMap::new();
    dict.insert("Filter".to_string(), Object::Name("ASCIIHexDecode".to_string()));
    let mut decode_params = HashMap::new();
    decode_params.insert("Predictor".to_string(), Object::Integer(1));
    dict.insert("DecodeParms".to_string(), Object::Dictionary(decode_params));
    let obj = Object::Stream {
        dict,
        data: bytes::Bytes::from_static(b"48656C6C6F"),
    };
    let decoded = obj.decode_stream_data().unwrap();
    assert_eq!(decoded, b"Hello");
}

#[test]
fn test_object_equality() {
    assert_eq!(Object::Null, Object::Null);
    assert_eq!(Object::Boolean(true), Object::Boolean(true));
    assert_ne!(Object::Boolean(true), Object::Boolean(false));
    assert_eq!(Object::Integer(42), Object::Integer(42));
    assert_ne!(Object::Integer(42), Object::Integer(43));
    assert_eq!(Object::String(b"abc".to_vec()), Object::String(b"abc".to_vec()));
    assert_ne!(Object::String(b"abc".to_vec()), Object::String(b"def".to_vec()));
    assert_ne!(Object::Null, Object::Integer(0));
}

#[test]
fn test_as_bool_false() {
    assert_eq!(Object::Boolean(false).as_bool(), Some(false));
}

#[test]
fn test_as_dict_on_stream_returns_stream_dict() {
    let mut dict = HashMap::new();
    dict.insert("Type".to_string(), Object::Name("XObject".to_string()));
    dict.insert("Subtype".to_string(), Object::Name("Image".to_string()));
    let obj = Object::Stream {
        dict: dict.clone(),
        data: bytes::Bytes::from_static(b"image data"),
    };
    let result_dict = obj.as_dict().unwrap();
    assert_eq!(result_dict.len(), 2);
    assert_eq!(result_dict.get("Type").unwrap().as_name(), Some("XObject"));
    assert_eq!(result_dict.get("Subtype").unwrap().as_name(), Some("Image"));
}

#[test]
fn test_encode_pdf_text_string_ascii_unchanged() {
    let bytes = encode_pdf_text_string("Hello, world!");
    assert_eq!(bytes, b"Hello, world!");
}

#[test]
fn test_encode_pdf_text_string_latin1_direct_byte() {
    // é = U+00E9 → byte 0xE9 in PDFDocEncoding (same as Latin-1) ~keep
    let bytes = encode_pdf_text_string("é");
    assert_eq!(bytes, vec![0xE9_u8]);
}

#[test]
fn test_encode_pdf_text_string_portuguese_accents() {
    let bytes = encode_pdf_text_string("Lógico");
    // L=0x4C  ó=0xF3  g=0x67  i=0x69  c=0x63  o=0x6F ~keep
    assert_eq!(bytes, vec![0x4C, 0xF3, 0x67, 0x69, 0x63, 0x6F]);
}

#[test]
fn test_encode_pdf_text_string_all_latin1_supplement() {
    for cp in 0x80_u32..=0xFF {
        let ch = char::from_u32(cp).unwrap();
        let s: String = ch.into();
        let bytes = encode_pdf_text_string(&s);
        assert_eq!(bytes, vec![cp as u8], "failed for U+{:04X}", cp);
    }
}

#[test]
fn test_encode_pdf_text_string_cjk_uses_utf16be_with_bom() {
    let bytes = encode_pdf_text_string("中");
    // BOM 0xFE 0xFF, then U+4E2D as big-endian: 0x4E 0x2D ~keep
    assert_eq!(bytes, vec![0xFE, 0xFF, 0x4E, 0x2D]);
}

#[test]
fn test_encode_pdf_text_string_mixed_triggers_utf16be() {
    let bytes = encode_pdf_text_string("aé中");
    assert_eq!(&bytes[..2], &[0xFE, 0xFF], "must start with BOM");
    // a=0x0061, é=0x00E9, 中=0x4E2D ~keep
    let expected = vec![0xFE, 0xFF, 0x00, 0x61, 0x00, 0xE9, 0x4E, 0x2D];
    assert_eq!(bytes, expected);
}

#[test]
fn test_encode_pdf_text_string_supplementary_plane_surrogate_pair() {
    // 𝄞 (MUSICAL SYMBOL G CLEF) = U+1D11E, encoded in UTF-16 as surrogate pair ~keep
    let bytes = encode_pdf_text_string("𝄞");
    assert_eq!(&bytes[..2], &[0xFE, 0xFF], "must start with BOM");
    // UTF-16BE surrogate pair for U+1D11E: 0xD834 0xDD1E ~keep
    assert_eq!(bytes, vec![0xFE, 0xFF, 0xD8, 0x34, 0xDD, 0x1E]);
}

#[test]
fn test_object_text_string_accepts_str() {
    match Object::text_string("hello") {
        Object::String(b) => assert_eq!(b, b"hello"),
        _ => panic!("expected String"),
    }
}

#[test]
fn test_object_text_string_accepts_owned_string() {
    let s = String::from("Ångström");
    match Object::text_string(s) {
        Object::String(b) => {
            // Å=0xC5  n=0x6E  g=0x67  s=0x73  t=0x74  r=0x72  ö=0xF6  m=0x6D ~keep
            assert_eq!(b, vec![0xC5, 0x6E, 0x67, 0x73, 0x74, 0x72, 0xF6, 0x6D]);
        }
        _ => panic!("expected String"),
    }
}

#[test]
fn test_object_text_string_empty() {
    match Object::text_string("") {
        Object::String(b) => assert!(b.is_empty()),
        _ => panic!("expected String"),
    }
}

#[test]
fn generic_stream_keeps_legacy_predictor_placement() {
    let params = Object::Dictionary(HashMap::from([("Predictor".into(), Object::Integer(15))]));
    let object = Object::Stream {
        dict: HashMap::from([
            (
                "Filter".into(),
                Object::Array(vec![
                    Object::Name("ASCIIHexDecode".into()),
                    Object::Name("RunLengthDecode".into()),
                ]),
            ),
            ("DecodeParms".into(), Object::Array(vec![Object::Null, params])),
        ]),
        data: bytes::Bytes::from_static(b"01004180>"),
    };

    assert_eq!(object.decode_stream_data().unwrap(), b"A");
    assert_eq!(object.decode_image_stream_data(None).unwrap(), b"\0A");
}
