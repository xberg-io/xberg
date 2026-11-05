//! Routing-only observations; mock text never qualifies real English OCR. ~keep
#[path = "xobject_budget_parity_tests/composition_adoption.rs"]
mod composition_adoption;
#[path = "xobject_budget_parity_tests/preferred_stage.rs"]
mod preferred_stage;
#[path = "xobject_budget_parity_tests/support.rs"]
mod support;
use super::{recover_pipeline_document_from_image_xobjects, recover_pipeline_page_from_image_xobjects};
use support::{BACKEND, Reply, TEXT, config, fixture, observer};

#[tokio::test]
#[serial_test::serial]
async fn raw_collectors_preserve_original_dct_bytes_and_paint_order() {
    let f = fixture(2, false);
    let (_guard, records) = observer(Reply::Text);
    let (cfg, ocr) = config(16_384, 16_384);
    for detached in [false, true] {
        records.lock().unwrap().clear();
        let out = if detached {
            super::raw_collection_test_support::detached(&f.doc, &cfg, &ocr, BACKEND).await
        } else {
            super::raw_collection_test_support::whole(&f.doc, &cfg, &ocr, BACKEND).await
        }
        .expect("both raw input collectors return routing payload");
        assert!(!out.text.is_empty());
        let calls = records.lock().unwrap();
        assert_eq!(calls.len(), 2);
        for (i, call) in calls.iter().enumerate() {
            assert_eq!(call.bytes, f.streams[i]);
            assert_eq!(call.config.language, ocr.language);
            assert_eq!(call.config.backend_options, ocr.backend_options);
            assert_eq!(out.images[i].image_index, i as u32);
            assert_eq!(out.images[i].source_path, Some(format!("xobject:page1:{i}")));
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn whole_document_retry_forwards_the_smaller_outer_cap_like_detached_retry() {
    let f = support::complete_fixture();
    let (_guard, records) = observer(Reply::Text);
    let outer = f.streams[0].len() + 1024;
    let (cfg, ocr) = config(outer, outer * 2);
    recover_pipeline_page_from_image_xobjects(&f.doc, 0, &cfg, &ocr, BACKEND)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        records.lock().unwrap()[0]
            .config
            .security_limits
            .as_ref()
            .unwrap()
            .max_content_size,
        outer
    );
    records.lock().unwrap().clear();
    recover_pipeline_document_from_image_xobjects(Some(&f.bytes), &cfg, &ocr, BACKEND)
        .await
        .unwrap()
        .unwrap();
    let calls = records.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].config.security_limits.as_ref().unwrap().max_content_size,
        outer
    );
}

#[tokio::test]
#[serial_test::serial]
async fn whole_document_retry_preserves_the_smaller_explicit_ocr_cap() {
    let f = support::complete_fixture();
    let (_guard, records) = observer(Reply::Text);
    let inner = f.streams[0].len() + 1024;
    let (cfg, ocr) = config(inner * 2, inner);
    let out = recover_pipeline_document_from_image_xobjects(Some(&f.bytes), &cfg, &ocr, BACKEND)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(out.text, TEXT);
    let calls = records.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].config.security_limits.as_ref().unwrap().max_content_size,
        inner
    );
}

#[tokio::test]
#[serial_test::serial]
async fn over_cap_original_dct_never_reaches_either_retry_backend() {
    let f = support::complete_fixture();
    let (_guard, records) = observer(Reply::Text);
    let cap = f.streams[0].len() - 1;
    assert!(cap > TEXT.len());
    let (cfg, ocr) = config(cap, cap * 2);
    let detached = recover_pipeline_page_from_image_xobjects(&f.doc, 0, &cfg, &ocr, BACKEND)
        .await
        .unwrap();
    assert!(detached.is_none());
    assert!(records.lock().unwrap().is_empty());
    let whole = recover_pipeline_document_from_image_xobjects(Some(&f.bytes), &cfg, &ocr, BACKEND)
        .await
        .unwrap();
    assert!(
        whole.is_none(),
        "whole-document retry must apply the same stored-stream admission"
    );
    assert!(records.lock().unwrap().is_empty());
}

#[tokio::test]
#[serial_test::serial]
async fn raw_collectors_preserve_only_the_admitted_stored_stream_prefix() {
    let f = fixture(2, false);
    let (_guard, records) = observer(Reply::Text);
    let cap = f.streams.iter().map(Vec::len).sum::<usize>() - 1;
    assert!(cap > f.streams[0].len());
    let (cfg, ocr) = config(cap, cap);
    for detached in [true, false] {
        records.lock().unwrap().clear();
        let out = if detached {
            super::raw_collection_test_support::detached(&f.doc, &cfg, &ocr, BACKEND).await
        } else {
            super::raw_collection_test_support::whole(&f.doc, &cfg, &ocr, BACKEND).await
        }
        .expect("first original DCT remains admitted as raw input");
        let calls = records.lock().unwrap();
        assert_eq!(
            calls.len(),
            1,
            "retry may copy only the admitted original stream prefix"
        );
        assert_eq!(calls[0].bytes, f.streams[0]);
        assert_eq!(out.images.len(), 1);
        assert_eq!(out.images[0].image_index, 0);
        assert_eq!(out.images[0].source_path.as_deref(), Some("xobject:page1:0"));
    }
}

#[tokio::test]
#[serial_test::serial]
async fn empty_or_failed_retry_does_not_publish_successful_content() {
    let f = support::complete_fixture();
    let (cfg, ocr) = config(16_384, 16_384);
    for reply in [Reply::Empty, Reply::Fail] {
        let (_guard, records) = observer(reply);
        let whole = recover_pipeline_document_from_image_xobjects(Some(&f.bytes), &cfg, &ocr, BACKEND)
            .await
            .unwrap();
        let detached = recover_pipeline_page_from_image_xobjects(&f.doc, 0, &cfg, &ocr, BACKEND)
            .await
            .unwrap();
        assert!(whole.is_none() && detached.is_none());
        assert_eq!(records.lock().unwrap().len(), 2);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn pre_cancelled_whole_document_retry_does_not_call_a_backend() {
    let f = support::complete_fixture();
    let (_guard, records) = observer(Reply::Text);
    let (mut cfg, ocr) = config(16_384, 16_384);
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    cfg.cancel_token = Some(token);
    assert!(matches!(
        recover_pipeline_page_from_image_xobjects(&f.doc, 0, &cfg, &ocr, BACKEND).await,
        Err(crate::XbergError::Cancelled)
    ));
    assert!(records.lock().unwrap().is_empty());
    assert!(matches!(
        recover_pipeline_document_from_image_xobjects(Some(&f.bytes), &cfg, &ocr, BACKEND).await,
        Err(crate::XbergError::Cancelled)
    ));
    assert!(records.lock().unwrap().is_empty());
}

#[tokio::test]
#[serial_test::serial]
async fn legacy_small_raw_image_recovery_is_not_lost_by_dct_budget_parity() {
    let f = fixture(1, true);
    let (_guard, records) = observer(Reply::Text);
    let (cfg, ocr) = config(16_384, 16_384);
    let out = recover_pipeline_document_from_image_xobjects(Some(&f.bytes), &cfg, &ocr, BACKEND)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(out.text, TEXT);
    let calls = records.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    let dimensions = image::ImageReader::new(std::io::Cursor::new(&calls[0].bytes))
        .with_guessed_format()
        .unwrap()
        .into_dimensions()
        .unwrap();
    assert_eq!(dimensions, (32, 24));
}

#[tokio::test]
#[serial_test::serial]
async fn raw_outer_filtered_dct_transport_does_not_claim_a_complete_projection() {
    let f = support::filtered_fixture();
    let (_guard, records) = observer(Reply::Text);
    let (cfg, ocr) = config(16_384, 16_384);
    assert!(
        recover_pipeline_document_from_image_xobjects(Some(&f.bytes), &cfg, &ocr, BACKEND)
            .await
            .unwrap()
            .is_none()
    );
    assert!(records.lock().unwrap().is_empty());
    let out = super::raw_collection_test_support::whole(&f.doc, &cfg, &ocr, BACKEND)
        .await
        .expect("existing outer-filtered JPEG raw transport remains recoverable");
    assert_eq!(out.text, TEXT);
    let calls = records.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let expected = image::load_from_memory(&f.streams[0]).unwrap().into_rgb8();
    let recovered = image::load_from_memory(&calls[0].bytes)
        .expect("legacy retry supplies a complete independently decodable image")
        .into_rgb8();
    assert_eq!(expected.dimensions(), (32, 24));
    assert_eq!(recovered.dimensions(), (32, 24));
    assert_eq!(
        recovered.as_raw(),
        expected.as_raw(),
        "outer-filter recovery preserves actual source pixels"
    );
    let format = match image::guess_format(&calls[0].bytes).unwrap() {
        image::ImageFormat::Jpeg => "jpeg",
        image::ImageFormat::Png => "png",
        other => panic!("unexpected legacy recovery format: {other:?}"),
    };
    assert_eq!(out.images.len(), 1);
    assert_eq!(out.images[0].format.as_ref(), format);
}

async fn forwarded_table_cell_limits(
    f: &support::Fixture,
    cfg: &crate::core::config::ExtractionConfig,
    ocr: &crate::core::config::OcrConfig,
    records: &support::Records,
) -> [usize; 2] {
    let mut forwarded = [0; 2];
    for (slot, detached) in [false, true].into_iter().enumerate() {
        records.lock().unwrap().clear();
        if detached {
            recover_pipeline_page_from_image_xobjects(&f.doc, 0, cfg, ocr, BACKEND).await
        } else {
            recover_pipeline_document_from_image_xobjects(Some(&f.bytes), cfg, ocr, BACKEND).await
        }
        .unwrap()
        .expect("both retry routes return routing payload");
        let calls = records.lock().unwrap();
        forwarded[slot] = calls[0].config.security_limits.as_ref().unwrap().max_table_cells;
    }
    forwarded
}

#[tokio::test]
#[serial_test::serial]
async fn both_retry_routes_forward_the_same_non_size_limits() {
    let f = support::complete_fixture();
    let (_guard, records) = observer(Reply::Text);
    let (mut cfg, mut ocr) = config(16_384, 16_384);
    cfg.security_limits.as_mut().unwrap().max_table_cells = 9;
    ocr.security_limits.as_mut().unwrap().max_table_cells = 7;
    assert_eq!(
        forwarded_table_cell_limits(&f, &cfg, &ocr, &records).await,
        [7, 7],
        "the OCR config's own limits take precedence on both routes"
    );
    ocr.security_limits = None;
    assert_eq!(
        forwarded_table_cell_limits(&f, &cfg, &ocr, &records).await,
        [9, 9],
        "without OCR limits, both routes start from the extraction limits"
    );
}

#[path = "xobject_budget_parity_tests/bounded_transport.rs"]
mod bounded_transport;
