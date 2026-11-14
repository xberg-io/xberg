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
