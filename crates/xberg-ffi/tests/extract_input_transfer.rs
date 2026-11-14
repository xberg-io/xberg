use xberg_ffi::{
    xberg_extract, xberg_extract_input_from_json_with_bytes, xberg_extract_owned, xberg_extraction_config_free,
    xberg_extraction_config_from_json, xberg_extraction_result_free, xberg_last_error_code,
};

#[test]
fn ownership_slot_preserves_input_before_transfer_and_clears_it_after_transfer() {
    let bytes = b"transport probe";
    let metadata = c"{\"kind\":\"bytes\",\"filename\":\"probe.txt\"}";
    // SAFETY: strings are NUL-terminated, the payload is valid for its length, and the
    // ownership slot is writable and exclusively held by this test.
    unsafe {
        let mut input = xberg_extract_input_from_json_with_bytes(bytes.as_ptr(), bytes.len(), metadata.as_ptr());
        let config = xberg_extraction_config_from_json(c"{\"use_cache\":false}".as_ptr());
        assert_ne!(input, 0);
        assert_ne!(config, 0);
        let original = input;

        assert_eq!(xberg_extract_owned(&mut input, 0), 0);
        assert_ne!(xberg_last_error_code(), 0);
        assert_eq!(input, original);

        for _ in 0..2 {
            let result = xberg_extract(input, config);
            assert_ne!(result, 0);
            xberg_extraction_result_free(result);
        }

        let result = xberg_extract_owned(&mut input, config);
        assert_ne!(result, 0);
        assert_eq!(input, 0);
        xberg_extraction_result_free(result);
        assert_eq!(xberg_extract(original, config), 0);
        assert_ne!(xberg_last_error_code(), 0);
        let bad_metadata = c"{\"kind\":\"bytes\",\"mime_type\":\"application/x-unsupported-test\"}";
        let mut bad_input =
            xberg_extract_input_from_json_with_bytes(bytes.as_ptr(), bytes.len(), bad_metadata.as_ptr());
        assert_ne!(bad_input, 0);
        assert_eq!(xberg_extract_owned(&mut bad_input, config), 0);
        assert_ne!(xberg_last_error_code(), 0);
        assert_eq!(bad_input, 0);
        xberg_extraction_config_free(config);
    }
}

#[test]
#[cfg(all(feature = "pdf", feature = "layout-detection", feature = "redaction"))]
fn companion_argument_validation_leaves_the_input_owned_by_the_caller() {
    use xberg_ffi::{
        xberg_extract_input_free, xberg_extract_with_external_redaction_owned, xberg_extract_with_layout_pages_owned,
    };

    let bytes = b"transport probe";
    // SAFETY: the test owns the writable slot and all pointers refer to live inputs.
    unsafe {
        let mut input = xberg_extract_input_from_json_with_bytes(
            bytes.as_ptr(),
            bytes.len(),
            c"{\"kind\":\"bytes\",\"filename\":\"probe.txt\"}".as_ptr(),
        );
        let config = xberg_extraction_config_from_json(c"{\"use_cache\":false}".as_ptr());
        assert_ne!(input, 0);
        assert_ne!(config, 0);
        let original = input;
        assert_eq!(
            xberg_extract_with_layout_pages_owned(&mut input, config, c"[".as_ptr()),
            0
        );
        assert_ne!(xberg_last_error_code(), 0);
        assert_eq!(input, original);
        assert_eq!(
            xberg_extract_with_external_redaction_owned(
                &mut input,
                config,
                std::ptr::null(),
                std::ptr::null(),
                u32::MAX,
            ),
            0
        );
        assert_ne!(xberg_last_error_code(), 0);
        assert_eq!(input, original);
        xberg_extract_input_free(input);
        xberg_extraction_config_free(config);
    }
}

#[test]
fn omitted_kind_extracts_supplied_bytes() {
    use std::ffi::CStr;
    use xberg_ffi::{xberg_extraction_result_to_json, xberg_free_string};

    let bytes = b"split byte kind probe";
    let metadata = c"{\"filename\":\"probe.txt\",\"mime_type\":\"text/plain\"}";
    // SAFETY: all strings and payload pointers are live, and this test owns the handles.
    unsafe {
        let mut input = xberg_extract_input_from_json_with_bytes(bytes.as_ptr(), bytes.len(), metadata.as_ptr());
        let config = xberg_extraction_config_from_json(c"{\"use_cache\":false}".as_ptr());
        assert_ne!(input, 0);
        assert_ne!(config, 0);
        let result = xberg_extract_owned(&mut input, config);
        xberg_extraction_config_free(config);
        assert_eq!(input, 0);
        assert_ne!(result, 0);
        let json = xberg_extraction_result_to_json(result);
        assert!(!json.is_null());
        let value: serde_json::Value = serde_json::from_slice(CStr::from_ptr(json).to_bytes()).unwrap();
        xberg_free_string(json);
        xberg_extraction_result_free(result);
        assert!(value.get("errors").is_none());
        assert_eq!(value["results"].as_array().unwrap().len(), 1);
        assert_eq!(
            value["results"][0]["content"].as_str().unwrap().trim(),
            "split byte kind probe"
        );
    }
}

#[test]
fn split_constructor_rejects_uri_metadata() {
    use std::ffi::CStr;
    use xberg_ffi::{xberg_extract_input_free, xberg_last_error_context};

    let bytes = b"split byte kind probe";
    for (metadata, message) in [
        (c"{\"kind\":\"uri\"}", "kind must be bytes"),
        (c"{\"kind\":\"bytes\",\"uri\":\"conflict.txt\"}", "omit uri"),
        (c"{\"uri\":\"conflict.txt\"}", "omit uri"),
    ] {
        // SAFETY: metadata and payload are live, and the error context is read before another FFI call.
        unsafe {
            let input = xberg_extract_input_from_json_with_bytes(bytes.as_ptr(), bytes.len(), metadata.as_ptr());
            if input != 0 {
                xberg_extract_input_free(input);
                panic!("split constructor accepted {metadata:?}");
            }
            assert_ne!(xberg_last_error_code(), 0);
            let context = xberg_last_error_context();
            assert!(!context.is_null());
            assert!(CStr::from_ptr(context).to_string_lossy().contains(message));
        }
    }
}
