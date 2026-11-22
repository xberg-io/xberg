//! Unit tests for [`super`].
//!
//! Split out of `forms.rs` purely for file size, mirroring the split used for
//! `document.rs`/`document/tests.rs` and `extractors/text/mod.rs`/`extractors/text/tests.rs`. ~keep

use super::*;

#[test]
fn test_parse_field_type() {
    assert_eq!(FormExtractor::parse_field_type("Btn"), FieldType::Button);
    assert_eq!(FormExtractor::parse_field_type("Tx"), FieldType::Text);
    assert_eq!(FormExtractor::parse_field_type("Ch"), FieldType::Choice);
    assert_eq!(FormExtractor::parse_field_type("Sig"), FieldType::Signature);
    assert!(matches!(
        FormExtractor::parse_field_type("Unknown"),
        FieldType::Unknown(_)
    ));
}

#[test]
fn test_parse_field_value_string() {
    let obj = Object::String(b"John Doe".to_vec());
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Text);
    assert!(matches!(value, FieldValue::Text(ref s) if s == "John Doe"));
}

#[test]
fn test_parse_field_value_boolean() {
    let obj = Object::Name("Yes".to_string());
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Button);
    assert!(matches!(value, FieldValue::Boolean(true)));

    let obj = Object::Name("Off".to_string());
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Button);
    assert!(matches!(value, FieldValue::Boolean(false)));
}

#[test]
fn test_parse_field_value_array() {
    let obj = Object::Array(vec![
        Object::String(b"Option1".to_vec()),
        Object::String(b"Option2".to_vec()),
    ]);
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Choice);
    assert!(matches!(
        value,
        FieldValue::Array(ref v) if v.len() == 2 && v[0] == "Option1"
    ));
}

#[test]
fn test_form_field_construction_minimal() {
    let field = FormField {
        name: "field1".to_string(),
        field_type: FieldType::Text,
        value: FieldValue::None,
        tooltip: None,
        full_name: "field1".to_string(),
        bounds: None,
        object_ref: None,
        flags: None,
        default_value: None,
        max_length: None,
        alignment: None,
        default_appearance: None,
        border_style: None,
        appearance_chars: None,
    };
    assert_eq!(field.name, "field1");
    assert_eq!(field.field_type, FieldType::Text);
    assert_eq!(field.value, FieldValue::None);
    assert!(field.tooltip.is_none());
    assert!(field.bounds.is_none());
    assert!(field.object_ref.is_none());
    assert!(field.flags.is_none());
}

#[test]
fn test_form_field_construction_full() {
    let field = FormField {
        name: "name".to_string(),
        field_type: FieldType::Text,
        value: FieldValue::Text("John".to_string()),
        tooltip: Some("Enter name".to_string()),
        full_name: "form.name".to_string(),
        bounds: Some([10.0, 20.0, 200.0, 40.0]),
        object_ref: Some(ObjectRef::new(5, 0)),
        flags: Some(field_flags::REQUIRED),
        default_value: Some(FieldValue::Text("Default".to_string())),
        max_length: Some(100),
        alignment: Some(1),
        default_appearance: Some("/Helv 12 Tf 0 g".to_string()),
        border_style: Some(BorderStyle::default()),
        appearance_chars: Some(AppearanceCharacteristics::default()),
    };
    assert_eq!(field.full_name, "form.name");
    assert_eq!(field.bounds.unwrap()[0], 10.0);
    assert_eq!(field.object_ref.unwrap().id, 5);
    assert_eq!(field.flags.unwrap(), field_flags::REQUIRED);
    assert_eq!(field.max_length.unwrap(), 100);
    assert_eq!(field.alignment.unwrap(), 1);
    assert!(field.default_appearance.is_some());
    assert!(field.border_style.is_some());
    assert!(field.appearance_chars.is_some());
}

#[test]
fn test_field_type_equality() {
    assert_eq!(FieldType::Button, FieldType::Button);
    assert_eq!(FieldType::Text, FieldType::Text);
    assert_eq!(FieldType::Choice, FieldType::Choice);
    assert_eq!(FieldType::Signature, FieldType::Signature);
    assert_ne!(FieldType::Button, FieldType::Text);
}

#[test]
fn test_field_type_unknown_variant() {
    let unknown = FieldType::Unknown("Custom".to_string());
    assert!(matches!(unknown, FieldType::Unknown(ref s) if s == "Custom"));
    assert_eq!(FieldType::Unknown("X".to_string()), FieldType::Unknown("X".to_string()));
    assert_ne!(FieldType::Unknown("X".to_string()), FieldType::Unknown("Y".to_string()));
}

#[test]
fn test_field_type_clone() {
    let ft = FieldType::Button;
    let cloned = ft.clone();
    assert_eq!(ft, cloned);
}

#[test]
fn test_field_value_text() {
    let val = FieldValue::Text("hello".to_string());
    assert!(matches!(val, FieldValue::Text(ref s) if s == "hello"));
}

#[test]
fn test_field_value_boolean_true() {
    let val = FieldValue::Boolean(true);
    assert!(matches!(val, FieldValue::Boolean(true)));
}

#[test]
fn test_field_value_boolean_false() {
    let val = FieldValue::Boolean(false);
    assert!(matches!(val, FieldValue::Boolean(false)));
}

#[test]
fn test_field_value_name() {
    let val = FieldValue::Name("Option1".to_string());
    assert!(matches!(val, FieldValue::Name(ref s) if s == "Option1"));
}

#[test]
fn test_field_value_array_empty() {
    let val = FieldValue::Array(vec![]);
    assert!(matches!(val, FieldValue::Array(ref v) if v.is_empty()));
}

#[test]
fn test_field_value_none() {
    let val = FieldValue::None;
    assert!(matches!(val, FieldValue::None));
}

#[test]
fn test_field_value_clone() {
    let val = FieldValue::Text("test".to_string());
    let cloned = val.clone();
    assert_eq!(val, cloned);
}

#[test]
fn test_parse_field_value_button_on() {
    let obj = Object::Name("On".to_string());
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Button);
    assert!(matches!(value, FieldValue::Boolean(true)));
}

#[test]
fn test_parse_field_value_button_no() {
    // /No is a real on-state (only /Off means unselected, ISO 32000-1
    // §12.7.4.2.3): the export value must survive. ~keep
    let obj = Object::Name("No".to_string());
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Button);
    assert!(matches!(value, FieldValue::Name(ref s) if s == "No"));
}

#[test]
fn test_parse_field_value_button_custom_name() {
    let obj = Object::Name("RadioOption3".to_string());
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Button);
    assert!(matches!(value, FieldValue::Name(ref s) if s == "RadioOption3"));
}

#[test]
fn test_parse_field_value_name_for_choice() {
    let obj = Object::Name("SelectedItem".to_string());
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Choice);
    assert!(matches!(value, FieldValue::Name(ref s) if s == "SelectedItem"));
}

#[test]
fn test_parse_field_value_name_for_text_field() {
    let obj = Object::Name("SomeName".to_string());
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Text);
    assert!(matches!(value, FieldValue::Name(ref s) if s == "SomeName"));
}

#[test]
fn test_parse_field_value_boolean_object() {
    let obj = Object::Boolean(true);
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Button);
    assert!(matches!(value, FieldValue::Boolean(true)));

    let obj = Object::Boolean(false);
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Button);
    assert!(matches!(value, FieldValue::Boolean(false)));
}

#[test]
fn test_parse_field_value_null_returns_none() {
    let obj = Object::Null;
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Text);
    assert!(matches!(value, FieldValue::None));
}

#[test]
fn test_parse_field_value_integer_returns_none() {
    let obj = Object::Integer(42);
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Text);
    assert!(matches!(value, FieldValue::None));
}

#[test]
fn test_parse_field_value_array_with_names() {
    let obj = Object::Array(vec![
        Object::Name("Item1".to_string()),
        Object::Name("Item2".to_string()),
    ]);
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Choice);
    match value {
        FieldValue::Array(v) => {
            assert_eq!(v.len(), 2);
            assert_eq!(v[0], "Item1");
            assert_eq!(v[1], "Item2");
        }
        _ => panic!("Expected Array"),
    }
}

#[test]
fn test_parse_field_value_array_mixed_types() {
    let obj = Object::Array(vec![
        Object::String(b"Text".to_vec()),
        Object::Integer(42),
        Object::Name("Name".to_string()),
    ]);
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Choice);
    match value {
        FieldValue::Array(v) => {
            assert_eq!(v.len(), 2);
            assert_eq!(v[0], "Text");
            assert_eq!(v[1], "Name");
        }
        _ => panic!("Expected Array"),
    }
}

#[test]
fn test_border_style_default() {
    let bs = BorderStyle::default();
    assert_eq!(bs.width, 1.0);
    assert_eq!(bs.style, BorderStyleType::Solid);
    assert!(bs.dash_array.is_none());
}

#[test]
fn test_border_style_custom() {
    let bs = BorderStyle {
        width: 2.5,
        style: BorderStyleType::Dashed,
        dash_array: Some(vec![3, 2]),
    };
    assert_eq!(bs.width, 2.5);
    assert_eq!(bs.style, BorderStyleType::Dashed);
    assert_eq!(bs.dash_array.as_ref().unwrap(), &vec![3, 2]);
}

#[test]
fn test_border_style_type_from_pdf_name_all_variants() {
    assert_eq!(BorderStyleType::from_pdf_name("S"), BorderStyleType::Solid);
    assert_eq!(BorderStyleType::from_pdf_name("D"), BorderStyleType::Dashed);
    assert_eq!(BorderStyleType::from_pdf_name("B"), BorderStyleType::Beveled);
    assert_eq!(BorderStyleType::from_pdf_name("I"), BorderStyleType::Inset);
    assert_eq!(BorderStyleType::from_pdf_name("U"), BorderStyleType::Underline);
}

#[test]
fn test_border_style_type_from_pdf_name_unknown() {
    assert_eq!(BorderStyleType::from_pdf_name("X"), BorderStyleType::Solid);
    assert_eq!(BorderStyleType::from_pdf_name(""), BorderStyleType::Solid);
}

#[test]
fn test_border_style_type_to_pdf_name() {
    assert_eq!(BorderStyleType::Solid.to_pdf_name(), "S");
    assert_eq!(BorderStyleType::Dashed.to_pdf_name(), "D");
    assert_eq!(BorderStyleType::Beveled.to_pdf_name(), "B");
    assert_eq!(BorderStyleType::Inset.to_pdf_name(), "I");
    assert_eq!(BorderStyleType::Underline.to_pdf_name(), "U");
}

#[test]
fn test_border_style_type_roundtrip() {
    let variants = vec![
        BorderStyleType::Solid,
        BorderStyleType::Dashed,
        BorderStyleType::Beveled,
        BorderStyleType::Inset,
        BorderStyleType::Underline,
    ];
    for v in variants {
        let name = v.to_pdf_name();
        let back = BorderStyleType::from_pdf_name(name);
        assert_eq!(v, back);
    }
}

#[test]
fn test_border_style_type_default() {
    let bst: BorderStyleType = Default::default();
    assert_eq!(bst, BorderStyleType::Solid);
}

#[test]
fn test_appearance_characteristics_default() {
    let ac = AppearanceCharacteristics::default();
    assert!(ac.background_color.is_none());
    assert!(ac.border_color.is_none());
    assert!(ac.caption.is_none());
    assert!(ac.rollover_caption.is_none());
    assert!(ac.alternate_caption.is_none());
    assert!(ac.rotation.is_none());
}

#[test]
fn test_appearance_characteristics_custom() {
    let ac = AppearanceCharacteristics {
        background_color: Some([1.0, 0.0, 0.0]),
        border_color: Some([0.0, 0.0, 1.0]),
        caption: Some("OK".to_string()),
        rollover_caption: Some("Hover".to_string()),
        alternate_caption: Some("Pressed".to_string()),
        rotation: Some(90),
    };
    assert_eq!(ac.background_color.unwrap(), [1.0, 0.0, 0.0]);
    assert_eq!(ac.border_color.unwrap(), [0.0, 0.0, 1.0]);
    assert_eq!(ac.caption.as_deref(), Some("OK"));
    assert_eq!(ac.rollover_caption.as_deref(), Some("Hover"));
    assert_eq!(ac.alternate_caption.as_deref(), Some("Pressed"));
    assert_eq!(ac.rotation.unwrap(), 90);
}

#[test]
fn test_parse_border_style_full() {
    use std::collections::HashMap;
    let mut dict = HashMap::new();
    dict.insert("W".to_string(), Object::Real(2.0));
    dict.insert("S".to_string(), Object::Name("D".to_string()));
    dict.insert(
        "D".to_string(),
        Object::Array(vec![Object::Integer(3), Object::Integer(1)]),
    );
    let obj = Object::Dictionary(dict);
    let bs = FormExtractor::parse_border_style(&obj).unwrap();
    assert_eq!(bs.width, 2.0);
    assert_eq!(bs.style, BorderStyleType::Dashed);
    assert_eq!(bs.dash_array.unwrap(), vec![3, 1]);
}

#[test]
fn test_parse_border_style_integer_width() {
    use std::collections::HashMap;
    let mut dict = HashMap::new();
    dict.insert("W".to_string(), Object::Integer(3));
    let obj = Object::Dictionary(dict);
    let bs = FormExtractor::parse_border_style(&obj).unwrap();
    assert_eq!(bs.width, 3.0);
    assert_eq!(bs.style, BorderStyleType::Solid);
}

#[test]
fn test_parse_border_style_defaults() {
    use std::collections::HashMap;
    let dict = HashMap::new();
    let obj = Object::Dictionary(dict);
    let bs = FormExtractor::parse_border_style(&obj).unwrap();
    assert_eq!(bs.width, 1.0);
    assert_eq!(bs.style, BorderStyleType::Solid);
    assert!(bs.dash_array.is_none());
}

#[test]
fn test_parse_border_style_not_dict() {
    let obj = Object::Integer(42);
    let result = FormExtractor::parse_border_style(&obj);
    assert!(result.is_none());
}

#[test]
fn test_field_flags_read_only() {
    assert_eq!(field_flags::READ_ONLY, 1);
}

#[test]
fn test_field_flags_required() {
    assert_eq!(field_flags::REQUIRED, 2);
}

#[test]
fn test_field_flags_no_export() {
    assert_eq!(field_flags::NO_EXPORT, 4);
}

#[test]
fn test_field_flags_combined() {
    let flags = field_flags::READ_ONLY | field_flags::REQUIRED;
    assert_eq!(flags, 3);
    assert!(flags & field_flags::READ_ONLY != 0);
    assert!(flags & field_flags::REQUIRED != 0);
    assert!(flags & field_flags::NO_EXPORT == 0);
}

#[test]
fn test_field_flags_button_flags() {
    assert_eq!(field_flags::PUSH_BUTTON, 1 << 16);
    assert_eq!(field_flags::RADIO, 1 << 15);
}

#[test]
fn test_field_flags_text_flags() {
    assert_eq!(field_flags::MULTILINE, 1 << 12);
    assert_eq!(field_flags::PASSWORD, 1 << 13);
    assert_eq!(field_flags::DO_NOT_SCROLL, 1 << 23);
    assert_eq!(field_flags::COMB, 1 << 24);
    assert_eq!(field_flags::RICH_TEXT, 1 << 25);
}

#[test]
fn test_field_flags_choice_flags() {
    assert_eq!(field_flags::COMBO, 1 << 17);
    assert_eq!(field_flags::EDIT, 1 << 18);
    assert_eq!(field_flags::SORT, 1 << 19);
    assert_eq!(field_flags::MULTI_SELECT, 1 << 21);
    assert_eq!(field_flags::DO_NOT_SPELL_CHECK, 1 << 22);
    assert_eq!(field_flags::COMMIT_ON_SEL_CHANGE, 1 << 26);
}

#[test]
fn test_decode_text_string_ascii() {
    let bytes = b"Hello World";
    let result = FormExtractor::decode_text_string(bytes);
    assert_eq!(result, Some("Hello World".to_string()));
}

#[test]
fn test_decode_text_string_utf16be() {
    let bytes = vec![0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69];
    let result = FormExtractor::decode_text_string(&bytes);
    assert_eq!(result, Some("Hi".to_string()));
}

#[test]
fn test_decode_text_string_empty() {
    let bytes = b"";
    let result = FormExtractor::decode_text_string(bytes);
    assert_eq!(result, Some("".to_string()));
}

#[test]
fn test_decode_text_string_utf16be_empty_after_bom() {
    let bytes = vec![0xFE, 0xFF];
    let result = FormExtractor::decode_text_string(&bytes);
    assert_eq!(result, Some("".to_string()));
}

#[test]
fn test_form_field_clone() {
    let field = FormField {
        name: "test".to_string(),
        field_type: FieldType::Text,
        value: FieldValue::Text("val".to_string()),
        tooltip: Some("tip".to_string()),
        full_name: "test".to_string(),
        bounds: Some([0.0, 0.0, 100.0, 20.0]),
        object_ref: None,
        flags: Some(1),
        default_value: None,
        max_length: Some(50),
        alignment: Some(0),
        default_appearance: None,
        border_style: None,
        appearance_chars: None,
    };
    let cloned = field.clone();
    assert_eq!(cloned.name, "test");
    assert_eq!(cloned.flags, Some(1));
    assert_eq!(cloned.max_length, Some(50));
}

#[test]
fn test_form_field_debug() {
    let field = FormField {
        name: "f".to_string(),
        field_type: FieldType::Signature,
        value: FieldValue::None,
        tooltip: None,
        full_name: "f".to_string(),
        bounds: None,
        object_ref: None,
        flags: None,
        default_value: None,
        max_length: None,
        alignment: None,
        default_appearance: None,
        border_style: None,
        appearance_chars: None,
    };
    let debug_str = format!("{:?}", field);
    assert!(debug_str.contains("Signature"));
}

#[test]
fn test_parse_field_type_all_known() {
    assert_eq!(FormExtractor::parse_field_type("Btn"), FieldType::Button);
    assert_eq!(FormExtractor::parse_field_type("Tx"), FieldType::Text);
    assert_eq!(FormExtractor::parse_field_type("Ch"), FieldType::Choice);
    assert_eq!(FormExtractor::parse_field_type("Sig"), FieldType::Signature);
}

#[test]
fn test_parse_field_type_preserves_unknown_name() {
    match FormExtractor::parse_field_type("FooBar") {
        FieldType::Unknown(s) => assert_eq!(s, "FooBar"),
        _ => panic!("Expected Unknown variant"),
    }
}

#[test]
fn test_parse_field_value_utf16be_string() {
    let bytes = vec![0xFE, 0xFF, 0x00, 0x41, 0x00, 0x42];
    let obj = Object::String(bytes);
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Text);
    assert!(matches!(value, FieldValue::Text(ref s) if s == "AB"));
}

#[test]
fn test_parse_field_value_real_returns_none() {
    let obj = Object::Real(std::f64::consts::PI);
    let value = FormExtractor::parse_field_value(&obj, &FieldType::Text);
    assert!(matches!(value, FieldValue::None));
}

#[test]
fn inherited_lookups_do_not_deep_copy_the_parent() {
    const KIDS: u32 = 50;
    let kid_refs: String = (5..5 + KIDS).map(|id| format!("{id} 0 R ")).collect();
    let mut objects: Vec<(u32, String)> = vec![
        (
            1,
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>".to_string(),
        ),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>".to_string()),
        (
            4,
            format!("<< /T (P) /FT /Tx /V (v) /DV (d) /DA (/Helv 0 Tf) /Kids [{kid_refs}] >>"),
        ),
    ];
    objects.extend((5..5 + KIDS).map(|id| (id, "<< /Parent 4 0 R /Rect [0 0 1 1] >>".to_string())));
    let max_id = 4 + KIDS;
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0usize; max_id as usize + 1];
    for (id, body) in &objects {
        offsets[*id as usize] = pdf.len();
        pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", max_id + 1).as_bytes());
    for offset in &offsets[1..] {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            max_id + 1
        )
        .as_bytes(),
    );

    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let before = doc.cached_object_clone_count();
    let fields = FormExtractor::extract_fields(&doc).expect("fields");
    let clones = doc.cached_object_clone_count() - before;
    assert_eq!(
        fields.iter().filter(|f| f.default_appearance.is_some()).count(),
        KIDS as usize + 1
    );
    assert!(
        clones <= 3 * KIDS as usize,
        "inherited lookups cloned cached objects {clones} times for {KIDS} kids"
    );
}
