/// Regression tests: vtable Bytes params carry companion length
///
/// The alef vtable generator previously emitted only `*const u8` for `&[u8]`
/// trait-method parameters without a companion `{name}_len: usize`. Binary
/// payloads contain embedded NUL bytes; read-until-NUL semantics silently
/// truncated every real image or document buffer at the first `0x00`.
///
/// Fix shipped in alef ≥ v0.19.21 and is present in the generated FFI shim.
/// These tests construct a vtable bridge directly, pass a buffer with an
/// embedded NUL at a known offset, and assert the full buffer is received.
///
/// Per-test state is passed via `user_data` — no global statics — so tests
/// are independent and can run in parallel without interfering.
use kreuzberg_ffi::{
    KreuzbergDocumentExtractorBridge, KreuzbergDocumentExtractorVTable, KreuzbergOcrBackendBridge,
    KreuzbergOcrBackendVTable,
};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

// ── Per-test callback state ───────────────────────────────────────────────

struct CallbackState {
    received_len: AtomicUsize,
    received_last_byte: AtomicU8,
}

impl CallbackState {
    fn new() -> Self {
        Self {
            received_len: AtomicUsize::new(0),
            received_last_byte: AtomicU8::new(0),
        }
    }
}

// ── C callback stubs ─────────────────────────────────────────────────────

unsafe extern "C" fn ocr_process_image(
    user_data: *const std::ffi::c_void,
    image_bytes: *const u8,
    image_bytes_len: usize,
    _config: *const std::ffi::c_char,
    out_result: *mut *mut std::ffi::c_char,
    out_error: *mut *mut std::ffi::c_char,
) -> i32 {
    // SAFETY: user_data points to a CallbackState that the calling test keeps alive.
    let state = unsafe { &*(user_data as *const CallbackState) };
    state.received_len.store(image_bytes_len, Ordering::SeqCst);
    if image_bytes_len > 0 {
        // SAFETY: caller guarantees image_bytes[0..image_bytes_len] is valid.
        let last = unsafe { *image_bytes.add(image_bytes_len - 1) };
        state.received_last_byte.store(last, Ordering::SeqCst);
    }
    unsafe { *out_result = std::ptr::null_mut() };
    let msg = std::ffi::CString::new("stub").unwrap();
    // SAFETY: caller owns out_error and will free it via kreuzberg_free_string.
    unsafe { *out_error = msg.into_raw() };
    1
}

unsafe extern "C" fn extractor_extract_bytes(
    user_data: *const std::ffi::c_void,
    content: *const u8,
    content_len: usize,
    _mime_type: *const std::ffi::c_char,
    _config: *const std::ffi::c_char,
    out_result: *mut *mut std::ffi::c_char,
    out_error: *mut *mut std::ffi::c_char,
) -> i32 {
    // SAFETY: user_data points to a CallbackState that the calling test keeps alive.
    let state = unsafe { &*(user_data as *const CallbackState) };
    state.received_len.store(content_len, Ordering::SeqCst);
    if content_len > 0 {
        // SAFETY: caller guarantees content[0..content_len] is valid.
        let last = unsafe { *content.add(content_len - 1) };
        state.received_last_byte.store(last, Ordering::SeqCst);
    }
    unsafe { *out_result = std::ptr::null_mut() };
    let msg = std::ffi::CString::new("stub").unwrap();
    unsafe { *out_error = msg.into_raw() };
    1
}

// ── Tests ─────────────────────────────────────────────────────────────────

/// OcrBackend.process_image must pass the full buffer length even when
/// the payload contains embedded NUL bytes.
#[tokio::test]
async fn ocr_backend_vtable_process_image_passes_full_length_with_embedded_nuls() {
    // 8-byte buffer; NUL at index 3. strlen-style reads would stop at 3.
    let image_bytes: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];

    let state = Box::new(CallbackState::new());
    let state_ptr = state.as_ref() as *const CallbackState as *const std::ffi::c_void;

    let vtable = KreuzbergOcrBackendVTable {
        process_image: Some(ocr_process_image),
        process_image_file: None,
        name_fn: None,
        version_fn: None,
        initialize_fn: None,
        shutdown_fn: None,
        supports_language: None,
        backend_type: None,
        supported_languages: None,
        supports_table_detection: None,
        supports_document_processing: None,
        process_document: None,
        free_user_data: None,
    };

    // SAFETY: state lives for the duration of this test and outlives the bridge.
    let bridge = unsafe { KreuzbergOcrBackendBridge::new("test-ocr-stub".to_string(), vtable, state_ptr) };

    use kreuzberg::OcrBackend;
    let _ = bridge
        .process_image(&image_bytes, &kreuzberg::OcrConfig::default())
        .await;

    assert_eq!(
        state.received_len.load(Ordering::SeqCst),
        8,
        "process_image vtable received wrong length (truncated at embedded NUL?)"
    );
    assert_eq!(
        state.received_last_byte.load(Ordering::SeqCst),
        0xEF,
        "process_image vtable could not read past the embedded NUL"
    );
}

/// DocumentExtractor.extract_bytes must pass the full buffer length even when
/// the document bytes contain embedded NUL bytes.
#[tokio::test]
async fn document_extractor_vtable_extract_bytes_passes_full_length_with_embedded_nuls() {
    // 8-byte buffer; NUL at index 2.
    let content: Vec<u8> = vec![0x50, 0x4B, 0x00, 0x03, 0x14, 0x00, 0x00, 0x02];

    let state = Box::new(CallbackState::new());
    let state_ptr = state.as_ref() as *const CallbackState as *const std::ffi::c_void;

    let vtable = KreuzbergDocumentExtractorVTable {
        extract_bytes: Some(extractor_extract_bytes),
        extract_file: None,
        name_fn: None,
        version_fn: None,
        initialize_fn: None,
        shutdown_fn: None,
        supported_mime_types: None,
        priority: None,
        can_handle: None,
        free_user_data: None,
    };

    // SAFETY: state lives for the duration of this test and outlives the bridge.
    let bridge = unsafe { KreuzbergDocumentExtractorBridge::new("test-extractor-stub".to_string(), vtable, state_ptr) };

    use kreuzberg::DocumentExtractor;
    let _ = bridge
        .extract_bytes(
            &content,
            "application/octet-stream",
            &kreuzberg::ExtractionConfig::default(),
        )
        .await;

    assert_eq!(
        state.received_len.load(Ordering::SeqCst),
        8,
        "extract_bytes vtable received wrong length (truncated at embedded NUL?)"
    );
    assert_eq!(
        state.received_last_byte.load(Ordering::SeqCst),
        0x02,
        "extract_bytes vtable could not read past the embedded NUL"
    );
}

/// ImageKind numeric values: PageRaster must be 10 and Unknown must be 11.
///
/// alef ≥ v0.19.21 added PageRaster between Mask (9) and Unknown, bumping
/// Unknown from 10 → 11. Any C/Go/Java/C# code that hardcoded Unknown = 10
/// must be updated; this test pins the new ordinals so the renumbering is
/// visible to CI.
#[test]
fn image_kind_page_raster_is_10_and_unknown_is_11() {
    // SAFETY: pure integer dispatch, no pointers.
    assert_eq!(
        unsafe { kreuzberg_ffi::kreuzberg_image_kind_from_i32(10) },
        10,
        "PageRaster == 10"
    );
    assert_eq!(
        unsafe { kreuzberg_ffi::kreuzberg_image_kind_from_i32(11) },
        11,
        "Unknown == 11"
    );
    // Old Unknown value must now resolve to PageRaster, not Unknown.
    assert_ne!(
        unsafe { kreuzberg_ffi::kreuzberg_image_kind_from_i32(10) },
        -1,
        "10 must be valid"
    );
}
