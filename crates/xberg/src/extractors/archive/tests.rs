use super::*;
use std::io::{Cursor, Write};
use tar::Builder as TarBuilder;
use zip::write::{FileOptions, ZipWriter};

#[tokio::test]
async fn test_zip_extractor() {
    let extractor = ZipExtractor::new();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("test.txt", options).unwrap();
        zip.write_all(b"Hello, World!").unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&bytes, "application/zip", &config)
        .await
        .unwrap();
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

    assert_eq!(result.mime_type, "application/zip");
    assert!(result.content.contains("ZIP Archive"));
    assert!(result.content.contains("test.txt"));
    assert!(result.content.contains("Hello, World!"));
    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        crate::types::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.format, "ZIP");
    assert_eq!(archive_meta.file_count, 1);
}

/// Regression test for #121: member bodies were emitted by iterating an `AHashMap`, whose
/// order aHash randomizes per process, so the same archive rendered differently on every run.
///
/// The member names below are deliberately anti-alphabetical, so this also pins *which*
/// deterministic order was chosen: archive order, matching the "Files:" listing — a plain
/// `sort()` would have produced alpha/middle/zebra and failed here.
#[tokio::test]
async fn should_emit_archive_members_in_archive_order_not_hash_order() {
    let extractor = ZipExtractor::new();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        for (name, body) in [
            ("zebra.txt", "zebra body"),
            ("alpha.txt", "alpha body"),
            ("middle.txt", "middle body"),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&bytes, "application/zip", &config)
        .await
        .unwrap();
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

    let positions: Vec<usize> = ["=== zebra.txt ===", "=== alpha.txt ===", "=== middle.txt ==="]
        .iter()
        .map(|marker| {
            result
                .content
                .find(marker)
                .unwrap_or_else(|| panic!("{marker} missing from {:?}", result.content))
        })
        .collect();

    assert!(
        positions[0] < positions[1] && positions[1] < positions[2],
        "members must appear in archive order (zebra, alpha, middle); got offsets {positions:?} in {:?}",
        result.content
    );
}

#[tokio::test]
async fn test_zip_filters_bookkeeping_and_binary_junk_from_children() {
    let extractor = ZipExtractor::new();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();

        zip.start_file("report.txt", options).unwrap();
        zip.write_all(b"Quarterly report body.").unwrap();

        // macOS Finder bookkeeping file (binary "Bud1..." header).
        zip.start_file(".DS_Store", options).unwrap();
        zip.write_all(&[0x00, 0x00, 0x00, 0x01, b'B', b'u', b'd', b'1'])
            .unwrap();

        // AppleDouble resource fork sidecar under the macOS archive-utility folder.
        zip.start_file("__MACOSX/._report.txt", options).unwrap();
        zip.write_all(&[
            0x00, 0x05, 0x16, 0x07, 0x00, 0x02, b'M', b'a', b'c', b' ', b'O', b'S', b' ', b'X',
        ])
        .unwrap();

        // AppleDouble sidecar at the top level (same file, no __MACOSX wrapper).
        zip.start_file("._report.txt", options).unwrap();
        zip.write_all(&[
            0x00, 0x05, 0x16, 0x07, 0x00, 0x02, b'M', b'a', b'c', b' ', b'O', b'S', b' ', b'X',
        ])
        .unwrap();

        // Python bytecode cache.
        zip.start_file("__pycache__/mod.cpython-311.pyc", options).unwrap();
        zip.write_all(&[0x42, 0x0d, 0x0d, 0x0a, 0x00, 0x00, 0x00, 0x00])
            .unwrap();

        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&bytes, "application/zip", &config)
        .await
        .unwrap();

    let children = result.children.expect("archive should extract the real document");
    assert_eq!(
        children.len(),
        1,
        "only report.txt should survive filtering: {children:?}"
    );
    assert_eq!(children[0].path, "report.txt");
    assert_eq!(children[0].mime_type, "text/plain");

    assert!(
        !result.processing_warnings.is_empty(),
        "expected a ProcessingWarning about filtered bookkeeping/binary entries"
    );
    let warning = &result.processing_warnings[0];
    assert_eq!(warning.source, "archive");
    assert!(
        warning.message.contains("Filtered"),
        "warning message should mention filtering: {}",
        warning.message
    );
}

/// Regression test for task #709: the per-entry recursive-extraction loop in
/// `build_archive_doc` must stop starting new entries once cancellation is
/// signalled, instead of working through every remaining entry regardless.
///
/// Proves the checkpoint actually stops work (not merely that an error comes
/// back elsewhere): the token is cancelled *before* extraction starts, so the
/// loop's first-iteration check must break before any of the 3 entries is
/// recursively extracted, leaving zero children — against code with the
/// checkpoint removed, all 3 entries are still recursed into regardless of
/// cancellation, so `children` would be `Some` with 3 entries, not `None`.
#[tokio::test]
async fn test_zip_extraction_stops_recursing_once_cancelled() {
    let extractor = ZipExtractor::new();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();
        for (name, content) in [("a.txt", "alpha"), ("b.txt", "bravo"), ("c.txt", "charlie")] {
            zip.start_file(name, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    let bytes = cursor.into_inner();

    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let cancelled_config = ExtractionConfig {
        cancel_token: Some(token),
        ..ExtractionConfig::default()
    };

    let cancelled_result = extractor
        .extract_content(&bytes, "application/zip", &cancelled_config)
        .await
        .expect("extraction must not error when cancelled, only skip recursive children");

    assert!(
        cancelled_result.children.is_none(),
        "no archive entry should be recursively extracted once the token is already \
         cancelled, got {:?}",
        cancelled_result.children
    );
    assert!(
        cancelled_result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "archive" && warning.message.contains("cancelled")),
        "a cancelled run should explain why entries were skipped: {:?}",
        cancelled_result.processing_warnings
    );

    // Sanity check: the same 3 entries, uncancelled, all recurse normally — this
    // rules out the empty result above being an unrelated bug (e.g. MIME
    // detection rejecting `.txt`) rather than the cancellation checkpoint.
    let uncancelled_result = extractor
        .extract_content(&bytes, "application/zip", &ExtractionConfig::default())
        .await
        .unwrap();
    assert_eq!(
        uncancelled_result.children.map(|c| c.len()).unwrap_or(0),
        3,
        "all 3 entries should be recursively extracted when nothing is cancelled"
    );
}

#[tokio::test]
async fn test_zip_markdown_member_routes_to_markdown_extractor() {
    let markdown = "# Title\n\nBody paragraph.\n\n## Section\n\n- a\n- b\n";

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default();
        zip.start_file("doc.md", options).unwrap();
        zip.write_all(markdown.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig {
        output_format: crate::core::config::OutputFormat::Markdown,
        ..Default::default()
    };

    let result = ZipExtractor::new()
        .extract_content(&bytes, "application/zip", &config)
        .await
        .unwrap();

    let children = result.children.expect("archive should extract its member");
    let member = children.iter().find(|c| c.path == "doc.md").unwrap();
    assert_eq!(member.mime_type, "text/markdown");

    let rendered = member
        .result
        .formatted_content
        .as_ref()
        .unwrap_or(&member.result.content);
    assert!(rendered.contains("# Title"), "heading lost: {rendered:?}");
    assert!(!rendered.contains("\\#"), "heading was escaped as prose: {rendered:?}");
}

#[tokio::test]
async fn test_tar_extractor() {
    let extractor = TarExtractor::new();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut tar = TarBuilder::new(&mut cursor);

        let data = b"Hello, World!";
        let mut header = tar::Header::new_gnu();
        header.set_path("test.txt").unwrap();
        header.set_size(data.len() as u64);
        header.set_cksum();
        tar.append(&header, &data[..]).unwrap();

        tar.finish().unwrap();
    }

    let bytes = cursor.into_inner();
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&bytes, "application/x-tar", &config)
        .await
        .unwrap();
    let result =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);

    assert_eq!(result.mime_type, "application/x-tar");
    assert!(result.content.contains("TAR Archive"));
    assert!(result.content.contains("test.txt"));
    assert!(result.content.contains("Hello, World!"));
    assert!(result.metadata.format.is_some());
    let archive_meta = match result.metadata.format.as_ref().unwrap() {
        crate::types::FormatMetadata::Archive(meta) => meta,
        _ => panic!("Expected Archive metadata"),
    };
    assert_eq!(archive_meta.format, "TAR");
    assert_eq!(archive_meta.file_count, 1);
}

#[tokio::test]
async fn test_zip_extractor_invalid() {
    let extractor = ZipExtractor::new();
    let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&invalid_bytes, "application/zip", &config)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tar_extractor_invalid() {
    let extractor = TarExtractor::new();
    let invalid_bytes = vec![0, 1, 2, 3, 4, 5];
    let config = ExtractionConfig::default();

    let result = extractor
        .extract_content(&invalid_bytes, "application/x-tar", &config)
        .await;
    assert!(result.is_err());
}

#[test]
fn test_zip_plugin_interface() {
    let extractor = ZipExtractor::new();
    assert_eq!(extractor.name(), "zip-extractor");
    assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
    assert!(extractor.supported_mime_types().contains(&"application/zip"));
    assert_eq!(extractor.priority(), 50);
}

#[test]
fn test_tar_plugin_interface() {
    let extractor = TarExtractor::new();
    assert_eq!(extractor.name(), "tar-extractor");
    assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
    assert!(extractor.supported_mime_types().contains(&"application/x-tar"));
    assert!(extractor.supported_mime_types().contains(&"application/tar"));
    assert_eq!(extractor.priority(), 50);
}

#[test]
fn test_gzip_plugin_interface() {
    let extractor = GzipExtractor::new();
    assert_eq!(extractor.name(), "gzip-extractor");
    assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
    assert!(extractor.supported_mime_types().contains(&"application/gzip"));
    assert!(extractor.supported_mime_types().contains(&"application/x-gzip"));
    assert_eq!(extractor.priority(), 50);
}

#[tokio::test]
async fn test_gzip_extractor_valid_data() {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(b"Hello from gzip extraction!").unwrap();
    let compressed = encoder.finish().unwrap();

    let extractor = GzipExtractor::new();
    let config = ExtractionConfig::default();
    let result = extractor
        .extract_content(&compressed, "application/gzip", &config)
        .await;
    assert!(result.is_ok());
    let extraction = result.unwrap();
    let extraction =
        crate::extraction::derive::derive_extraction_result(extraction, true, crate::core::config::OutputFormat::Plain);
    assert!(extraction.content.contains("Hello from gzip extraction!"));
}

#[tokio::test]
async fn tar_gz_budget_counts_only_the_gzip_stream() {
    let bytes = include_bytes!("fixtures/one-member.tar.gz");
    let payload = "Hello from tar.gz!";
    let member_size = payload.len() + 1;
    let tar_size = 10 * 1024;
    let config_for_limit = |limit| ExtractionConfig {
        max_archive_depth: 0,
        ..config_with_limits(SecurityLimits {
            max_archive_size: limit,
            ..Default::default()
        })
    };
    let extractor = GzipExtractor::new();

    let error = extractor
        .extract_content(bytes, "application/gzip", &config_for_limit(tar_size - 1))
        .await
        .expect_err("the inflated TAR exceeds the archive budget by one byte");
    let crate::XbergError::Validation { message, .. } = error else {
        panic!("expected a validation error, got {error}");
    };
    assert!(message.contains("max_archive_size"), "{message}");

    let result = extractor
        .extract_content(bytes, "application/gzip", &config_for_limit(tar_size))
        .await
        .expect("the inflated TAR fits the archive budget exactly");
    let Some(crate::types::FormatMetadata::Archive(metadata)) = &result.metadata.format else {
        panic!("expected archive metadata");
    };
    assert_eq!(metadata.file_list, ["note.txt"]);
    assert_eq!(metadata.total_size, member_size as u64);
    let extracted =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
    assert!(extracted.content.contains(payload));
}

#[tokio::test]
async fn tar_gz_budget_charges_only_the_inflated_tar_and_sparse_hole() {
    let stored = [b'x'; 512];
    let hole = 1024;
    let expanded = hole + stored.len();
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::GNUSparse);
    header.set_size(stored.len() as u64);
    let gnu = header.as_gnu_mut().unwrap();
    gnu.set_real_size(expanded as u64);
    gnu.sparse[0].set_offset(hole as u64);
    gnu.sparse[0].set_length(stored.len() as u64);

    let mut tar = TarBuilder::new(Vec::new());
    tar.append_data(&mut header, "sparse.bin", stored.as_slice()).unwrap();
    let tar = tar.into_inner().unwrap();
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&tar).unwrap();
    let gzip = gzip.finish().unwrap();

    let exact_limit = tar.len() + hole;
    for limit in [exact_limit - 1, exact_limit] {
        let limits = SecurityLimits {
            max_archive_size: limit,
            ..Default::default()
        };
        let budget = Arc::new(ArchiveBudget::new(&limits));
        let config = ExtractionConfig {
            archive_budget: Some(Arc::clone(&budget)),
            max_archive_depth: 0,
            ..config_with_limits(limits)
        };
        let result = GzipExtractor::new()
            .extract_content(&gzip, "application/gzip", &config)
            .await;

        if limit == exact_limit {
            let doc = result.unwrap();
            let Some(crate::types::FormatMetadata::Archive(metadata)) = &doc.metadata.format else {
                panic!("expected archive metadata");
            };
            assert_eq!(metadata.file_list, ["sparse.bin"]);
            assert_eq!(metadata.total_size, expanded as u64);
            assert!(doc.processing_warnings.is_empty());
            assert_eq!(budget.remaining_decompressed(), 0);
        } else {
            let error = result.unwrap_err();
            let crate::XbergError::Validation { message, .. } = error else {
                panic!("expected a validation error, got {error}");
            };
            assert!(
                message.contains("sparse.bin") && message.contains("max_archive_size"),
                "{message}"
            );
            assert_eq!(budget.remaining_decompressed(), hole - 1);
        }
    }
}

#[tokio::test]
async fn test_gzip_extractor_invalid_data() {
    let extractor = GzipExtractor::new();
    let config = ExtractionConfig::default();
    let result = extractor
        .extract_content(&[0, 1, 2, 3], "application/gzip", &config)
        .await;
    assert!(result.is_err());
}

/// A ZIP whose members are stored uncompressed, so each level's decompressed size is its
/// members' byte length and stays far below `max_compression_ratio`.
fn stored_zip(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut cursor);
        let options = FileOptions::<'_, '_, ()>::default().compression_method(zip::CompressionMethod::Stored);
        for (name, body) in members {
            writer.start_file(*name, options).unwrap();
            writer.write_all(body).unwrap();
        }
        writer.finish().unwrap();
    }
    cursor.into_inner()
}

fn config_with_limits(limits: crate::extractors::security::SecurityLimits) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        security_limits: Some(limits),
        ..ExtractionConfig::default()
    }
}

fn recursion_warnings(doc: &InternalDocument) -> Vec<&str> {
    doc.processing_warnings
        .iter()
        .filter(|warning| warning.source == "archive_recursive_extraction")
        .map(|warning| &*warning.message)
        .collect()
}

/// Every nesting level used to get a fresh `max_archive_size`, so a chain of archives each
/// under the limit did `depth` times the work the operator allowed. Each level here fits
/// the limit on its own; the two levels together exceed it by one byte.
#[tokio::test]
async fn nested_archive_levels_share_one_decompression_budget() {
    let inner_text = vec![b'x'; 3000];
    let level2 = stored_zip(&[("inner.txt", inner_text.as_slice())]);
    let level1 = stored_zip(&[("level2.zip", level2.as_slice())]);
    let root = stored_zip(&[("level1.zip", level1.as_slice())]);
    let limit = level1.len() + level2.len() - 1;
    assert!(
        level1.len() <= limit && level2.len() <= limit,
        "each level must fit the limit alone"
    );
    assert!(
        level1.len() + level2.len() > limit,
        "the levels together must exceed it"
    );

    let unbounded = ZipExtractor::new()
        .extract_content(&root, "application/zip", &ExtractionConfig::default())
        .await
        .unwrap();
    let level1_doc = &unbounded.children.as_ref().expect("level1.zip extracted")[0];
    assert_eq!(level1_doc.path, "level1.zip");
    let level2_doc = &level1_doc.result.children.as_ref().expect("level2.zip extracted")[0];
    assert_eq!(level2_doc.path, "level2.zip");
    let inner_doc = &level2_doc.result.children.as_ref().expect("inner.txt extracted")[0];
    assert_eq!(inner_doc.path, "inner.txt");

    let limits = crate::extractors::security::SecurityLimits {
        max_archive_size: limit,
        ..Default::default()
    };
    let bounded = ZipExtractor::new()
        .extract_content(&root, "application/zip", &config_with_limits(limits))
        .await
        .expect("the outermost archive fits the limit");

    assert!(
        bounded.children.is_none(),
        "level1.zip must not be extracted once the root used most of the budget: {:?}",
        bounded
            .children
            .as_ref()
            .map(|c| c.iter().map(|e| &e.path).collect::<Vec<_>>())
    );
    let warnings = recursion_warnings(&bounded);
    assert!(
        warnings
            .iter()
            .any(|message| message.contains("level1.zip") && message.contains("Archive too large")),
        "the skipped nested archive must be reported with the size limit: {warnings:?}"
    );
}

/// Sibling archives draw from the same budget: two nested archives that each fit what the
/// parent left over, but not both.
#[tokio::test]
async fn sibling_nested_archives_share_one_decompression_budget() {
    let inner_text = vec![b'y'; 3000];
    let nested = stored_zip(&[("inner.txt", inner_text.as_slice())]);
    let root = stored_zip(&[("a.zip", nested.as_slice()), ("b.zip", nested.as_slice())]);
    let limits = crate::extractors::security::SecurityLimits {
        max_archive_size: 2 * nested.len() + 2 * inner_text.len() - 1,
        ..Default::default()
    };

    let result = ZipExtractor::new()
        .extract_content(&root, "application/zip", &config_with_limits(limits))
        .await
        .expect("the outermost archive fits the limit");

    let extracted = result.children.as_ref().map_or(0, Vec::len);
    assert_eq!(extracted, 1, "only one sibling fits the remaining budget");
    let warnings = recursion_warnings(&result);
    assert_eq!(warnings.len(), 1, "the other sibling must be reported: {warnings:?}");
    assert!(warnings[0].contains("Archive too large"), "{warnings:?}");
}

/// Child results used to accumulate without any overall cap. Each child here fits
/// `max_content_size` alone; together they do not.
#[tokio::test]
async fn accumulated_child_output_is_capped_by_max_content_size() {
    let note = vec![b'z'; 450];
    let nested = stored_zip(&[("note.txt", note.as_slice())]);
    let root = stored_zip(&[
        ("one.zip", nested.as_slice()),
        ("two.zip", nested.as_slice()),
        ("three.zip", nested.as_slice()),
        ("four.zip", nested.as_slice()),
    ]);
    let max_content_size = 1000;
    let config = ExtractionConfig {
        max_archive_depth: 1,
        ..config_with_limits(crate::extractors::security::SecurityLimits {
            max_content_size,
            ..Default::default()
        })
    };

    let result = ZipExtractor::new()
        .extract_content(&root, "application/zip", &config)
        .await
        .expect("the outermost archive fits the limits");

    let children = result.children.unwrap_or_default();
    let retained: usize = children.iter().map(|child| child.result.content.len()).sum();
    assert!(
        children.len() < 4,
        "the four children together exceed the budget, but all were kept"
    );
    assert!(
        retained <= max_content_size,
        "retained child output {retained} exceeds max_content_size {max_content_size}"
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "archive" && warning.message.contains("max_content_size")),
        "dropping children must be reported: {:?}",
        result.processing_warnings
    );
}

#[tokio::test]
async fn nested_tar_budget_counts_both_duplicate_members_once() {
    let mut tar = TarBuilder::new(Vec::new());
    for payload in [b"old", b"new"] {
        let mut header = tar::Header::new_ustar();
        header.set_mode(0o644);
        header.set_size(payload.len() as u64);
        tar.append_data(&mut header, "note.txt", &payload[..]).unwrap();
    }
    let nested = tar.into_inner().unwrap();
    let root = stored_zip(&[("nested.tar", &nested)]);
    let exact_limit = nested.len() + b"old".len() + b"new".len();

    for limit in [exact_limit - 1, exact_limit] {
        let limits = SecurityLimits {
            max_archive_size: limit,
            ..Default::default()
        };
        let budget = Arc::new(ArchiveBudget::new(&limits));
        let config = ExtractionConfig {
            archive_budget: Some(Arc::clone(&budget)),
            ..config_with_limits(limits)
        };
        let result = ZipExtractor::new()
            .extract_content(&root, "application/zip", &config)
            .await
            .unwrap();
        if limit == exact_limit {
            assert_eq!(budget.remaining_decompressed(), 0);
            assert_eq!(result.children.as_ref().unwrap()[0].path, "nested.tar");
        } else {
            assert_eq!(budget.remaining_decompressed(), limit - nested.len());
            assert!(result.children.is_none());
            assert!(recursion_warnings(&result).iter().any(|warning| {
                warning.contains("nested.tar") && warning.contains("total uncompressed size exceeds limit")
            }));
        }
    }
}

#[tokio::test]
async fn solid_7z_budget_counts_binary_and_text_members_once() {
    let bytes = include_bytes!("fixtures/two-members-solid.7z");
    let member_bytes = b"opaque\n".len() + b"hello\n".len();
    let limits = SecurityLimits {
        max_archive_size: member_bytes,
        ..Default::default()
    };
    let budget = Arc::new(ArchiveBudget::new(&limits));
    let config = ExtractionConfig {
        archive_budget: Some(Arc::clone(&budget)),
        ..config_with_limits(limits)
    };

    let result = SevenZExtractor::new()
        .extract_content(bytes, "application/x-7z-compressed", &config)
        .await
        .unwrap();

    assert_eq!(budget.remaining_decompressed(), 0);
    let Some(crate::types::FormatMetadata::Archive(metadata)) = &result.metadata.format else {
        panic!("Expected archive metadata");
    };
    assert_eq!(metadata.file_count, 2);
    assert_eq!(metadata.total_size, member_bytes as u64);
    let extracted =
        crate::extraction::derive::derive_extraction_result(result, true, crate::core::config::OutputFormat::Plain);
    assert!(extracted.content.contains("hello"));
}
