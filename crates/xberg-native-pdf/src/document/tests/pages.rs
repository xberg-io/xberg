use super::super::*;
use super::pdf_fixtures::*;

#[test]
fn test_page_inherits_mediabox() {
    let mut pdf = b"%PDF-1.4\n".to_vec();

    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 400 600] >>\nendobj\n");

    let off3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R >>\nendobj\n");

    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 4\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());

    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(doc.page_count().unwrap(), 1);
    let page = doc.get_page(0).unwrap();
    let page_dict = page.as_dict().unwrap();
    assert!(page_dict.contains_key("MediaBox"));
}

#[test]
fn test_page_count_rescued_when_count_is_zero_but_pages_exist() {
    // The motivating broken-/Count case: a `/Pages` node whose `/Count`
    // says 0 while its `/Kids` hold real pages. An ObjStm-packed `/Pages` tree
    // that the standard reader cannot resolve reaches `page_count` the same way
    // - `primary == Ok(0)`. Here the standard reader trusts the literal `/Count`
    // and returns 0, but `get_page` still walks `/Pages` -> `/Kids` and reaches
    // every page, so the rescue enumerates them.
    //
    // WITHOUT the rescue block this returns 0 (verified: reverting the
    // document.rs hunk makes this assertion fail with `0 != 3`); WITH it, 3. ~keep
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 0 >>\nendobj\n");
    let mut offs = vec![off1, off2];
    for n in 3..=5u32 {
        offs.push(pdf.len());
        pdf.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
                n
            )
            .as_bytes(),
        );
    }
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offs {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    // The standard /Count reader really does report 0 here, so the count of 3
    // comes entirely from the enumerator rescue (not from the primary path). ~keep
    assert_eq!(
        doc.get_page_count_standard().unwrap(),
        0,
        "fixture must drive the standard reader to 0"
    );
    assert_eq!(doc.page_count().unwrap(), 3, "rescue must enumerate the real pages");
}

#[test]
fn test_deeply_nested_page_tree() {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let off1 = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let off2 = pdf.len();
    pdf.extend_from_slice(
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 595 842] /Resources << >> >>\nendobj\n",
    );
    let off3 = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Pages /Kids [4 0 R] /Count 1 /Parent 2 0 R >>\nendobj\n");
    let off4 = pdf.len();
    pdf.extend_from_slice(b"4 0 obj\n<< /Type /Page /Parent 3 0 R >>\nendobj\n");
    let xref_off = pdf.len();
    pdf.extend_from_slice(b"xref\n0 5\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off1).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off2).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off3).as_bytes());
    pdf.extend_from_slice(format!("{:010} 00000 n \n", off4).as_bytes());
    pdf.extend_from_slice(format!("trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", xref_off).as_bytes());
    let doc = PdfDocument::from_bytes(pdf).unwrap();
    assert_eq!(doc.page_count().unwrap(), 1);
    let page = doc.get_page(0).unwrap();
    assert!(page.as_dict().unwrap().contains_key("MediaBox"));
}

#[test]
fn test_page_text_serializable() {
    let page_text = crate::layout::PageText {
        spans: Vec::new(),
        chars: Vec::new(),
        page_width: 612.0,
        page_height: 792.0,
    };
    let json = serde_json::to_string(&page_text).unwrap();
    // Without the `wasm` feature, field names are snake_case ~keep
    assert!(json.contains("page_width"));
    assert!(json.contains("page_height"));
}

#[test]
fn test_is_cm_or_symbol_font() {
    assert!(PdfDocument::is_cm_or_symbol_font("ABCDEF+CMSY10"));
    assert!(PdfDocument::is_cm_or_symbol_font("CMR12"));
    assert!(PdfDocument::is_cm_or_symbol_font("Symbol"));
    assert!(!PdfDocument::is_cm_or_symbol_font("ABCDEF+Helvetica"));
    assert!(!PdfDocument::is_cm_or_symbol_font("TimesNewRoman"));
}

#[test]
fn test_get_page_rotation_status_135_is_malformed_not_folded_to_valid_zero() {
    // GH#1654: `get_page_rotation` folds this to plain `0`, indistinguishable
    // from a genuine `/Rotate 0` or an absent entry. The new accessor must
    // distinguish it as Malformed instead. ~keep
    let doc = PdfDocument::from_bytes(build_pdf_with_rotate_token(Some("135"), false)).unwrap();
    assert_eq!(doc.get_page_rotation_status(0).unwrap(), PageRotation::Malformed);
}

/// GH#1755. The page tree is walked recursively, so a chain of `/Pages` nodes is a
/// chain of stack frames. Unfixed, a chain overflows a 2 MiB thread stack — an abort,
/// not a catchable panic, so one uploaded PDF took the whole consuming process down.
/// Measured on a 2 MiB thread before the cap landed: abort at ~370 levels in a debug
/// build and at ~3,000 in a release build. The red step therefore cannot live here —
/// a genuine overflow would kill the test process rather than fail one test — so what
/// is asserted is the post-cap contract: every page-tree entry point RETURNS.
#[test]
fn should_return_rather_than_abort_when_page_tree_chain_exceeds_the_depth_cap() {
    let levels = MAX_PAGE_TREE_DEPTH as usize + 4;
    let doc = PdfDocument::from_bytes(build_page_tree_chain_pdf(levels)).unwrap();

    // Degraded, not lost: the tree walk stops at the cap, and the object scan
    // still recovers the single leaf page. ~keep
    assert_eq!(doc.page_count().unwrap(), 1, "page_count must degrade, not abort");

    let page = doc.get_page(0).expect("get_page must degrade to the scanning fallback");
    assert!(
        !page.as_dict().unwrap().contains_key("MediaBox"),
        "the page was recovered by scanning, so the root's inheritable /MediaBox is NOT merged \
         - this is what distinguishes the capped walk from the control below"
    );

    let page_ref_error = doc.get_page_ref(0).expect_err("get_page_ref has no scanning fallback");
    assert!(
        matches!(page_ref_error, Error::InvalidPdf(_)),
        "expected the tree walk to run out of kids, got {:?}",
        page_ref_error
    );

    // GH#1755: collect_page_refs (objects.rs) used to be the one page-tree walker that
    // propagated RecursionLimitExceeded with `?` instead of skipping the offending
    // branch, so all_page_refs() surfaced `Err` here where every other walker above
    // degrades. It now matches them: the over-cap branch is skipped like any other bad
    // branch. This fixture is a single linear chain with no fork above the cap, so
    // "skip the one branch that fails" and "collect everything else" coincide at an
    // empty result — not because the walker gave up, but because there was nothing
    // else in the tree to find. ~keep
    assert_eq!(
        doc.all_page_refs().unwrap(),
        Vec::new(),
        "all_page_refs must degrade like the other page-tree walkers, not abort"
    );
}

/// The control for the test above: one level shallower than the cap resolves
/// through the page tree itself, inherited attributes and all. Without it, a cap
/// of zero would pass the over-cap test just as well.
#[test]
fn should_resolve_the_page_normally_when_the_chain_stays_under_the_depth_cap() {
    let levels = MAX_PAGE_TREE_DEPTH as usize - 1;
    let doc = PdfDocument::from_bytes(build_page_tree_chain_pdf(levels)).unwrap();

    assert_eq!(doc.page_count().unwrap(), 1);

    let page = doc.get_page(0).unwrap();
    let page_dict = page.as_dict().unwrap();
    assert!(
        page_dict.contains_key("MediaBox"),
        "the tree walk reached the leaf, so the root's /MediaBox must be inherited"
    );
    assert!(page_dict.contains_key("Resources"), "/Resources is inheritable too");

    assert_eq!(doc.get_page_ref(0).unwrap().id, (2 + levels) as u32);
    assert_eq!(doc.all_page_refs().unwrap().len(), 1);
}

fn build_page_inheritance_pdf(intermediate_attrs: &str) -> Vec<u8> {
    let mut kids = String::new();
    let mut objects = Vec::new();
    objects.push((3, b"<< /Type /Pages /Parent 2 0 R /Count 65 /Kids [".to_vec()));
    for id in 4..=68 {
        kids.push_str(&format!("{id} 0 R "));
        let attrs = match id {
            4 => "/Resources 93 0 R /MediaBox 94 0 R /CropBox 95 0 R /Rotate 96 0 R",
            5 => "/Resources 80 0 R /MediaBox 81 0 R /CropBox 82 0 R /Rotate 83 0 R",
            _ => "",
        };
        objects.push((id, format!("<< /Type /Page /Parent 3 0 R {attrs} >>").into_bytes()));
    }
    objects[0]
        .1
        .extend_from_slice(format!("{kids}] {intermediate_attrs} >>").as_bytes());
    objects.push((69, b"<< /Type /Page /Parent 2 0 R >>".to_vec()));
    objects.extend([
        (80, b"<< /Marker 8 >>".to_vec()),
        (81, b"[0 0 300 300]".to_vec()),
        (82, b"[0 0 280 280]".to_vec()),
        (83, b"270".to_vec()),
    ]);
    let refs: Vec<_> = objects.iter().map(|(id, body)| (*id, body.as_slice())).collect();
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 69 0 R] /Count 66 /Resources << /Marker 1 >> /MediaBox [0 0 100 100] /CropBox [0 0 90 90] /Rotate 90 >>",
        &refs,
    )
}

fn assert_inherited_values(doc: &PdfDocument, index: usize, media: f32, crop: i64, rotation: i32, marker: i64) {
    let page = doc.get_page(index).unwrap();
    let dict = page.as_dict().unwrap();
    assert_eq!(
        doc.get_page_media_box(index)
            .unwrap_or_else(|error| panic!("page {index}: {error:?}; dictionary: {dict:?}")),
        (0.0, 0.0, media, media)
    );
    let crop_object = doc.resolve_obj_ref(dict.get("CropBox").unwrap());
    let crop_array = crop_object.as_array().unwrap();
    assert_eq!(crop_array[2].as_integer(), Some(crop));
    assert_eq!(doc.get_page_rotation(index).unwrap(), rotation);
    let resources = doc.resolve_obj_ref(dict.get("Resources").unwrap());
    assert_eq!(
        resources.as_dict().unwrap().get("Marker").unwrap().as_integer(),
        Some(marker)
    );
}

#[test]
fn should_fall_back_from_dangling_inheritable_references_in_lazy_and_bulk_page_walks() {
    let pdf = build_page_inheritance_pdf("/Resources 97 0 R /MediaBox [0 0 200 200] /CropBox 98 0 R /Rotate 99 0 R");
    let lazy = PdfDocument::from_bytes(pdf.clone()).unwrap();
    assert!(
        lazy.get_page_ref(0).is_ok(),
        "tree must be traversable: {:?}",
        lazy.get_page_ref(0)
    );
    assert_eq!(
        lazy.load_object(ObjectRef { id: 94, generation: 0 }).unwrap(),
        Object::Null
    );
    assert_inherited_values(&lazy, 0, 200.0, 90, 90, 1);
    assert_inherited_values(&lazy, 1, 300.0, 280, 270, 8);
    assert_inherited_values(&lazy, 65, 100.0, 90, 90, 1);

    let bulk = PdfDocument::from_bytes(pdf).unwrap();
    for index in 0..=64 {
        bulk.get_page(index).unwrap();
    }
    assert_eq!(
        bulk.page_cache.lock_or_recover().len(),
        66,
        "bulk walk must cache every page"
    );
    assert_inherited_values(&bulk, 0, 200.0, 90, 90, 1);
    assert_inherited_values(&bulk, 1, 300.0, 280, 270, 8);
    assert_inherited_values(&bulk, 65, 100.0, 90, 90, 1);
}

#[test]
fn should_prefer_nearest_valid_ancestor_and_preserve_valid_indirect_leaf_values() {
    let pdf = build_page_inheritance_pdf(
        "/Resources << /Marker 2 >> /MediaBox [0 0 200 200] /CropBox [0 0 180 180] /Rotate 180",
    );
    for bulk_walk in [false, true] {
        let doc = PdfDocument::from_bytes(pdf.clone()).unwrap();
        if bulk_walk {
            for index in 0..=64 {
                doc.get_page(index).unwrap();
            }
            assert_eq!(
                doc.page_cache.lock_or_recover().len(),
                66,
                "bulk walk must cache every page"
            );
        }
        assert_inherited_values(&doc, 0, 200.0, 180, 180, 2);
        assert_inherited_values(&doc, 1, 300.0, 280, 270, 8);
        assert_inherited_values(&doc, 65, 100.0, 90, 90, 1);

        let leaf = doc.get_page(1).unwrap();
        let leaf = leaf.as_dict().unwrap();
        for (attribute, object_id) in [("Resources", 80), ("MediaBox", 81), ("CropBox", 82), ("Rotate", 83)] {
            assert_eq!(leaf.get(attribute).unwrap().as_reference().unwrap().id, object_id);
        }
    }
}
