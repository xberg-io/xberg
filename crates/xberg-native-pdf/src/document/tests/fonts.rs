use super::super::*;
use super::common::*;
use super::pdf_fixtures::*;

/// Two subsets of one base font must dedup to ONE entry, and that entry
/// must be the SAME bytes every time - the bug this fix exists for.
///
/// `get_font_set()` hands the subsets back in `HashMap` order, and each
/// call builds a fresh map whose iteration order is independently seeded,
/// so the old `or_insert` returned a different subset from run to run for
/// one unchanged PDF. Extracting repeatedly makes that flake fatal rather
/// than occasional; swapping which resource slot holds the larger program
/// shows the choice is driven by the total order, not by encounter order.
#[test]
fn embedded_font_subset_choice_is_deterministic() {
    for big_in_f1 in [true, false] {
        let (pdf, small, big) = build_pdf_with_two_font_subsets(big_in_f1);
        let mut previous: Option<Vec<u8>> = None;

        for round in 0..64 {
            let doc = PdfDocument::from_bytes(pdf.clone()).expect("open two-subset pdf");
            let fonts = doc.extract_embedded_fonts().expect("extract fonts");

            assert_eq!(
                fonts.len(),
                1,
                "the two subsets share a base name and must dedup to one entry \
                     (big_in_f1={big_in_f1}, round={round})"
            );
            let (name, bytes) = &fonts[0];
            assert_eq!(name, "Helvetica", "the subset prefix must be stripped");
            assert_eq!(
                bytes, &big,
                "the LARGER subset must win regardless of which slot holds it \
                     (big_in_f1={big_in_f1}, round={round})"
            );
            assert_ne!(bytes, &small);

            if let Some(prev) = &previous {
                assert_eq!(
                    prev, bytes,
                    "repeated extraction of one unchanged PDF must be byte-identical \
                         (big_in_f1={big_in_f1}, round={round})"
                );
            }
            previous = Some(bytes.clone());
        }
    }
}

/// The same guarantee on the variant that also returns the Unicode/width
/// maps: it carries its own copy of the subset choice, so it needs its own
/// guard against regressing back to `or_insert`.
#[test]
fn embedded_font_subset_choice_is_deterministic_with_maps() {
    let (pdf, small, big) = build_pdf_with_two_font_subsets(true);
    let mut previous: Option<Vec<u8>> = None;

    for round in 0..64 {
        let doc = PdfDocument::from_bytes(pdf.clone()).expect("open two-subset pdf");
        let fonts = doc
            .extract_embedded_fonts_with_unicode_maps_and_widths()
            .expect("extract fonts with maps");

        assert_eq!(fonts.len(), 1, "must dedup to one entry (round={round})");
        let (name, bytes, _uni, _widths) = &fonts[0];
        assert_eq!(name, "Helvetica");
        assert_eq!(bytes, &big, "the LARGER subset must win (round={round})");
        assert_ne!(bytes, &small);

        if let Some(prev) = &previous {
            assert_eq!(prev, bytes, "repeated extraction must be stable (round={round})");
        }
        previous = Some(bytes.clone());
    }
}

#[test]
fn test_font_identity_hash_same_font() {
    let mut dict1 = std::collections::HashMap::new();
    dict1.insert("BaseFont".to_string(), Object::Name("Helvetica".to_string()));
    dict1.insert("Subtype".to_string(), Object::Name("Type1".to_string()));

    let mut dict2 = std::collections::HashMap::new();
    dict2.insert("BaseFont".to_string(), Object::Name("Helvetica".to_string()));
    dict2.insert("Subtype".to_string(), Object::Name("Type1".to_string()));

    let hash1 = PdfDocument::font_identity_hash_cheap(&Object::Dictionary(dict1));
    let hash2 = PdfDocument::font_identity_hash_cheap(&Object::Dictionary(dict2));
    assert_eq!(hash1, hash2);
}

#[test]
fn test_font_identity_hash_different_fonts() {
    let mut dict1 = std::collections::HashMap::new();
    dict1.insert("BaseFont".to_string(), Object::Name("Helvetica".to_string()));

    let mut dict2 = std::collections::HashMap::new();
    dict2.insert("BaseFont".to_string(), Object::Name("Times-Roman".to_string()));

    let hash1 = PdfDocument::font_identity_hash_cheap(&Object::Dictionary(dict1));
    let hash2 = PdfDocument::font_identity_hash_cheap(&Object::Dictionary(dict2));
    assert_ne!(hash1, hash2);
}

#[test]
fn test_font_identity_hash_null_object() {
    let hash = PdfDocument::font_identity_hash_cheap(&Object::Null);
    // Should not panic, returns some hash ~keep
    let _ = hash;
}

// Two non-subset fonts sharing BaseFont/Subtype/Encoding but with
// different /Widths must NOT share a cross-document cache key. ~keep
#[test]
fn test_font_identity_hash_differs_on_widths() {
    let base = || {
        let mut d = std::collections::HashMap::new();
        d.insert("BaseFont".to_string(), Object::Name("Helvetica".to_string()));
        d.insert("Subtype".to_string(), Object::Name("Type1".to_string()));
        d.insert("FirstChar".to_string(), Object::Integer(65));
        d.insert("LastChar".to_string(), Object::Integer(67));
        d
    };
    let mut a = base();
    a.insert(
        "Widths".to_string(),
        Object::Array(vec![Object::Integer(600), Object::Integer(600), Object::Integer(600)]),
    );
    let mut b = base();
    b.insert(
        "Widths".to_string(),
        Object::Array(vec![Object::Integer(667), Object::Integer(667), Object::Integer(722)]),
    );

    let hash_a = PdfDocument::font_identity_hash_cheap(&Object::Dictionary(a));
    let hash_b = PdfDocument::font_identity_hash_cheap(&Object::Dictionary(b));
    assert_ne!(
        hash_a, hash_b,
        "fonts with identical BaseFont but different /Widths must not collide"
    );

    let mut c = base();
    c.insert(
        "Widths".to_string(),
        Object::Array(vec![Object::Integer(600), Object::Integer(600), Object::Integer(600)]),
    );
    let mut a2 = base();
    a2.insert(
        "Widths".to_string(),
        Object::Array(vec![Object::Integer(600), Object::Integer(600), Object::Integer(600)]),
    );
    assert_eq!(
        PdfDocument::font_identity_hash_cheap(&Object::Dictionary(c)),
        PdfDocument::font_identity_hash_cheap(&Object::Dictionary(a2)),
        "identical fonts must still share a cache key"
    );
}

// Vertical metrics live on the descendant CIDFont. Their resolved content,
// never the PDF-local object number, determines shared font identity. ~keep
#[test]
fn vertical_metrics_differentiate_font_cache_key() {
    let doc = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let same_w2 = Object::Array(vec![
        Object::Integer(1),
        Object::Integer(3),
        Object::Array(vec![Object::Integer(880), Object::Integer(-500), Object::Integer(500)]),
    ]);
    let different_w2 = Object::Array(vec![
        Object::Integer(1),
        Object::Integer(3),
        Object::Array(vec![Object::Integer(880), Object::Integer(-600), Object::Integer(600)]),
    ]);
    doc.object_cache
        .lock_or_recover()
        .insert(ObjectRef::new(100, 0), same_w2.clone());
    doc.object_cache
        .lock_or_recover()
        .insert(ObjectRef::new(200, 0), same_w2);
    doc.object_cache
        .lock_or_recover()
        .insert(ObjectRef::new(201, 0), different_w2);
    let font = |w2_reference, default_vertical_advance| {
        let descendant = Object::Dictionary(std::collections::HashMap::from([
            ("Subtype".to_string(), Object::Name("CIDFontType2".to_string())),
            (
                "DW2".to_string(),
                Object::Array(vec![Object::Integer(880), Object::Integer(default_vertical_advance)]),
            ),
            ("W2".to_string(), Object::Reference(w2_reference)),
        ]));
        Object::Dictionary(std::collections::HashMap::from([
            ("BaseFont".to_string(), Object::Name("Identity-CIDFont".to_string())),
            ("Subtype".to_string(), Object::Name("Type0".to_string())),
            ("DescendantFonts".to_string(), Object::Array(vec![descendant])),
        ]))
    };

    let hash_100 = doc.font_identity_hash_with_descendants(&font(ObjectRef::new(100, 0), -1000));
    let hash_200 = doc.font_identity_hash_with_descendants(&font(ObjectRef::new(200, 0), -1000));
    let different_w2_hash = doc.font_identity_hash_with_descendants(&font(ObjectRef::new(201, 0), -1000));
    let different_dw2_hash = doc.font_identity_hash_with_descendants(&font(ObjectRef::new(100, 0), -880));

    assert_eq!(
        hash_100, hash_200,
        "identical indirect /W2 content at different object numbers must share a cache key"
    );
    assert_ne!(
        hash_100, different_w2_hash,
        "different descendant /W2 content must not share a cache key"
    );
    assert_ne!(
        hash_100, different_dw2_hash,
        "different descendant /DW2 content must not share a cache key"
    );
}

// Type 3 fonts are document-local and must be kept out of the
// cross-document global font cache (Layer 6). The gate uses
// font_is_document_local; pin its classification here. ~keep
#[test]
fn test_type3_font_is_document_local() {
    let mut type3 = std::collections::HashMap::new();
    type3.insert("Subtype".to_string(), Object::Name("Type3".to_string()));
    type3.insert("Name".to_string(), Object::Name("F1".to_string()));
    assert!(
        PdfDocument::font_is_document_local(&Object::Dictionary(type3)),
        "Type3 fonts must be treated as document-local (uncacheable cross-document)"
    );

    for subtype in ["Type1", "TrueType", "Type0", "CIDFontType2"] {
        let mut d = std::collections::HashMap::new();
        d.insert("Subtype".to_string(), Object::Name(subtype.to_string()));
        d.insert("BaseFont".to_string(), Object::Name("Helvetica".to_string()));
        assert!(
            !PdfDocument::font_is_document_local(&Object::Dictionary(d)),
            "{subtype} must remain cacheable across documents"
        );
    }
    assert!(!PdfDocument::font_is_document_local(&Object::Null));

    // Subset fonts (six uppercase letters + '+', ISO 32000-1 §9.6.4) are
    // document-local regardless of subtype — their glyph subset and
    // ToUnicode are document-specific and must not be shared cross-document. ~keep
    for subtype in ["Type1", "TrueType", "Type0", "CIDFontType2"] {
        let mut d = std::collections::HashMap::new();
        d.insert("Subtype".to_string(), Object::Name(subtype.to_string()));
        d.insert(
            "BaseFont".to_string(),
            Object::Name("AAAAAA+ArialUnicodeMS".to_string()),
        );
        assert!(
            PdfDocument::font_is_document_local(&Object::Dictionary(d)),
            "subset {subtype} must be treated as document-local"
        );
    }

    // Subset-prefix edge cases: a 6-uppercase name without '+', a lowercase
    // tag, a short tag, and an empty real name are NOT subsets — stay cacheable. ~keep
    for name in ["ARIALX", "abcdef+Real", "AAAAA+Short", "AAAAAA+"] {
        let mut d = std::collections::HashMap::new();
        d.insert("Subtype".to_string(), Object::Name("Type0".to_string()));
        d.insert("BaseFont".to_string(), Object::Name(name.to_string()));
        assert!(
            !PdfDocument::font_is_document_local(&Object::Dictionary(d)),
            "{name} is not a subset tag and must remain cacheable"
        );
    }

    // A non-Type3 font missing /BaseFont fails safe to document-local. ~keep
    let mut no_basefont = std::collections::HashMap::new();
    no_basefont.insert("Subtype".to_string(), Object::Name("Type0".to_string()));
    assert!(
        PdfDocument::font_is_document_local(&Object::Dictionary(no_basefont)),
        "a non-Type3 font with no /BaseFont must fail safe to document-local"
    );
}

#[test]
fn test_font_identity_hash_with_encoding_dict() {
    let mut font_dict = std::collections::HashMap::new();
    font_dict.insert("BaseFont".to_string(), Object::Name("Helvetica".to_string()));
    font_dict.insert("Subtype".to_string(), Object::Name("Type1".to_string()));
    let mut enc = std::collections::HashMap::new();
    enc.insert("Type".to_string(), Object::Name("Encoding".to_string()));
    font_dict.insert("Encoding".to_string(), Object::Dictionary(enc));
    assert_ne!(PdfDocument::font_identity_hash_cheap(&Object::Dictionary(font_dict)), 0);
}

#[test]
fn test_font_identity_hash_with_encoding_ref() {
    let mut font_dict = std::collections::HashMap::new();
    font_dict.insert("BaseFont".to_string(), Object::Name("Helvetica".to_string()));
    font_dict.insert("Encoding".to_string(), Object::Reference(ObjectRef::new(99, 0)));
    assert_ne!(PdfDocument::font_identity_hash_cheap(&Object::Dictionary(font_dict)), 0);
}

#[test]
fn test_font_identity_hash_tounicode_changes_hash() {
    let mut d1 = std::collections::HashMap::new();
    d1.insert("BaseFont".to_string(), Object::Name("Arial".to_string()));
    d1.insert("ToUnicode".to_string(), Object::Reference(ObjectRef::new(50, 0)));
    let h1 = PdfDocument::font_identity_hash_cheap(&Object::Dictionary(d1));

    let mut d2 = std::collections::HashMap::new();
    d2.insert("BaseFont".to_string(), Object::Name("Arial".to_string()));
    let h2 = PdfDocument::font_identity_hash_cheap(&Object::Dictionary(d2));
    assert_ne!(h1, h2);
}

#[test]
fn test_font_identity_hash_with_descendant_fonts() {
    let mut d = std::collections::HashMap::new();
    d.insert("BaseFont".to_string(), Object::Name("CIDFont".to_string()));
    d.insert("Subtype".to_string(), Object::Name("Type0".to_string()));
    d.insert(
        "DescendantFonts".to_string(),
        Object::Array(vec![Object::Reference(ObjectRef::new(20, 0))]),
    );
    assert_ne!(PdfDocument::font_identity_hash_cheap(&Object::Dictionary(d)), 0);
}

// Regression: two same-named, non-embedded simple fonts whose /Encoding are
// REFERENCES to different /Differences arrays must not share an identity
// hash. The cheap hash folds only a constant marker for a referenced
// /Encoding, so without folding the referenced encoding's CONTENT they
// collide and the second font decodes through the first's /Differences (a
// substitution-cipher scramble). font_identity_hash_with_descendants must
// distinguish them. ~keep
#[test]
fn test_font_identity_hash_folds_referenced_encoding_differences() {
    let doc = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();

    let enc = |names: &[&str]| {
        let mut diffs = vec![Object::Integer(1)];
        diffs.extend(names.iter().map(|n| Object::Name((*n).to_string())));
        let mut d = std::collections::HashMap::new();
        d.insert("Type".to_string(), Object::Name("Encoding".to_string()));
        d.insert("Differences".to_string(), Object::Array(diffs));
        Object::Dictionary(d)
    };
    doc.object_cache
        .lock_or_recover()
        .insert(ObjectRef::new(100, 0), enc(&["T", "h", "i", "s"]));
    doc.object_cache
        .lock_or_recover()
        .insert(ObjectRef::new(101, 0), enc(&["one", "T", "h", "e"]));

    let font = |enc_ref: u32| {
        let mut f = std::collections::HashMap::new();
        f.insert("BaseFont".to_string(), Object::Name("Times-Roman".to_string()));
        f.insert("Subtype".to_string(), Object::Name("Type1".to_string()));
        f.insert("Encoding".to_string(), Object::Reference(ObjectRef::new(enc_ref, 0)));
        Object::Dictionary(f)
    };

    let h100 = doc.font_identity_hash_with_descendants(&font(100));
    let h101 = doc.font_identity_hash_with_descendants(&font(101));
    assert_ne!(
        h100, h101,
        "fonts with different referenced /Differences must not collide"
    );

    assert_eq!(
        doc.font_identity_hash_with_descendants(&font(100)),
        doc.font_identity_hash_with_descendants(&font(100)),
    );
}

#[test]
fn font_identity_hash_ignores_document_local_reference_numbers() {
    let doc = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let cmap = Object::Stream {
        dict: std::collections::HashMap::new(),
        data: bytes::Bytes::from_static(b"same semantic cmap"),
    };
    doc.object_cache
        .lock_or_recover()
        .insert(ObjectRef::new(100, 0), cmap.clone());
    doc.object_cache.lock_or_recover().insert(ObjectRef::new(200, 0), cmap);
    let font = |reference| {
        Object::Dictionary(std::collections::HashMap::from([
            ("BaseFont".to_string(), Object::Name("CIDFont+F1".to_string())),
            ("Subtype".to_string(), Object::Name("Type0".to_string())),
            ("ToUnicode".to_string(), Object::Reference(reference)),
        ]))
    };

    assert_eq!(
        doc.font_identity_hash_with_descendants(&font(ObjectRef::new(100, 0))),
        doc.font_identity_hash_with_descendants(&font(ObjectRef::new(200, 0))),
        "resolved semantic content, not a document-local object number, defines font identity"
    );
}

// Regression for F32: object numbers are document-local. Two PDFs can use
// the same `/Widths 100 0 R` reference for different width arrays, so the
// cross-document cache key must fold the referenced array's content. ~keep
#[test]
fn font_identity_hash_folds_referenced_simple_widths_content() {
    let document_with_widths = |widths: Vec<Object>| {
        let doc = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
        doc.object_cache
            .lock_or_recover()
            .insert(ObjectRef::new(100, 0), Object::Array(widths));
        doc
    };
    let font = || {
        let mut dict = std::collections::HashMap::new();
        dict.insert("BaseFont".to_string(), Object::Name("Helvetica".to_string()));
        dict.insert("Subtype".to_string(), Object::Name("Type1".to_string()));
        dict.insert("Encoding".to_string(), Object::Name("WinAnsiEncoding".to_string()));
        dict.insert("FirstChar".to_string(), Object::Integer(65));
        dict.insert("LastChar".to_string(), Object::Integer(67));
        dict.insert("Widths".to_string(), Object::Reference(ObjectRef::new(100, 0)));
        Object::Dictionary(dict)
    };

    let narrow = document_with_widths(vec![Object::Integer(400), Object::Integer(400), Object::Integer(400)]);
    let wide = document_with_widths(vec![Object::Integer(700), Object::Integer(700), Object::Integer(700)]);

    assert_ne!(
        narrow.font_identity_hash_with_descendants(&font()),
        wide.font_identity_hash_with_descendants(&font()),
        "the same object id must not alias different referenced /Widths content across documents"
    );
}

// Regression for F32's Type0 variant: descendant `/W` arrays are commonly
// indirect, and their object numbers are no identity across documents. ~keep
#[test]
fn font_identity_hash_folds_referenced_descendant_widths_content() {
    let document_with_widths = |widths: Vec<Object>| {
        let doc = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
        let mut descendant = std::collections::HashMap::new();
        descendant.insert("Subtype".to_string(), Object::Name("CIDFontType2".to_string()));
        descendant.insert("W".to_string(), Object::Reference(ObjectRef::new(200, 0)));
        descendant.insert(
            "CIDSystemInfo".to_string(),
            Object::Dictionary(std::collections::HashMap::from([
                ("Registry".to_string(), Object::String(b"Adobe".to_vec())),
                ("Ordering".to_string(), Object::String(b"Identity".to_vec())),
                ("Supplement".to_string(), Object::Integer(0)),
            ])),
        );
        doc.object_cache
            .lock_or_recover()
            .insert(ObjectRef::new(6, 0), Object::Dictionary(descendant));
        doc.object_cache
            .lock_or_recover()
            .insert(ObjectRef::new(200, 0), Object::Array(widths));
        doc
    };
    let font = || {
        let mut dict = std::collections::HashMap::new();
        dict.insert("BaseFont".to_string(), Object::Name("CIDFont+F1".to_string()));
        dict.insert("Subtype".to_string(), Object::Name("Type0".to_string()));
        dict.insert("Encoding".to_string(), Object::Name("Identity-H".to_string()));
        dict.insert(
            "DescendantFonts".to_string(),
            Object::Array(vec![Object::Reference(ObjectRef::new(6, 0))]),
        );
        Object::Dictionary(dict)
    };

    let narrow = document_with_widths(vec![
        Object::Integer(1),
        Object::Array(vec![Object::Integer(400), Object::Integer(400)]),
    ]);
    let wide = document_with_widths(vec![
        Object::Integer(1),
        Object::Array(vec![Object::Integer(900), Object::Integer(900)]),
    ]);

    assert_ne!(
        narrow.font_identity_hash_with_descendants(&font()),
        wide.font_identity_hash_with_descendants(&font()),
        "the same object id must not alias different referenced descendant /W content across documents"
    );
}

#[test]
fn font_identity_hash_resolves_indirect_descendant_array_and_descriptor_content() {
    let document_with_descriptor = |flags: i64, program: &'static [u8]| {
        let doc = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
        doc.object_cache.lock_or_recover().insert(
            ObjectRef::new(100, 0),
            Object::Array(vec![Object::Reference(ObjectRef::new(6, 0))]),
        );
        doc.object_cache.lock_or_recover().insert(
            ObjectRef::new(6, 0),
            Object::Dictionary(std::collections::HashMap::from([
                ("Subtype".to_string(), Object::Name("CIDFontType2".to_string())),
                (
                    "CIDSystemInfo".to_string(),
                    Object::Dictionary(std::collections::HashMap::from([
                        ("Registry".to_string(), Object::String(b"Adobe".to_vec())),
                        ("Ordering".to_string(), Object::String(b"Identity".to_vec())),
                        ("Supplement".to_string(), Object::Integer(0)),
                    ])),
                ),
                ("FontDescriptor".to_string(), Object::Reference(ObjectRef::new(7, 0))),
            ])),
        );
        doc.object_cache.lock_or_recover().insert(
            ObjectRef::new(7, 0),
            Object::Dictionary(std::collections::HashMap::from([
                ("Flags".to_string(), Object::Integer(flags)),
                ("FontFile2".to_string(), Object::Reference(ObjectRef::new(8, 0))),
            ])),
        );
        doc.object_cache.lock_or_recover().insert(
            ObjectRef::new(8, 0),
            Object::Stream {
                dict: std::collections::HashMap::new(),
                data: bytes::Bytes::from_static(program),
            },
        );
        doc
    };
    let font = || {
        Object::Dictionary(std::collections::HashMap::from([
            ("BaseFont".to_string(), Object::Name("CIDFont+F1".to_string())),
            ("Subtype".to_string(), Object::Name("Type0".to_string())),
            ("Encoding".to_string(), Object::Name("Identity-H".to_string())),
            ("DescendantFonts".to_string(), Object::Reference(ObjectRef::new(100, 0))),
        ]))
    };

    let first = document_with_descriptor(4, b"first font program");
    let second = document_with_descriptor(32, b"second font program");

    assert_ne!(
        first.font_identity_hash_with_descendants(&font()),
        second.font_identity_hash_with_descendants(&font()),
        "indirect DescendantFonts, FontDescriptor metrics, and font-program content must be resolved"
    );
}

#[test]
fn cyclic_and_oversized_font_reference_graphs_are_not_shared() {
    let cyclic = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    cyclic
        .object_cache
        .lock_or_recover()
        .insert(ObjectRef::new(100, 0), Object::Reference(ObjectRef::new(100, 0)));
    let font = |reference| {
        Object::Dictionary(std::collections::HashMap::from([
            ("BaseFont".to_string(), Object::Name("Helvetica".to_string())),
            ("Subtype".to_string(), Object::Name("Type1".to_string())),
            ("Widths".to_string(), Object::Reference(reference)),
        ]))
    };

    assert!(
        !cyclic
            .font_identity_hash_details(&font(ObjectRef::new(100, 0)))
            .cacheable
    );

    let overdeep = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    for object_id in 100..=132 {
        overdeep.object_cache.lock_or_recover().insert(
            ObjectRef::new(object_id, 0),
            Object::Reference(ObjectRef::new(object_id + 1, 0)),
        );
    }
    overdeep
        .object_cache
        .lock_or_recover()
        .insert(ObjectRef::new(133, 0), Object::Array(vec![Object::Integer(400)]));

    assert!(
        !overdeep
            .font_identity_hash_details(&font(ObjectRef::new(100, 0)))
            .cacheable
    );

    let overwide = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let mut references = Vec::with_capacity(FONT_IDENTITY_MAX_RESOLVED_REFERENCES + 1);
    for offset in 0..=FONT_IDENTITY_MAX_RESOLVED_REFERENCES {
        let object_id = 1000 + u32::try_from(offset).expect("reference cap fits u32");
        let reference = ObjectRef::new(object_id, 0);
        overwide
            .object_cache
            .lock_or_recover()
            .insert(reference, Object::Integer(400));
        references.push(Object::Reference(reference));
    }
    overwide
        .object_cache
        .lock_or_recover()
        .insert(ObjectRef::new(999, 0), Object::Array(references));

    assert!(
        !overwide
            .font_identity_hash_details(&font(ObjectRef::new(999, 0)))
            .cacheable
    );
}

#[test]
fn encrypted_font_identity_is_not_shared_from_raw_ciphertext() {
    let mut document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    document.trailer = Object::Dictionary(std::collections::HashMap::from([(
        "Encrypt".to_string(),
        Object::Reference(ObjectRef::new(99, 0)),
    )]));
    let font = Object::Dictionary(std::collections::HashMap::from([
        ("BaseFont".to_string(), Object::Name("Helvetica".to_string())),
        ("Subtype".to_string(), Object::Name("Type1".to_string())),
    ]));

    assert!(!document.font_identity_hash_details(&font).cacheable);
}

#[test]
fn shared_font_stream_is_hashed_once_per_document() {
    let document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let stream_data = bytes::Bytes::from(vec![b'F'; 4096]);
    document.object_cache.lock_or_recover().insert(
        ObjectRef::new(100, 0),
        Object::Stream {
            dict: std::collections::HashMap::new(),
            data: stream_data.clone(),
        },
    );
    let font = |base_font: &str| {
        Object::Dictionary(std::collections::HashMap::from([
            ("BaseFont".to_string(), Object::Name(base_font.to_string())),
            ("Subtype".to_string(), Object::Name("Type0".to_string())),
            ("ToUnicode".to_string(), Object::Reference(ObjectRef::new(100, 0))),
        ]))
    };

    assert!(document.font_identity_hash_details(&font("FontA")).cacheable);
    assert!(document.font_identity_hash_details(&font("FontB")).cacheable);
    assert_eq!(document.font_identity_hashed_bytes(), stream_data.len());
}

#[test]
fn font_hash_byte_budget_disables_shared_identity_caches() {
    let document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    document.object_cache.lock_or_recover().insert(
        ObjectRef::new(100, 0),
        Object::Stream {
            dict: std::collections::HashMap::new(),
            data: bytes::Bytes::from(vec![b'F'; FONT_IDENTITY_MAX_HASHED_BYTES + 1]),
        },
    );
    let font = Object::Dictionary(std::collections::HashMap::from([
        ("BaseFont".to_string(), Object::Name("FontA".to_string())),
        ("Subtype".to_string(), Object::Name("Type0".to_string())),
        ("ToUnicode".to_string(), Object::Reference(ObjectRef::new(100, 0))),
    ]));

    assert!(!document.font_identity_hash_details(&font).cacheable);
    assert!(!document.font_identity_shared_cache_enabled());
    let cheap_font = Object::Dictionary(std::collections::HashMap::from([(
        "BaseFont".to_string(),
        Object::Name("Helvetica".to_string()),
    )]));
    assert!(!document.font_identity_hash_details(&cheap_font).cacheable);
}

#[test]
fn memoized_font_references_still_obey_per_root_reference_limit() {
    let document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let mut references = Vec::with_capacity(FONT_IDENTITY_MAX_RESOLVED_REFERENCES + 1);
    for offset in 0..=FONT_IDENTITY_MAX_RESOLVED_REFERENCES {
        let object_id = 1000 + u32::try_from(offset).expect("reference cap fits u32");
        let reference = ObjectRef::new(object_id, 0);
        document
            .object_cache
            .lock_or_recover()
            .insert(reference, Object::Integer(400));
        let font = Object::Dictionary(std::collections::HashMap::from([(
            "Widths".to_string(),
            Object::Reference(reference),
        )]));
        assert!(document.font_identity_hash_details(&font).cacheable);
        references.push(Object::Reference(reference));
    }
    document
        .object_cache
        .lock_or_recover()
        .insert(ObjectRef::new(999, 0), Object::Array(references));
    let font = Object::Dictionary(std::collections::HashMap::from([(
        "Widths".to_string(),
        Object::Reference(ObjectRef::new(999, 0)),
    )]));

    assert!(!document.font_identity_hash_details(&font).cacheable);
}

#[test]
fn memoized_font_reference_still_obeys_remaining_depth_budget() {
    let document = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let shared_reference = ObjectRef::new(100, 0);
    let mut shared_object = Object::Integer(400);
    for _ in 0..12 {
        shared_object = Object::Dictionary(std::collections::HashMap::from([(
            "FontDescriptor".to_string(),
            shared_object,
        )]));
    }
    document
        .object_cache
        .lock_or_recover()
        .insert(shared_reference, shared_object);
    let shallow_font = Object::Dictionary(std::collections::HashMap::from([(
        "Widths".to_string(),
        Object::Reference(shared_reference),
    )]));
    assert!(document.font_identity_hash_details(&shallow_font).cacheable);

    let first_outer_reference = ObjectRef::new(200, 0);
    for object_id in 200..210 {
        let next_reference = if object_id == 209 {
            shared_reference
        } else {
            ObjectRef::new(object_id + 1, 0)
        };
        document.object_cache.lock_or_recover().insert(
            ObjectRef::new(object_id, 0),
            Object::Dictionary(std::collections::HashMap::from([(
                "FontDescriptor".to_string(),
                Object::Reference(next_reference),
            )])),
        );
    }
    let deep_font = Object::Dictionary(std::collections::HashMap::from([(
        "Widths".to_string(),
        Object::Reference(first_outer_reference),
    )]));

    assert!(!document.font_identity_hash_details(&deep_font).cacheable);
}

#[test]
fn test_load_fonts_public_empty_resources() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let mut ext = crate::extractors::TextExtractor::new();
    assert!(
        doc.load_fonts_public(&Object::Dictionary(std::collections::HashMap::new()), &mut ext)
            .is_ok()
    );
}

#[test]
fn test_load_fonts_public_resources_not_dict() {
    let pdf = build_minimal_pdf(b"");
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    let mut ext = crate::extractors::TextExtractor::new();
    assert!(doc.load_fonts_public(&Object::Integer(42), &mut ext).is_ok());
}

#[test]
fn test_get_page_rotation_still_folds_absent_and_malformed_to_zero() {
    // Pins get_page_rotation's existing fold-to-0 contract, which must not
    // change: both absent and out-of-spec /Rotate values still read as 0
    // through the original accessor. ~keep
    let absent = PdfDocument::from_bytes(build_pdf_with_rotate_token(None, false)).unwrap();
    assert_eq!(absent.get_page_rotation(0).unwrap(), 0);
    let malformed = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("135"), false)).unwrap();
    assert_eq!(malformed.get_page_rotation(0).unwrap(), 0);
}

/// #1746: a `/Font` dictionary shared across pages must donate TrueType
/// cmaps between its own fonts only ONCE, not on every page that touches it.
/// Every simulated page must still see the donated cmap (the cache must not
/// merely go quiet while serving stale, undonated fonts), but the donation
/// work itself — `donate_truetype_cmaps_within_set` — must not re-run past
/// the first page.
#[test]
fn donated_truetype_cmap_is_reused_not_recomputed_per_page() {
    let doc = PdfDocument::from_bytes(build_minimal_pdf(b"")).unwrap();
    let (_font_dict_ref, resources) = donor_and_recipient_font_resources(&doc);

    const PAGES: usize = 5;
    for page in 0..PAGES {
        let mut extractor = crate::extractors::TextExtractor::new();
        doc.load_fonts(&resources, &mut extractor).expect("load_fonts");

        let recipient = extractor
            .get_font_set()
            .into_iter()
            .find(|(name, _)| name == "F2")
            .map(|(_, font)| font)
            .unwrap_or_else(|| panic!("page {page}: recipient font F2 missing from extractor"));
        assert!(
            recipient.truetype_cmap().is_some(),
            "page {page}: recipient must carry the donor's cmap"
        );
    }

    assert_eq!(
        doc.donation_call_count(),
        1,
        "donation must run once for a font set reused across {PAGES} pages, not once per page"
    );
    assert_eq!(
        doc.donation_apply_count(),
        1,
        "share_truetype_cmaps must apply the donation once (page 0) and find the recipient \
         already carrying a cmap on every later page, not redo the Arc::make_mut mutation \
         {PAGES} times"
    );
}

/// A one-page PDF with one line of Helvetica text whose `/Encoding` is a `/Differences`
/// dictionary, written inline in the font or as its own indirect object.
fn build_differences_encoding_pdf(indirect: bool) -> Vec<u8> {
    let encoding_dict = "<< /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [65 /A] >>";
    let encoding = if indirect { "6 0 R" } else { encoding_dict };
    let content = b"BT /F1 12 Tf 72 700 Td (A plain line of text.) Tj ET";
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets: Vec<usize> = Vec::new();

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
          /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>\nendobj\n",
    );
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!("5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding {encoding} >>\nendobj\n")
            .as_bytes(),
    );
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("6 0 obj\n{encoding_dict}\nendobj\n").as_bytes());

    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn stream_expected_warnings(events: &[CapturedEvent]) -> usize {
    events
        .iter()
        .filter(|event| {
            event
                .fields
                .get("message")
                .is_some_and(|message| message.contains("dictionary used where stream expected"))
        })
        .count()
}

/// GH#1795: a `/Differences` encoding dictionary is a valid `/Encoding`, not a stream
/// written as a dictionary, so loading the font logs nothing and the text is intact.
#[test]
fn should_load_a_differences_encoding_dictionary_without_a_stream_warning() {
    for indirect in [false, true] {
        let doc = PdfDocument::from_bytes(build_differences_encoding_pdf(indirect)).unwrap();
        let (text, events) = capture_events(|| doc.extract_text(0));
        let text = text.unwrap_or_else(|e| panic!("indirect={indirect}: extract text: {e}"));

        assert!(
            text.contains("A plain line of text."),
            "indirect={indirect}: text must be intact, got {text:?}"
        );
        assert_eq!(
            stream_expected_warnings(&events),
            0,
            "indirect={indirect}: a /Differences dictionary must not log a stream warning"
        );
    }
}

/// The control for the test above: the capture does see this warning when a plain
/// dictionary really is decoded as a stream, so its zero is not vacuous.
#[test]
fn should_capture_the_stream_warning_when_a_dictionary_is_decoded_as_a_stream() {
    let dictionary = Object::Dictionary(std::collections::HashMap::new());
    let (decoded, events) = capture_events(|| dictionary.decode_stream_data());

    assert_eq!(
        decoded.expect("a dictionary decodes as an empty stream"),
        Vec::<u8>::new()
    );
    assert_eq!(stream_expected_warnings(&events), 1);
}
