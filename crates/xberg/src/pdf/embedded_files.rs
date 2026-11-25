//! PDF embedded file (portfolio/attachment) extraction using lopdf.
//!
//! PDFs can contain file attachments via the `/Names` → `/EmbeddedFiles` name tree
//! in the document catalog. This module extracts those files, detects their MIME
//! type, and returns them as `ArchiveEntry` values for recursive processing.

#[cfg(feature = "tokio-runtime")]
use crate::types::{ArchiveEntry, ProcessingWarning};
#[cfg(feature = "tokio-runtime")]
use lopdf::{Document, Object};
#[cfg(feature = "tokio-runtime")]
use std::borrow::Cow;

/// Maximum recursion depth when walking the `/EmbeddedFiles` name tree's `/Kids`
/// chain. Mirrors `bookmarks.rs`'s `MAX_NAME_TREE_DEPTH`: without a depth cap, a
/// name-tree node whose `/Kids` array references one of its own ancestors (or
/// itself) makes `collect_from_name_tree` recurse without ever returning, which
/// overflows the stack and aborts the whole process — unlike an ordinary panic,
/// a stack overflow cannot be caught with `catch_unwind`.
#[cfg(feature = "tokio-runtime")]
const MAX_EMBEDDED_NAME_TREE_DEPTH: usize = 50;

/// Maximum number of name-tree nodes visited in total, independent of depth, so
/// a wide (rather than deep) malformed tree cannot force unbounded work either.
#[cfg(feature = "tokio-runtime")]
const MAX_EMBEDDED_NAME_TREE_NODES: usize = 500;

/// Embedded file descriptor extracted from the PDF name tree.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmbeddedFile {
    /// The filename as stored in the PDF name tree.
    pub name: String,
    /// Raw file bytes from the embedded stream (already decompressed by lopdf).
    pub data: Vec<u8>,
    /// Compressed byte count of the original stream (before decompression).
    ///
    /// Used by callers to compute the decompression ratio and detect zip-bomb-style
    /// attacks that embed a tiny compressed stream expanding to gigabytes of data.
    pub compressed_size: usize,
    /// MIME type if specified in the filespec, otherwise `None`.
    pub mime_type: Option<String>,
}

/// Extract embedded file descriptors from a PDF document loaded via lopdf.
///
/// Walks the `/Names` → `/EmbeddedFiles` name tree in the catalog.
/// Returns an empty `Vec` if the document has no embedded files.
#[cfg(feature = "tokio-runtime")]
pub(crate) fn extract_embedded_files(document: &Document) -> Vec<EmbeddedFile> {
    let mut files = Vec::new();
    let Some(ef_dict) = embedded_files_name_tree(document) else {
        return files;
    };

    let mut visited_nodes = 0usize;
    collect_from_name_tree(document, &ef_dict, &mut files, &mut visited_nodes, 0);

    files
}

/// Whether `document` has a catalog `/Names` → `/EmbeddedFiles` name tree.
///
/// [`extract_embedded_files`] returns nothing without one, so a caller already holding a
/// lopdf parse of the bytes can skip [`extract_and_process_embedded_files`], which would
/// parse them again only to find no tree.
#[cfg(feature = "tokio-runtime")]
pub(crate) fn has_embedded_files_name_tree(document: &Document) -> bool {
    embedded_files_name_tree(document).is_some()
}

#[cfg(feature = "tokio-runtime")]
fn embedded_files_name_tree(document: &Document) -> Option<lopdf::Dictionary> {
    let catalog = document.catalog().ok()?;
    let names_obj = resolve_object(document, catalog.get(b"Names").ok()?);
    let Some(Object::Dictionary(names_dict)) = names_obj else {
        return None;
    };
    match resolve_object(document, names_dict.get(b"EmbeddedFiles").ok()?) {
        Some(Object::Dictionary(dict)) => Some(dict),
        _ => None,
    }
}

/// Recursively collect embedded files from a PDF name tree node.
///
/// Traversal is bounded by both `depth` and `visited_nodes` (see
/// `MAX_EMBEDDED_NAME_TREE_DEPTH`/`MAX_EMBEDDED_NAME_TREE_NODES`) so a
/// malformed or cyclic `/Kids` chain cannot recurse indefinitely.
#[cfg(feature = "tokio-runtime")]
fn collect_from_name_tree(
    document: &Document,
    dict: &lopdf::Dictionary,
    files: &mut Vec<EmbeddedFile>,
    visited_nodes: &mut usize,
    depth: usize,
) {
    if depth > MAX_EMBEDDED_NAME_TREE_DEPTH || *visited_nodes >= MAX_EMBEDDED_NAME_TREE_NODES {
        return;
    }
    *visited_nodes += 1;

    if let Ok(Object::Array(names_arr)) = dict.get(b"Names") {
        let mut i = 0;
        while i + 1 < names_arr.len() {
            let name = match &names_arr[i] {
                Object::String(bytes, _) => String::from_utf8_lossy(bytes).into_owned(),
                _ => {
                    i += 2;
                    continue;
                }
            };

            let filespec = resolve_object(document, &names_arr[i + 1]);
            if let Some(Object::Dictionary(fs_dict)) = filespec
                && let Some(ef) = extract_file_from_filespec(document, &name, &fs_dict)
            {
                files.push(ef);
            }

            i += 2;
        }
    }

    if let Ok(Object::Array(kids)) = dict.get(b"Kids") {
        for kid in kids {
            if *visited_nodes >= MAX_EMBEDDED_NAME_TREE_NODES {
                break;
            }
            let kid_obj = resolve_object(document, kid);
            if let Some(Object::Dictionary(kid_dict)) = kid_obj {
                collect_from_name_tree(document, &kid_dict, files, visited_nodes, depth + 1);
            }
        }
    }
}

/// Extract an embedded file from a filespec dictionary.
///
/// The filespec has:
/// - `/UF` or `/F`: display filename
/// - `/EF` → `/F`: reference to the embedded file stream
/// - `/AFRelationship`: optional relationship type
#[cfg(feature = "tokio-runtime")]
fn extract_file_from_filespec(
    document: &Document,
    tree_name: &str,
    fs_dict: &lopdf::Dictionary,
) -> Option<EmbeddedFile> {
    let display_name = fs_dict
        .get(b"UF")
        .or_else(|_| fs_dict.get(b"F"))
        .ok()
        .and_then(|obj| match obj {
            Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).into_owned()),
            _ => None,
        })
        .unwrap_or_else(|| tree_name.to_string());

    let ef_obj = resolve_object(document, fs_dict.get(b"EF").ok()?)?;
    let ef_dict = match ef_obj {
        Object::Dictionary(d) => d,
        _ => return None,
    };

    let stream_obj = ef_dict.get(b"F").or_else(|_| ef_dict.get(b"UF")).ok()?;
    let stream_id = stream_obj.as_reference().ok()?;

    let stream = match document.get_object(stream_id) {
        Ok(Object::Stream(s)) => s,
        _ => return None,
    };

    let compressed_size = stream.content.len();

    let data = stream.decompressed_content().unwrap_or_else(|_| stream.content.clone());

    let mime_type = stream
        .dict
        .get(b"Subtype")
        .ok()
        .and_then(|obj| obj.as_name().ok())
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .or_else(|| {
            std::path::Path::new(&display_name)
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(|ext| mime_guess::from_ext(ext).first())
                .map(|m| m.to_string())
        });

    Some(EmbeddedFile {
        name: display_name,
        data,
        compressed_size,
        mime_type,
    })
}

/// Resolve a PDF object through references.
#[cfg(feature = "tokio-runtime")]
fn resolve_object<'a>(document: &'a Document, obj: &'a Object) -> Option<Object> {
    match obj {
        Object::Reference(id) => document.get_object(*id).ok().cloned(),
        other => Some(other.clone()),
    }
}

/// Check an embedded file against the decompression-ratio and size caps.
///
/// Returns the warning to record when the file must be skipped, or `None` when
/// it is eligible for extraction.
#[cfg(feature = "tokio-runtime")]
fn embedded_file_skip_warning(
    file: &EmbeddedFile,
    max_ratio: usize,
    max_embedded_file_bytes: Option<u64>,
) -> Option<ProcessingWarning> {
    if file.compressed_size > 0 {
        let ratio = file.data.len() as f64 / file.compressed_size as f64;
        if ratio > max_ratio as f64 {
            return Some(ProcessingWarning {
                source: Cow::Borrowed("pdf_embedded_files"),
                message: Cow::Owned(format!(
                    "Skipped embedded file '{}': decompression ratio {:.0}x exceeds limit {}x \
                     (compressed {} B → decompressed {} B)",
                    file.name,
                    ratio,
                    max_ratio,
                    file.compressed_size,
                    file.data.len(),
                )),
            });
        }
    }

    if max_embedded_file_bytes.is_some_and(|cap| file.data.len() as u64 > cap) {
        let cap = max_embedded_file_bytes.unwrap_or(0);
        return Some(ProcessingWarning {
            source: Cow::Borrowed("pdf_embedded_files"),
            message: Cow::Owned(format!(
                "Skipped embedded file '{}': size {} bytes exceeds cap {} bytes",
                file.name,
                file.data.len(),
                cap,
            )),
        });
    }

    None
}

/// Extract embedded files from PDF bytes and recursively process them.
///
/// Returns `(children, warnings)`. The children are `ArchiveEntry` values
/// suitable for attaching to `InternalDocument.children`.
#[cfg(feature = "tokio-runtime")]
pub(crate) async fn extract_and_process_embedded_files(
    pdf_bytes: &[u8],
    config: &crate::core::config::ExtractionConfig,
) -> (Vec<ArchiveEntry>, Vec<ProcessingWarning>) {
    let mut children = Vec::new();
    let mut warnings = Vec::new();

    let document = match Document::load_mem(pdf_bytes) {
        Ok(doc) => doc,
        Err(_) => return (children, warnings),
    };

    let embedded = extract_embedded_files(&document);
    if embedded.is_empty() {
        return (children, warnings);
    }

    if config.max_archive_depth == 0 {
        return (children, warnings);
    }

    let mut child_config = config.clone();
    child_config.max_archive_depth = config.max_archive_depth.saturating_sub(1);

    let max_ratio = config
        .security_limits
        .as_ref()
        .map(|sl| sl.max_compression_ratio)
        .unwrap_or(100);

    for file in embedded {
        let skipped = embedded_file_skip_warning(&file, max_ratio, config.max_embedded_file_bytes);
        if let Some(warning) = skipped {
            warnings.push(warning);
            continue;
        }

        let mime = file.mime_type.unwrap_or_else(|| {
            std::path::Path::new(&file.name)
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(|ext| mime_guess::from_ext(ext).first())
                .map(|m| m.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string())
        });

        if mime == "application/octet-stream" {
            continue;
        }

        match crate::core::extractor::extract_bytes(&file.data, &mime, &child_config).await {
            Ok(result) => {
                children.push(ArchiveEntry {
                    path: file.name,
                    mime_type: mime,
                    result: Box::new(result),
                });
            }
            Err(e) => {
                warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("pdf_embedded_files"),
                    message: Cow::Owned(format!("Failed to extract embedded '{}': {}", file.name, e)),
                });
            }
        }
    }

    (children, warnings)
}

#[cfg(all(test, feature = "tokio-runtime"))]
mod tests {
    use super::*;
    use crate::core::config::ExtractionConfig;
    use crate::extractors::security::SecurityLimits;
    use lopdf::dictionary;

    /// Build a filespec (`/EF` -> `/F` -> embedded-file stream) object graph and
    /// return the filespec's own object id, ready to be referenced from a `/Names`
    /// array entry.
    fn build_filespec(doc: &mut Document, name: &str, payload: &[u8]) -> lopdf::ObjectId {
        use lopdf::{Dictionary, Stream, StringFormat};

        let mut ef_stream_dict = Dictionary::new();
        ef_stream_dict.set("Type", Object::Name(b"EmbeddedFile".to_vec()));
        ef_stream_dict.set("Length", Object::Integer(payload.len() as i64));
        let ef_stream = Stream::new(ef_stream_dict, payload.to_vec());
        let ef_stream_id = doc.add_object(ef_stream);

        let mut ef_dict = Dictionary::new();
        ef_dict.set("F", Object::Reference(ef_stream_id));
        let ef_dict_id = doc.add_object(ef_dict);

        let mut fs_dict = Dictionary::new();
        fs_dict.set("F", Object::String(name.as_bytes().to_vec(), StringFormat::Literal));
        fs_dict.set("EF", Object::Reference(ef_dict_id));
        doc.add_object(fs_dict)
    }

    /// Wire a `/EmbeddedFiles` name-tree root into the catalog of `doc`.
    fn attach_embedded_files_root(doc: &mut Document, root: lopdf::ObjectId) {
        let mut names_dict = lopdf::Dictionary::new();
        names_dict.set("EmbeddedFiles", Object::Reference(root));
        let names_id = doc.add_object(names_dict);
        let mut catalog_dict = lopdf::Dictionary::new();
        catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog_dict.set("Names", Object::Reference(names_id));
        let catalog_id = doc.add_object(catalog_dict);
        doc.trailer.set("Root", Object::Reference(catalog_id));
    }

    /// A `/Kids` chain nested deeper than `MAX_EMBEDDED_NAME_TREE_DEPTH` must not
    /// let a leaf named far down the chain surface.
    ///
    /// Before `MAX_EMBEDDED_NAME_TREE_DEPTH`/`visited_nodes` were added,
    /// `collect_from_name_tree` had no bound on `/Kids` recursion at all: a name
    /// tree node whose `/Kids` array references one of its own ancestors (a
    /// cycle a malformed or malicious PDF can trivially construct) makes the
    /// function recurse forever, overflowing the stack and aborting the whole
    /// process. A stack overflow cannot be caught with `catch_unwind`, so
    /// reproducing the crash directly in a test would take the entire test
    /// binary down with it; instead this proves the bound that prevents it: a
    /// legal-looking chain nested past the depth cap must not be walked to its
    /// end, which is exactly the condition that made the cycle case unbounded.
    #[test]
    fn deeply_nested_kids_chain_beyond_the_depth_cap_is_not_walked() {
        let mut doc = Document::with_version("1.7");

        let leaf_fs_id = build_filespec(&mut doc, "after-budget.txt", b"unreachable");
        let mut current = doc.add_object(dictionary! {
            "Names" => vec![Object::string_literal("after-budget.txt"), Object::Reference(leaf_fs_id)],
        });
        for _ in 0..(MAX_EMBEDDED_NAME_TREE_DEPTH + 5) {
            current = doc.add_object(dictionary! { "Kids" => vec![Object::Reference(current)] });
        }
        attach_embedded_files_root(&mut doc, current);

        let files = extract_embedded_files(&doc);
        assert!(
            files.iter().all(|f| f.name != "after-budget.txt"),
            "leaf beyond the {MAX_EMBEDDED_NAME_TREE_DEPTH}-deep traversal cap must not be reached, got {:?}",
            files.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
    }

    /// Positive control: an ordinary, shallow `/Kids` chain (well within the
    /// depth cap) must still surface its embedded file exactly as before the
    /// cap was added — this is a crash fix, not a tightening of what resolves.
    #[test]
    fn shallow_kids_chain_still_resolves_embedded_file() {
        let mut doc = Document::with_version("1.7");
        let fs_id = build_filespec(&mut doc, "report.txt", b"hello");
        let leaf = doc.add_object(dictionary! {
            "Names" => vec![Object::string_literal("report.txt"), Object::Reference(fs_id)],
        });
        let mid = doc.add_object(dictionary! { "Kids" => vec![Object::Reference(leaf)] });
        let root = doc.add_object(dictionary! { "Kids" => vec![Object::Reference(mid)] });
        attach_embedded_files_root(&mut doc, root);

        let files = extract_embedded_files(&doc);
        assert_eq!(
            files.len(),
            1,
            "expected the shallow chain's leaf file to be found, got {:?}",
            files.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        assert_eq!(files[0].name, "report.txt");
        assert_eq!(files[0].data.as_slice(), b"hello".as_slice());
    }

    #[test]
    fn test_extract_embedded_files_no_names() {
        let doc = Document::with_version("1.5");
        let files = extract_embedded_files(&doc);
        assert!(files.is_empty());
        assert!(!has_embedded_files_name_tree(&doc));
    }

    /// The predicate that lets callers skip the attachment pass must hold exactly when the
    /// catalog reaches an `/EmbeddedFiles` dictionary, the precondition of any extracted file.
    #[test]
    fn embedded_files_name_tree_predicate_tracks_the_catalog_tree() {
        let mut with_tree = Document::with_version("1.7");
        let fs_id = build_filespec(&mut with_tree, "report.txt", b"hello");
        let root = with_tree.add_object(dictionary! {
            "Names" => vec![Object::string_literal("report.txt"), Object::Reference(fs_id)],
        });
        attach_embedded_files_root(&mut with_tree, root);
        assert!(has_embedded_files_name_tree(&with_tree));
        assert_eq!(extract_embedded_files(&with_tree).len(), 1);

        let mut dests_only = Document::with_version("1.7");
        let dests = dests_only.add_object(dictionary! { "Names" => Vec::<Object>::new() });
        let names = dests_only.add_object(dictionary! { "Dests" => Object::Reference(dests) });
        let catalog = dests_only.add_object(dictionary! { "Type" => "Catalog", "Names" => Object::Reference(names) });
        dests_only.trailer.set("Root", Object::Reference(catalog));
        assert!(!has_embedded_files_name_tree(&dests_only));
        assert!(extract_embedded_files(&dests_only).is_empty());

        let mut dangling = Document::with_version("1.7");
        let catalog = dangling.add_object(dictionary! {
            "Type" => "Catalog",
            "Names" => dictionary! { "EmbeddedFiles" => Object::Reference((999, 0)) },
        });
        dangling.trailer.set("Root", Object::Reference(catalog));
        assert!(!has_embedded_files_name_tree(&dangling));
        assert!(extract_embedded_files(&dangling).is_empty());
    }

    /// Build a synthetic `EmbeddedFile` with a given compressed and decompressed size
    /// and check whether the ratio guard would fire.
    fn ratio_would_skip(compressed: usize, decompressed: usize, max_ratio: usize) -> bool {
        if compressed == 0 {
            return false;
        }
        let ratio = decompressed as f64 / compressed as f64;
        ratio > max_ratio as f64
    }

    #[test]
    fn test_ratio_guard_fires_above_limit() {
        assert!(ratio_would_skip(1, 1001, 1000));
    }

    #[test]
    fn test_ratio_guard_passes_at_exact_limit() {
        assert!(!ratio_would_skip(1, 1000, 1000));
    }

    #[test]
    fn test_ratio_guard_passes_when_compressed_zero() {
        assert!(!ratio_would_skip(0, 50_000, 100));
    }

    #[test]
    fn test_ratio_guard_passes_below_limit() {
        assert!(!ratio_would_skip(100, 9_999, 100));
        assert!(ratio_would_skip(100, 10_001, 100));
    }

    #[tokio::test]
    async fn test_no_embedded_files_returns_empty() {
        let doc_bytes = {
            let mut doc = Document::with_version("1.5");
            let mut buf = Vec::new();
            doc.save_to(&mut buf).unwrap();
            buf
        };
        let config = ExtractionConfig::default();
        let (children, warnings) = extract_and_process_embedded_files(&doc_bytes, &config).await;
        assert!(children.is_empty());
        assert!(warnings.is_empty());
    }

    #[tokio::test]
    async fn test_embedded_file_over_size_cap_skipped_with_warning() {
        use lopdf::{Dictionary, Object, Stream};

        let mut doc = Document::with_version("1.5");

        let payload: Vec<u8> = vec![0u8; 200];

        let mut ef_stream_dict = Dictionary::new();
        ef_stream_dict.set("Type", Object::Name(b"EmbeddedFile".to_vec()));
        ef_stream_dict.set("Length", Object::Integer(payload.len() as i64));
        let ef_stream = Stream::new(ef_stream_dict, payload.clone());
        let ef_stream_id = doc.add_object(ef_stream);

        let mut ef_dict = Dictionary::new();
        ef_dict.set("F", Object::Reference(ef_stream_id));
        let ef_dict_id = doc.add_object(ef_dict);

        let mut fs_dict = Dictionary::new();
        fs_dict.set("Type", Object::Name(b"Filespec".to_vec()));
        fs_dict.set("F", Object::String(b"test.txt".to_vec(), lopdf::StringFormat::Literal));
        fs_dict.set("EF", Object::Reference(ef_dict_id));
        let fs_dict_id = doc.add_object(fs_dict);

        let names_arr = Object::Array(vec![
            Object::String(b"test.txt".to_vec(), lopdf::StringFormat::Literal),
            Object::Reference(fs_dict_id),
        ]);
        let mut ef_names_dict = Dictionary::new();
        ef_names_dict.set("Names", names_arr);
        let ef_names_id = doc.add_object(ef_names_dict);

        let mut names_dict = Dictionary::new();
        names_dict.set("EmbeddedFiles", Object::Reference(ef_names_id));
        let names_id = doc.add_object(names_dict);

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Kids", Object::Array(vec![]));
        pages_dict.set("Count", Object::Integer(0));
        let pages_id = doc.add_object(pages_dict);

        let mut catalog_dict = Dictionary::new();
        catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog_dict.set("Pages", Object::Reference(pages_id));
        catalog_dict.set("Names", Object::Reference(names_id));
        let catalog_id = doc.add_object(catalog_dict);
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();

        let config = ExtractionConfig {
            max_embedded_file_bytes: Some(10),
            ..Default::default()
        };
        let (_children, warnings) = extract_and_process_embedded_files(&buf, &config).await;

        let cap_warnings: Vec<_> = warnings.iter().filter(|w| w.message.contains("exceeds cap")).collect();
        assert_eq!(
            cap_warnings.len(),
            1,
            "expected one size-cap warning, got: {:?}",
            warnings
        );
        assert!(
            cap_warnings[0].message.contains("test.txt"),
            "warning must name the file: {}",
            cap_warnings[0].message
        );
    }

    #[tokio::test]
    async fn test_embedded_file_ratio_exceeded_skipped_with_warning() {
        use lopdf::{Dictionary, Object, Stream};

        let mut doc = Document::with_version("1.5");
        let payload: Vec<u8> = b"Hello".to_vec();
        let mut ef_stream_dict = Dictionary::new();
        ef_stream_dict.set("Type", Object::Name(b"EmbeddedFile".to_vec()));
        ef_stream_dict.set("Length", Object::Integer(payload.len() as i64));
        let ef_stream = Stream::new(ef_stream_dict, payload.clone());
        let ef_stream_id = doc.add_object(ef_stream);

        let mut ef_dict = Dictionary::new();
        ef_dict.set("F", Object::Reference(ef_stream_id));
        let ef_dict_id = doc.add_object(ef_dict);

        let mut fs_dict = Dictionary::new();
        fs_dict.set("F", Object::String(b"note.txt".to_vec(), lopdf::StringFormat::Literal));
        fs_dict.set("EF", Object::Reference(ef_dict_id));
        let fs_dict_id = doc.add_object(fs_dict);

        let names_arr = Object::Array(vec![
            Object::String(b"note.txt".to_vec(), lopdf::StringFormat::Literal),
            Object::Reference(fs_dict_id),
        ]);
        let mut ef_names_dict = Dictionary::new();
        ef_names_dict.set("Names", names_arr);
        let ef_names_id = doc.add_object(ef_names_dict);

        let mut names_dict = Dictionary::new();
        names_dict.set("EmbeddedFiles", Object::Reference(ef_names_id));
        let names_id = doc.add_object(names_dict);

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Kids", Object::Array(vec![]));
        pages_dict.set("Count", Object::Integer(0));
        let pages_id = doc.add_object(pages_dict);

        let mut catalog_dict = Dictionary::new();
        catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog_dict.set("Pages", Object::Reference(pages_id));
        catalog_dict.set("Names", Object::Reference(names_id));
        let catalog_id = doc.add_object(catalog_dict);
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buf = Vec::new();
        doc.save_to(&mut buf).unwrap();

        let config = ExtractionConfig {
            security_limits: Some(SecurityLimits {
                max_compression_ratio: 1,
                ..SecurityLimits::default()
            }),
            ..Default::default()
        };
        let (_children, warnings) = extract_and_process_embedded_files(&buf, &config).await;
        let ratio_warnings: Vec<_> = warnings.iter().filter(|w| w.message.contains("ratio")).collect();
        assert!(
            ratio_warnings.is_empty(),
            "stored-mode stream must not trigger ratio guard: {:?}",
            ratio_warnings
        );
    }
}
