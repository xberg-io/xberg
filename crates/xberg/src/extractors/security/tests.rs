use super::*;

#[cfg(feature = "office")]
fn archive_with_compressible_entry(entry_size: usize) -> zip::ZipArchive<std::io::Cursor<Vec<u8>>> {
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    const STORED_BALLAST_SIZE: usize = 4 * 1024 * 1024;

    let mut bytes = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
        let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        writer.start_file("ballast.bin", stored).expect("start stored entry");
        writer
            .write_all(&vec![0x5a; STORED_BALLAST_SIZE])
            .expect("write stored ballast");

        let deflated = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file("compact.bin", deflated)
            .expect("start deflated entry");
        writer.write_all(&vec![0; entry_size]).expect("write compact entry");
        writer.finish().expect("finish ZIP");
    }
    zip::ZipArchive::new(Cursor::new(bytes)).expect("open ZIP")
}

#[cfg(feature = "office")]
#[test]
fn zip_ratio_allows_small_highly_compressible_entry() {
    let mut archive = archive_with_compressible_entry(337 * 1024);

    assert!(
        ZipBombValidator::new(SecurityLimits::default())
            .validate(&mut archive)
            .is_ok()
    );
}

#[cfg(feature = "office")]
#[test]
fn zip_ratio_rejects_large_highly_compressible_entry() {
    let mut archive = archive_with_compressible_entry(4 * 1024 * 1024);

    assert!(matches!(
        ZipBombValidator::new(SecurityLimits::default()).validate(&mut archive),
        Err(SecurityError::ZipBombDetected { uncompressed_size, .. })
            if uncompressed_size == 4 * 1024 * 1024
    ));
}

#[test]
fn test_default_limits() {
    let limits = SecurityLimits::default();
    assert_eq!(limits.max_archive_size, 500 * 1024 * 1024);
    assert_eq!(limits.max_nesting_depth, 1024);
    assert_eq!(limits.max_entity_length, 1024 * 1024);
    assert_eq!(limits.max_table_cells, 100_000);
}

#[test]
fn test_default_max_pages_is_unlimited() {
    // A default that rejects real documents would be worse than the risk it
    // mitigates (issue #1451): the ceiling only applies once a caller opts in.
    assert_eq!(SecurityLimits::default().max_pages, None);
}

#[test]
fn test_too_many_pages_display_names_the_limit() {
    let error = SecurityError::TooManyPages {
        count: 4_000,
        max: 1_000,
    };
    let message = error.to_string();
    assert!(
        message.contains("4000"),
        "message must name the observed count: {message}"
    );
    assert!(
        message.contains("1000"),
        "message must name the configured max: {message}"
    );
    assert!(
        message.contains("max_pages"),
        "message must name the limit that was hit: {message}"
    );
}

#[test]
fn test_string_growth_validator_basic() {
    let mut v = StringGrowthValidator::new(100);
    assert!(v.check_append(50).is_ok());
    assert_eq!(v.current_size, 50);
    assert!(v.check_append(50).is_ok());
    assert_eq!(v.current_size, 100);
    assert!(matches!(
        v.check_append(1),
        Err(SecurityError::ContentTooLarge { size: 101, max: 100 })
    ));
}

#[test]
fn test_string_growth_validator_saturates_on_overflow() {
    let mut v = StringGrowthValidator::new(usize::MAX - 10);
    assert!(v.check_append(usize::MAX).is_err(), "saturating add cannot wrap");
}

#[test]
fn test_iteration_validator_basic() {
    let mut v = IterationValidator::new(3);
    assert!(v.check_iteration().is_ok());
    assert!(v.check_iteration().is_ok());
    assert!(v.check_iteration().is_ok());
    assert!(matches!(
        v.check_iteration(),
        Err(SecurityError::TooManyIterations { count: 4, max: 3 })
    ));
}

#[test]
fn test_depth_validator_push_pop() {
    let mut v = DepthValidator::new(3);
    assert!(v.push().is_ok());
    assert!(v.push().is_ok());
    assert!(v.push().is_ok());
    assert_eq!(v.current_depth, 3);
    assert!(matches!(
        v.push(),
        Err(SecurityError::NestingTooDeep { depth: 4, max: 3 })
    ));
    v.pop();
    assert_eq!(v.current_depth, 3);
}

#[test]
fn test_depth_validator_pop_saturates_at_zero() {
    let mut v = DepthValidator::new(10);
    v.pop();
    v.pop();
    assert_eq!(v.current_depth, 0, "underflow is impossible");
}

#[test]
fn test_entity_validator() {
    let v = EntityValidator::new(10);
    assert!(v.validate("short").is_ok());
    assert!(v.validate("0123456789").is_ok());
    assert!(matches!(
        v.validate("01234567890"),
        Err(SecurityError::EntityTooLong { length: 11, max: 10 })
    ));
    #[cfg(any(feature = "xml", feature = "office"))]
    {
        assert!(v.check_attr("href", "http://x").is_ok());
        assert!(v.check_attr("data", &"x".repeat(50)).is_err());
    }
}

#[test]
fn test_table_validator() {
    let mut v = TableValidator::new(10);
    assert!(v.add_cells(5).is_ok());
    assert_eq!(v.current_cells, 5);
    assert!(v.add_cells(5).is_ok());
    assert_eq!(v.current_cells, 10);
    let error = v
        .add_cells(1)
        .expect_err("the eleventh cell must exceed a ten-cell budget");
    assert!(matches!(&error, SecurityError::TooManyCells { cells: 11, max: 10 }));
    assert_eq!(
        error.to_string(),
        "Table cell limit exceeded: observed 11 cells, but `security_limits.max_table_cells` is 10. \
         If this input is trusted, raise `security_limits.max_table_cells`; otherwise reduce or split the table."
    );
}

#[test]
fn test_security_budget_depth_uses_the_tighter_of_the_two_configured_limits() {
    let nesting_is_tighter = SecurityLimits {
        max_xml_depth: 1024,
        max_nesting_depth: 5,
        ..SecurityLimits::default()
    };
    assert_eq!(
        SecurityBudget::from_limits(&nesting_is_tighter).depth.max_depth,
        5,
        "a tightened max_nesting_depth must not be discarded in favour of max_xml_depth"
    );

    let xml_is_tighter = SecurityLimits {
        max_xml_depth: 3,
        max_nesting_depth: 1024,
        ..SecurityLimits::default()
    };
    assert_eq!(
        SecurityBudget::from_limits(&xml_is_tighter).depth.max_depth,
        3,
        "a tightened max_xml_depth must not be discarded in favour of max_nesting_depth"
    );

    assert_eq!(
        SecurityBudget::from_limits(&SecurityLimits::default()).depth.max_depth,
        1024,
        "both defaults are 1024, so the default budget is unchanged"
    );
}

// `resolve_container_entry` matrix. Each case below is named after the input class
// from the path-traversal unification brief so the test file doubles as the
// executable version of that comparison table.

#[test]
fn parent_relative_target_in_bounds_pops_into_the_root() {
    // "../x" against a one-level base: the ".." exactly cancels "a", landing on "x"
    // at the container root. This is the PPTX/DOCX "spec-correct ../media/x.png" shape.
    assert_eq!(resolve_container_entry("a", "../x"), Ok("x".to_string()));
}

#[test]
fn parent_relative_target_out_of_bounds_at_the_root_is_rejected() {
    // Same "../x", but there is no base directory left to pop: this is a real escape.
    assert_eq!(
        resolve_container_entry("", "../x"),
        Err(PathTraversalError::EscapesRoot)
    );
}

#[test]
fn double_parent_within_a_single_level_base_escapes() {
    // "a/../../x": one push, then two pops. The base is empty, so after the local
    // "a" is popped there is nothing left for the second "..".
    assert_eq!(
        resolve_container_entry("", "a/../../x"),
        Err(PathTraversalError::EscapesRoot)
    );
}

#[test]
fn double_parent_within_a_two_level_base_is_in_bounds() {
    // Same shape, but the base has enough depth ("root") to absorb both "a" and the
    // outer "..": the whole path collapses to a container-root-relative "x".
    assert_eq!(resolve_container_entry("root", "a/../../x"), Ok("x".to_string()));
}

#[test]
fn absolute_target_is_root_relative_and_ignores_base() {
    // A leading "/" means "relative to the container root" (the OPC/EPUB convention),
    // not the host filesystem -- `base` is completely bypassed.
    assert_eq!(resolve_container_entry("word", "/abs/x"), Ok("abs/x".to_string()));
}

#[test]
fn windows_drive_letter_target_is_rejected() {
    assert_eq!(
        resolve_container_entry("word", "C:\\x"),
        Err(PathTraversalError::DriveOrUncPrefix)
    );
}

#[test]
fn unc_style_target_is_rejected() {
    // "\\server\share\x" normalises to "//server/share/x", which is caught by the
    // same UNC check as a literal "//..." input -- no separate UNC-specific parsing.
    assert_eq!(
        resolve_container_entry("word", "\\\\server\\share\\x"),
        Err(PathTraversalError::DriveOrUncPrefix)
    );
}

#[test]
fn backslash_traversal_is_normalised_the_same_as_forward_slash() {
    // "a\..\..\x": backslashes are converted to "/" explicitly, so this behaves
    // identically on every build platform instead of depending on `std::path`'s
    // target-dependent component parsing (the drift `has_path_traversal` had).
    assert_eq!(
        resolve_container_entry("", "a\\..\\..\\x"),
        Err(PathTraversalError::EscapesRoot)
    );
}

#[test]
fn dot_segments_are_transparent_to_in_bounds_traversal() {
    // "a/./../x": the "." is a no-op and the ".." cancels "a", leaving "x".
    assert_eq!(resolve_container_entry("", "a/./../x"), Ok("x".to_string()));
}

#[test]
fn four_dots_is_a_literal_component_not_a_traversal_token() {
    // "....//x": "...." is not the exact string "..", so it is pushed as an ordinary
    // (if unusual) literal segment. The doubled slash contributes an empty component,
    // which is dropped.
    assert_eq!(resolve_container_entry("", "....//x"), Ok("..../x".to_string()));
}

#[test]
fn bare_dotdot_against_a_one_level_base_has_no_file_left_to_resolve() {
    assert_eq!(resolve_container_entry("a", ".."), Err(PathTraversalError::EmptyResult));
}

#[test]
fn bare_dotdot_against_the_root_escapes() {
    assert_eq!(resolve_container_entry("", ".."), Err(PathTraversalError::EscapesRoot));
}

#[test]
fn empty_components_are_skipped() {
    assert_eq!(resolve_container_entry("", "a//b"), Ok("a/b".to_string()));
}

#[test]
fn trailing_dotdot_resolves_to_the_base_directory_itself() {
    // "a/..": pushes "a" onto "root" then immediately pops it back off, landing
    // exactly on the base -- allowed, even though the result names a directory
    // rather than a file (the caller's `by_name` lookup will simply miss).
    assert_eq!(resolve_container_entry("root", "a/.."), Ok("root".to_string()));
}

#[test]
fn nul_byte_is_rejected_outright() {
    assert_eq!(
        resolve_container_entry("word", "media/\0image1.png"),
        Err(PathTraversalError::InvalidByte)
    );
}

#[test]
fn percent_encoded_traversal_is_never_decoded_by_this_function() {
    // "%2e%2e%2f" contains no literal '/' -- it is one opaque literal segment here.
    // Decoding is the caller's job, and must happen *before* calling this function
    // (EPUB's `resolve_path` does exactly that); decoding afterwards would let a
    // decoded "../" slip past a boundary check that already ran.
    assert_eq!(
        resolve_container_entry("base", "%2e%2e%2f"),
        Ok("base/%2e%2e%2f".to_string())
    );
}

#[test]
fn multibyte_character_after_dotdot_is_a_literal_segment_not_a_slice_panic() {
    // ".." followed by a 4-byte emoji is not the exact string "..", so the whole
    // thing is pushed as a literal segment. Exact string comparison (rather than the
    // old PPTX code's fixed-byte-offset slice) can never land mid-character.
    assert_eq!(
        resolve_container_entry("base", "..\u{1F600}/x"),
        Ok("base/..\u{1F600}/x".to_string())
    );
}

// DOCX-specific cases: `docx.rs` calls this with `base = "word"`. These pin the two
// behaviours called out in the unification plan as a deliberate change from the
// deleted `has_path_traversal`, which rejected every `..` unconditionally.

#[test]
fn docx_word_relative_target_climbs_to_the_package_root_media_directory() {
    // The normal shape for a DOCX image whose relationship lives at the package
    // root's "media/" directory, one level above "word/". `has_path_traversal` used
    // to reject this outright; it is legitimate and must resolve.
    assert_eq!(
        resolve_container_entry("word", "../media/image1.png"),
        Ok("media/image1.png".to_string())
    );
}

#[test]
fn docx_word_relative_target_that_truly_escapes_the_package_still_errors() {
    assert_eq!(
        resolve_container_entry("word", "../../../etc/passwd"),
        Err(PathTraversalError::EscapesRoot)
    );
}

#[test]
fn docx_absolute_target_reroots_to_the_package_relative_name() {
    // `has_path_traversal` allowed this (asserted deliberately at what was
    // `security.rs:891`) and DOCX then re-rooted it by hand with `strip_prefix('/')`.
    // The shared helper folds that re-rooting into the same call.
    assert_eq!(
        resolve_container_entry("word", "/media/image1.png"),
        Ok("media/image1.png".to_string())
    );
}

/// Bytes that deflate to about their own size, like a photograph.
#[cfg(feature = "office")]
fn incompressible(len: usize) -> Vec<u8> {
    let mut state = 0x9E37_79B9u32;
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        })
        .collect()
}

#[cfg(feature = "office")]
fn deflated_archive(members: &[(&str, Vec<u8>)]) -> zip::ZipArchive<std::io::Cursor<Vec<u8>>> {
    use std::io::Write;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in members {
            writer.start_file(*name, options).expect("start_file");
            writer.write_all(bytes).expect("write");
        }
        writer.finish().expect("finish");
    }
    cursor.set_position(0);
    zip::ZipArchive::new(cursor).expect("archive")
}

#[cfg(feature = "office")]
#[test]
fn zip_bomb_ratio_cap_ignores_small_members_and_keeps_large_ones() {
    let validator = ZipBombValidator::new(SecurityLimits::default());

    // A blank-page image that deflates past 100:1 next to a real photograph:
    // the whole-archive ratio stays near 1:1, so only the per-member cap decides.
    let mut small = deflated_archive(&[
        ("blank.jpg", vec![b'A'; 200 * 1024]),
        ("photo.jpg", incompressible(2 * 1024 * 1024)),
    ]);
    validator
        .validate(&mut small)
        .expect("a 200 KiB member past the ratio cap is not a bomb");

    let mut large = deflated_archive(&[
        ("bomb.bin", vec![b'A'; 8 * 1024 * 1024]),
        ("photo.jpg", incompressible(2 * 1024 * 1024)),
    ]);
    let error = validator
        .validate(&mut large)
        .expect_err("an 8 MiB member past the ratio cap is rejected");
    assert!(matches!(error, SecurityError::ZipBombDetected { .. }), "{error}");
}

#[cfg(feature = "archives")]
#[test]
fn archive_budget_refuses_a_charge_past_the_limit_without_consuming_it() {
    let limits = SecurityLimits {
        max_archive_size: 100,
        max_content_size: 10,
        ..Default::default()
    };
    let budget = ArchiveBudget::new(&limits);

    budget.charge_decompressed(60).expect("60 of 100 fits");
    assert_eq!(budget.remaining_decompressed(), 40);
    assert!(matches!(
        budget.charge_decompressed(41),
        Err(SecurityError::ArchiveTooLarge { size: 101, max: 100 })
    ));
    assert_eq!(
        budget.remaining_decompressed(),
        40,
        "a refused charge must not consume budget"
    );
    budget.charge_decompressed(40).expect("exactly the remainder fits");
    assert_eq!(budget.remaining_decompressed(), 0);
    assert!(
        budget.charge_decompressed(usize::MAX).is_err(),
        "a huge charge must not wrap"
    );

    budget.charge_child_output(10).expect("10 of 10 fits");
    assert!(matches!(
        budget.charge_child_output(1),
        Err(SecurityError::ContentTooLarge { size: 11, max: 10 })
    ));
}
