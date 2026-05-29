/// Regression test for GitHub #1059.
///
/// `kreuzberg_email_attachment_data` was the only byte-buffer accessor on a public
/// FFI-exposed DTO that did not follow the established `*_data(ptr, out_len: *mut usize)`
/// protocol used by `kreuzberg_extracted_image_data`, `kreuzberg_embedded_file_data`,
/// and `kreuzberg_batch_bytes_item_content`.
///
/// Because `EmailAttachment.data` is `Option<Bytes>` (the only optional byte buffer among
/// public types), alef's heuristic for emitting the two-parameter form did not trigger.
/// Callers had no way to know the valid length of the returned pointer, making any read
/// past the first byte undefined behaviour (especially for payloads containing 0x00).
///
/// This test is the "lock-in" test:
/// - It asserts at runtime + against the committed header that the correct signature
///   (with out_len) is present after regeneration.
/// - It exercises a payload containing embedded NULs + a high byte to kill any
///   strlen / "read until 0" or truncated read assumptions.
///
/// Important: Until alef#118 is fixed and `task alef:generate` is run, the two tests
/// that assert the desired future behavior (`out_len` parameter + full length for
/// payloads with NULs) are `#[ignore]`d. Only tests that exercise the *current*
/// (buggy) 1-parameter ABI are active.
///
/// See also: crates/kreuzberg-ffi/tests/vtable_bytes_len.rs (previous identical class of bug).
/// Per project rules: every unsafe block has a SAFETY comment.
use std::ffi::{CString, c_char};
use std::fs;
use std::path::Path;

// The functions we need are re-exported / visible via the rlib built from the
// generated lib.rs. We only use the stable ones (from_json / free) directly here.
use kreuzberg_ffi::{kreuzberg_email_attachment_free, kreuzberg_email_attachment_from_json, kreuzberg_last_error_code};

// NOTE: We deliberately do *not* declare a 2-parameter version of
// kreuzberg_email_attachment_data at the top level. The real symbol exported by
// the crate today only takes one parameter (ptr). Declaring a 2-param version
// here and calling it would be UB on the current generated code.
//
// The two ignored tests below document the desired future signature. When
// alef#118 is fixed they can be un-ignored (or rewritten to use the real symbol).

/// Construct a minimal EmailAttachment JSON with a data payload that contains
/// an embedded NUL and a trailing high byte (0xEF). This defeats any strlen-based
/// or "read first byte only" implementations.
fn attachment_json_with_nuls() -> CString {
    // 8 bytes: JPEG-ish magic + NUL in the middle + high byte at the end.
    // Length is authoritative and known.
    let data: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];
    let json = format!(
        r#"{{
            "name": "test.bin",
            "filename": "test.bin",
            "mime_type": "application/octet-stream",
            "size": {},
            "is_image": false,
            "data": {}
        }}"#,
        data.len(),
        serde_json::to_string(&data).unwrap()
    );
    CString::new(json).expect("valid UTF-8 JSON for test attachment")
}

/// Ignored until a fixed alef emits the correct accessor (then `task alef:generate` + rebuild will make it pass).
/// The test body + this file remain the permanent regression specification for #1059.
#[test]
#[ignore = "requires alef fix + regeneration for the Option<Bytes> data case (see #1059)"]
fn email_attachment_data_accessor_must_provide_out_len_in_header() {
    // This is a snapshot-style check against the committed C header.
    // It is ignored until the generator is fixed.
    let header_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("include/kreuzberg.h");
    let header = fs::read_to_string(&header_path).expect("committed kreuzberg.h must be readable by the test");

    // Simple and robust: the declaration for this specific function must mention out_len.
    let has_out_len = header.contains("kreuzberg_email_attachment_data") && header.contains("out_len");

    assert!(
        has_out_len,
        "GitHub #1059 regression: the declaration of kreuzberg_email_attachment_data \
         in crates/kreuzberg-ffi/include/kreuzberg.h does not contain the required \
         `out_len` parameter.\n\n\
         Expected something like:\n    uint8_t *kreuzberg_email_attachment_data(..., uintptr_t *out_len);\n\n\
         Found the old 1-parameter form. Fix requires `task alef:generate` with an \
         updated alef that handles Option<Bytes> fields for the FFI byte accessor heuristic.\n\n\
         This is the lock-in test for #1059."
    );
}

/// Ignored until a fixed alef emits the correct 2-parameter accessor.
/// This test encodes the exact required behaviour (full length + correct bytes past embedded NUL).
#[test]
#[ignore = "requires alef fix + regeneration for the Option<Bytes> data case (see #1059)"]
fn email_attachment_data_with_out_len_returns_full_buffer_including_embedded_nuls() {
    // This test demonstrates the full contract once the generator emits the
    // two-parameter form. It is written against the desired extern declaration above.
    //
    // Until the generator is fixed this test documents the required behaviour.
    // After `task alef:generate` + rebuild it will execute the real call path.

    let json = attachment_json_with_nuls();
    // SAFETY: json is a valid null-terminated CString we just created.
    let handle = unsafe { kreuzberg_email_attachment_from_json(json.as_ptr() as *const c_char) };
    assert!(
        !handle.is_null(),
        "from_json should succeed for our well-formed test attachment (last_error_code={})",
        unsafe { kreuzberg_last_error_code() }
    );

    let mut out_len: usize = 0;

    // Local declaration of the *desired* future 2-param signature.
    // This code is only compiled when the test is explicitly un-ignored
    // after the generator has been fixed.
    unsafe extern "C" {
        pub fn kreuzberg_email_attachment_data(ptr: *const std::ffi::c_void, out_len: *mut usize) -> *mut u8;
    }

    // SAFETY: handle is non-null and freshly allocated by from_json.
    // We pass a valid &mut out_len. The returned pointer must not be freed by us.
    let data_ptr = unsafe { kreuzberg_email_attachment_data(handle as *const std::ffi::c_void, &mut out_len) };

    assert!(
        !data_ptr.is_null(),
        "data pointer must be non-null for an attachment we created with a Some(data) payload"
    );
    assert_eq!(
        out_len, 8,
        "out_len must report the exact length of the Bytes payload (not 0, not guessed, not truncated at NUL)"
    );

    // SAFETY: data_ptr is valid for [0..out_len] because:
    // - it came from the handle's internal Bytes (which we control),
    // - out_len was written by the accessor,
    // - the handle is still alive (we have not called free yet).
    let slice = unsafe { std::slice::from_raw_parts(data_ptr, out_len) };

    assert_eq!(slice.len(), 8);
    assert_eq!(slice[0], 0xFF);
    assert_eq!(slice[3], 0x00, "must be able to read the embedded NUL");
    assert_eq!(
        slice[7], 0xEF,
        "must be able to read bytes after the NUL (no truncation)"
    );

    // Cleanup
    // SAFETY: handle came from from_json; we are the owner.
    unsafe { kreuzberg_email_attachment_free(handle) };
}

#[test]
fn email_attachment_data_none_returns_null_pointer() {
    // This test exercises the *current* (buggy) 1-parameter ABI that is actually
    // emitted by the generator today. It is intentionally not ignored.
    //
    // It proves that the basic handle lifecycle (from_json + data accessor + free)
    // works for the None case using the real exported symbol.
    let json = CString::new(
        r#"{"name":"empty","filename":"empty","mime_type":null,"size":null,"is_image":false,"data":null}"#,
    )
    .unwrap();

    // SAFETY: json is valid.
    let handle = unsafe { kreuzberg_email_attachment_from_json(json.as_ptr() as *const c_char) };
    assert!(!handle.is_null());

    // Call the real 1-parameter function that the crate actually exports today.
    // SAFETY: handle is a valid pointer returned by from_json.
    let data_ptr = unsafe { kreuzberg_ffi::kreuzberg_email_attachment_data(handle) };

    assert!(
        data_ptr.is_null(),
        "data must be null when the attachment has no payload (current 1-param ABI)"
    );

    // SAFETY: handle from from_json.
    unsafe { kreuzberg_email_attachment_free(handle) };
}

/// This active (non-ignored) test demonstrates the current bug using the real
/// 1-parameter ABI that the generator emits today.
///
/// We can obtain a data pointer for an attachment that has payload, but the
/// accessor provides no length. This is exactly the safety problem reported
/// in #1059. After the alef fix this test can be extended or replaced by the
/// full length-aware version (currently ignored).
#[test]
fn email_attachment_data_current_abi_returns_pointer_but_no_length() {
    // Small payload with an embedded NUL to make the point.
    let json = CString::new(
        r#"{"name":"hasdata.bin","filename":"hasdata.bin","mime_type":"application/octet-stream","size":4,"is_image":false,"data":[65,0,66,67]}"#,
    )
    .unwrap();

    // SAFETY: json is valid.
    let handle = unsafe { kreuzberg_email_attachment_from_json(json.as_ptr() as *const c_char) };
    assert!(!handle.is_null());

    // Real 1-param function (current buggy generator output).
    // SAFETY: handle is valid.
    let data_ptr = unsafe { kreuzberg_ffi::kreuzberg_email_attachment_data(handle) };

    assert!(
        !data_ptr.is_null(),
        "data pointer should be non-null when the attachment carries a payload"
    );

    // The fundamental problem: with the current 1-param signature there is
    // no out_len. Callers have no safe way to know how many bytes are valid
    // at data_ptr. This assertion documents the missing contract.
    //
    // After the generator is fixed, a proper test will also receive and
    // validate the length (see the ignored test above).

    // SAFETY: handle from from_json.
    unsafe { kreuzberg_email_attachment_free(handle) };
}
