//! Cross-reference table reconstruction for damaged PDFs.
//!
//! When the xref table is corrupted, missing, or incomplete, this module
//! provides functionality to reconstruct it by scanning the entire PDF file
//! for object markers.
//!
//! This is a fallback mechanism used only when standard xref parsing fails.

use crate::error::{Error, Result};
use crate::object::{Object, ObjectRef};
use crate::parser::parse_object;
use crate::xref::{CrossRefTable, XRefEntry};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::sync::LazyLock;

static RE_OBJ_PATTERN: LazyLock<regex::bytes::Regex> =
    LazyLock::new(|| regex::bytes::Regex::new(r"(\d+)\s+(\d+)\s+obj").unwrap());
static RE_TRAILER: LazyLock<regex::bytes::Regex> = LazyLock::new(|| regex::bytes::Regex::new(r"trailer\s*<<").unwrap());

/// Bytes parsed after a `trailer` keyword. A real trailer dictionary is a few
/// hundred bytes; without a bound, every `trailer<<` match in a hostile file
/// re-parses to end of file.
const MAX_TRAILER_WINDOW: usize = 64 * 1024;

fn trace_trailer_parse_failure(error: &nom::Err<nom::error::Error<&[u8]>>, input_len: usize, input_offset: usize) {
    match error {
        nom::Err::Error(parse_error) | nom::Err::Failure(parse_error) => {
            let error_offset = input_offset + input_len.saturating_sub(parse_error.input.len());
            tracing::warn!(
                error_offset,
                parser_error_kind = ?parse_error.code,
                "failed to parse trailer dictionary"
            );
        }
        nom::Err::Incomplete(_) => {
            tracing::warn!(
                error_offset = input_offset + input_len,
                parser_error_kind = "incomplete",
                "failed to parse trailer dictionary"
            );
        }
    }
}

// SPEC COMPLIANCE FIX: Validate that this is actually an object header
// PDF Spec: ISO 32000-1:2008, Section 7.5.4 - Cross-Reference Table
//
// Previous implementation would add ANY "N G obj" pattern to the xref table,
// even if it appeared inside strings, comments, or corrupted data.
//
// This creates security risks:
// 1. False positives can point to invalid object locations
// 2. Can cause crashes when trying to parse non-object data as objects
// 3. Malicious PDFs can craft fake object headers to confuse parsers
//
// Correct behavior: Validate that the pattern is followed by valid object syntax ~keep
/// Whether the bytes following one `N G obj` regex match actually look like
/// the start of a PDF object, so a false positive inside a string/comment/
/// corrupted data is never added to the xref table. Split out of
/// [`reconstruct_xref`] to keep it short.
fn object_header_is_valid(contents: &[u8], offset: u64, match_byte_len: usize, obj_num: u32, gen_num: u16) -> bool {
    let validation_start = offset + match_byte_len as u64;
    if validation_start >= contents.len() as u64 {
        return true;
    }
    let remaining = &contents[validation_start as usize..];

    let mut i = 0;
    while i < remaining.len() && remaining[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= remaining.len() {
        return true;
    }

    let next_byte = remaining[i];

    // Valid object should start with:
    // - << (dictionary)
    // - [ (array)
    // - < (hex string or dict - ambiguous at this point)
    // - ( (literal string)
    // - / (name)
    // - t, f, n (true, false, null)
    // - digit or - (number) ~keep
    let is_valid_object_start =
        matches!(next_byte, b'<' | b'[' | b'(' | b'/' | b't' | b'f' | b'n' | b'-') || next_byte.is_ascii_digit();

    if !is_valid_object_start {
        tracing::trace!(
            offset,
            next_byte = format_args!(
                "0x{:02x} '{}'",
                next_byte,
                if next_byte.is_ascii_graphic() {
                    next_byte as char
                } else {
                    '?'
                }
            ),
            "skipping false positive object header"
        );
        return false;
    }

    tracing::trace!(object_id = obj_num, generation = gen_num, offset, "validated object");
    true
}

/// Parse and validate one `N G obj` regex capture into an xref entry, or
/// `None` if the capture is malformed or fails [`object_header_is_valid`].
/// Split out of [`reconstruct_xref`] to keep it short.
fn parse_object_candidate(contents: &[u8], capture: &regex::bytes::Captures) -> Option<(u32, XRefEntry)> {
    let full_match = capture.get(0)?;
    let obj_num_bytes = capture.get(1)?.as_bytes();
    let gen_num_bytes = capture.get(2)?.as_bytes();

    let obj_num: u32 = match std::str::from_utf8(obj_num_bytes).ok().and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => {
            tracing::warn!(offset = full_match.start(), "failed to parse object number");
            return None;
        }
    };

    let gen_num: u16 = match std::str::from_utf8(gen_num_bytes).ok().and_then(|s| s.parse().ok()) {
        Some(n) => n,
        None => {
            tracing::warn!(offset = full_match.start(), "failed to parse generation number");
            return None;
        }
    };

    let offset = full_match.start() as u64;
    if !object_header_is_valid(contents, offset, full_match.as_bytes().len(), obj_num, gen_num) {
        return None;
    }

    Some((obj_num, XRefEntry::uncompressed(offset, gen_num)))
}

/// Reconstruct the cross-reference table by scanning the entire PDF file.
///
/// This function scans for "N G obj" patterns throughout the file and builds
/// an xref table from the discovered objects. It also attempts to find the
/// trailer dictionary and identify the catalog.
///
/// # Performance
///
/// For small to medium files (<10 MB), the entire file is read into memory.
/// For larger files, this could be optimized to scan in chunks, but that's
/// deferred until needed.
///
/// # Returns
///
/// The reconstructed cross-reference table, the trailer, and any SYNTHETIC
/// objects the caller must inject (they have no byte offset in the file).
/// Synthetic objects appear only when the file's own Catalog / page-tree root did
/// not survive and one had to be rebuilt from the orphaned pages; the vector is
/// empty in every ordinary reconstruction.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - No objects are found during scanning
/// - No catalog, page-tree root, or page object survived to rebuild from
///
/// # Example
///
/// ```no_run
/// # use std::fs::File;
/// # use std::io::BufReader;
/// # use xberg_native_pdf::xref_reconstruction::reconstruct_xref;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let file = File::open("damaged.pdf")?;
/// let mut reader = BufReader::new(file);
/// let (xref, trailer, _synthetic) = reconstruct_xref(&mut reader)?;
/// println!("Reconstructed {} objects", xref.len());
/// # Ok(())
/// # }
/// ```
pub fn reconstruct_xref<R: Read + Seek>(reader: &mut R) -> Result<(CrossRefTable, Object, Vec<(ObjectRef, Object)>)> {
    tracing::warn!("xref table is corrupted or missing; reconstructing by scanning file");

    reader.seek(SeekFrom::Start(0))?;
    let mut contents = Vec::new();
    reader.read_to_end(&mut contents)?;

    tracing::debug!(bytes = contents.len(), "file size");

    let mut xref = CrossRefTable::new();
    let mut objects_found = 0;

    for capture in RE_OBJ_PATTERN.captures_iter(&contents) {
        if let Some((obj_num, entry)) = parse_object_candidate(&contents, &capture) {
            xref.add_entry(obj_num, entry);
            objects_found += 1;
        }
    }

    tracing::info!(count = objects_found, "reconstructed xref");

    if objects_found == 0 {
        return Err(Error::InvalidPdf(
            "No objects found during xref reconstruction".to_string(),
        ));
    }

    let (trailer, synthetic) = find_trailer(&contents, reader, &xref)?;

    Ok((xref, trailer, synthetic))
}

/// Parse one `trailer` keyword match: if it parses to a /Root-bearing
/// dictionary, record it as the new `best_trailer`; otherwise salvage any
/// /Encrypt, /ID, /Info entries it carries. Split out of [`find_trailer`] to
/// keep it short.
fn process_trailer_match(
    contents: &[u8],
    mat: regex::bytes::Match,
    best_trailer: &mut Option<(Object, usize)>,
    salvaged: &mut HashMap<String, (Object, usize)>,
) {
    let trailer_start = mat.start();
    tracing::debug!(offset = trailer_start, "found trailer keyword");

    let trailer_keyword_end = trailer_start + 7; // len("trailer") ~keep
    let window_end = contents.len().min(trailer_keyword_end + MAX_TRAILER_WINDOW);
    let input = &contents[trailer_keyword_end..window_end];
    let (rest, obj) = match parse_object(input) {
        Ok(parsed) => parsed,
        Err(e) => {
            trace_trailer_parse_failure(&e, input.len(), trailer_keyword_end);
            return;
        }
    };

    // The lenient parser treats the window edge as end of input, so a dictionary cut off
    // there parses "successfully" without its closing `>>`. ~keep
    if window_end < contents.len() && !input[..input.len() - rest.len()].trim_ascii_end().ends_with(b">>") {
        tracing::debug!(
            offset = trailer_start,
            "trailer dictionary exceeds the parse window; skipping"
        );
        return;
    }

    // Only accept a parsed trailer that actually carries /Root.
    // A Linearized file's sparse end-of-file trailer legitimately
    // omits /Root — the Catalog is reachable via the linearization
    // parameters / first xref chain, not the trailing trailer
    // (issue #509). Accepting a /Root-less trailer here would
    // short-circuit Catalog discovery and fail downstream with
    // "Trailer missing /Root entry". The *last* /Root-bearing
    // trailer still wins for /Root itself. ~keep
    if obj.as_dict().is_some_and(|d| d.get("Root").is_some()) {
        *best_trailer = Some((obj, trailer_start));
        return;
    }

    if let Some(d) = obj.as_dict() {
        for key in ["Encrypt", "ID", "Info"] {
            if let Some(v) = d.get(key) {
                salvaged.insert(key.to_string(), (v.clone(), trailer_start));
            }
        }
    }
    tracing::debug!(
        offset = trailer_start,
        "parsed trailer has no /Root, skipping (Catalog located by object scan; \
         /Encrypt /ID /Info preserved)"
    );
}

/// Merge `salvaged` /Encrypt, /ID, /Info entries into a /Root-bearing
/// `trailer`, using most-recent-occurrence-wins (ISO 32000-1 §7.5.5): a
/// salvaged value overrides the trailer's only when it was parsed from a
/// *later* offset, and always fills a key the trailer lacks. Split out of
/// [`find_trailer`] to keep it short.
fn merge_salvaged_into_trailer(trailer: &mut Object, salvaged: &HashMap<String, (Object, usize)>, best_off: usize) {
    if salvaged.is_empty() {
        return;
    }
    let Object::Dictionary(d) = trailer else {
        return;
    };
    for (key, (value, off)) in salvaged {
        match d.get(key) {
            Some(_) if *off <= best_off => {} // existing is newer/equal ~keep
            _ => {
                d.insert(key.clone(), value.clone());
            }
        }
    }
}

/// Find and parse the trailer dictionary.
///
/// Searches for "trailer" keyword in the file and attempts to parse the
/// dictionary that follows. If not found, attempts to reconstruct a minimal
/// trailer by finding the catalog object.
fn find_trailer<R: Read + Seek>(
    contents: &[u8],
    reader: &mut R,
    xref: &CrossRefTable,
) -> Result<(Object, Vec<(ObjectRef, Object)>)> {
    tracing::debug!("searching for trailer dictionary");

    // Search for all "trailer" keywords and prefer the last valid one.
    // Per ISO 32000-1:2008 Section 7.5.5, the most recent trailer (from the
    // latest incremental update) takes precedence. Using the first trailer can
    // miss /Encrypt entries added in later revisions.
    // The chosen /Root-bearing trailer plus the byte offset it was parsed
    // from (RE_TRAILER yields matches in ascending file order, so a later
    // offset = a more recent incremental update). ~keep
    let mut best_trailer: Option<(Object, usize)> = None;
    // /Encrypt /ID /Info salvaged from /Root-less parsed trailers, each
    // tracked with the offset it came from. If no /Root-bearing trailer
    // exists and we synthesize a minimal one, an encrypted file's /Encrypt
    // (and /ID, used for the encryption key) would otherwise be lost, making
    // the document undecryptable. Per ISO 32000-1 §7.5.5 the most recent
    // occurrence wins — including over a /Root-bearing trailer that appears
    // earlier in the file. ~keep
    let mut salvaged: HashMap<String, (Object, usize)> = HashMap::new();
    for mat in RE_TRAILER.find_iter(contents) {
        process_trailer_match(contents, mat, &mut best_trailer, &mut salvaged);
    }
    if let Some((mut trailer, best_off)) = best_trailer {
        merge_salvaged_into_trailer(&mut trailer, &salvaged, best_off);
        tracing::info!("successfully parsed trailer dictionary (last /Root-bearing occurrence)");
        // A parsed /Root-bearing trailer needs no synthesis. ~keep
        return Ok((trailer, Vec::new()));
    }

    // No /Root-bearing trailer found — synthesize one by scanning objects
    // for /Type /Catalog (handles Linearized files whose only trailer is the
    // sparse, /Root-less end-of-file trailer). ~keep
    tracing::warn!("no /Root-bearing trailer found; reconstructing minimal trailer via Catalog scan");
    let salvaged_values: HashMap<String, Object> = salvaged.into_iter().map(|(k, (v, _))| (k, v)).collect();
    reconstruct_minimal_trailer(reader, xref, &salvaged_values)
}

/// Reconstruct a minimal trailer dictionary.
///
/// Scans objects to find the catalog (object with /Type /Catalog) and
/// creates a minimal trailer with the required entries, plus any
/// `salvaged` entries (/Encrypt, /ID, /Info) carried over from a
/// /Root-less parsed trailer so encrypted documents remain decryptable.
fn reconstruct_minimal_trailer<R: Read + Seek>(
    reader: &mut R,
    xref: &CrossRefTable,
    salvaged: &HashMap<String, Object>,
) -> Result<(Object, Vec<(ObjectRef, Object)>)> {
    tracing::debug!("scanning objects to find catalog");

    let mut catalog_ref = None;

    // Scan objects looking for the catalog. `all_object_numbers()` is
    // `HashMap`-backed, so iterating it directly is nondeterministic: a
    // bounded scan over an arbitrary subset can miss the Catalog on
    // different runs (even for a ~114-object Linearized file).
    // `smallest_object_numbers` is deterministic, visits low-numbered
    // objects first (where the Catalog conventionally lives), and bounds the
    // candidate set *before* sorting so a maliciously sparse/huge xref stays
    // O(n log MAX_SCAN) time / O(MAX_SCAN) memory. ~keep
    const MAX_SCAN: usize = 4096;
    let obj_nums = xref.smallest_object_numbers(MAX_SCAN);
    let mut checked = 0usize;
    for obj_num in obj_nums {
        if checked >= MAX_SCAN {
            break;
        }

        if let Some(entry) = xref.get(obj_num) {
            if !entry.in_use {
                continue;
            }
            checked += 1;

            match load_object_at_offset(reader, entry.offset) {
                Ok(obj) => {
                    if is_catalog(&obj) {
                        tracing::info!(object_id = obj_num, generation = entry.generation, "found catalog");
                        catalog_ref = Some((obj_num, entry.generation));
                        break;
                    }
                }
                Err(error) => {
                    tracing::trace!(
                        object_id = obj_num,
                        offset = entry.offset,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "failed to load object"
                    );
                    continue;
                }
            }
        }
    }

    // No Catalog anywhere in the surviving objects. This is the truncated-file
    // case (a web crawl capped mid-stream, an incremental update lost its tail):
    // the Catalog and page-tree root lived at the end and are gone, but the page
    // objects themselves survived. Rebuild a Catalog from those pages rather than
    // declaring the whole document a total loss. `synthetic` carries the objects
    // we invent - they have no byte offset, so the caller injects them into the
    // object cache. ~keep
    let (root_ref, synthetic) = match catalog_ref {
        Some((cat_num, cat_gen)) => (ObjectRef::new(cat_num, cat_gen), Vec::new()),
        None => synthesize_catalog_from_pages(reader, xref)?,
    };

    let mut trailer_dict = HashMap::new();
    trailer_dict.insert("Root".to_string(), Object::Reference(root_ref));
    trailer_dict.insert("Size".to_string(), Object::Integer(xref.len() as i64));

    // Carry over /Encrypt, /ID, /Info salvaged from a skipped /Root-less
    // trailer. Never clobber the Root/Size we just computed. ~keep
    for (key, value) in salvaged {
        if key != "Root" && key != "Size" {
            trailer_dict.insert(key.clone(), value.clone());
        }
    }

    Ok((Object::Dictionary(trailer_dict), synthetic))
}

/// Scan the surviving uncompressed objects for a `/Type /Pages` node
/// (preferring a genuine root — one with no `/Parent`) and every
/// `/Type /Page` object, in deterministic low-object-number-first order.
/// Split out of [`synthesize_catalog_from_pages`] to keep it short. ~keep
fn find_surviving_pages_and_page_objects<R: Read + Seek>(
    reader: &mut R,
    xref: &CrossRefTable,
) -> (Option<u32>, Option<u32>, Vec<u32>) {
    // Deterministic, low-first scan (the same bound the Catalog scan uses). ~keep
    const MAX_SCAN: usize = 4096;
    let obj_nums = xref.smallest_object_numbers(MAX_SCAN);

    let mut pages_root: Option<u32> = None;
    let mut pages_any: Option<u32> = None;
    let mut page_objs: Vec<u32> = Vec::new();
    for obj_num in obj_nums {
        let Some(entry) = xref.get(obj_num) else {
            continue;
        };
        if !entry.in_use {
            continue;
        }
        let Ok(obj) = load_object_at_offset(reader, entry.offset) else {
            continue;
        };
        let Some(dict) = obj.as_dict() else { continue };
        match dict.get("Type").and_then(|t| t.as_name()) {
            Some("Pages") => {
                pages_any.get_or_insert(obj_num);
                if dict.get("Parent").is_none() {
                    pages_root.get_or_insert(obj_num);
                }
            }
            Some("Page") => page_objs.push(obj_num),
            _ => {}
        }
    }
    (pages_root, pages_any, page_objs)
}

/// Build a flat single-level `/Pages` dictionary over `page_objs`, in object
/// order, with a fallback US Letter `/MediaBox` for pages that inherited
/// their size from a now-lost parent. Split out of
/// [`synthesize_catalog_from_pages`] to keep it short.
fn build_orphan_pages_dict(page_objs: &[u32]) -> Object {
    let kids: Vec<Object> = page_objs
        .iter()
        .map(|&n| Object::Reference(ObjectRef::new(n, 0)))
        .collect();
    let mut pages = HashMap::new();
    pages.insert("Type".to_string(), Object::Name("Pages".to_string()));
    pages.insert("Count".to_string(), Object::Integer(page_objs.len() as i64));
    pages.insert("Kids".to_string(), Object::Array(kids));
    // Fallback media box for any page that inherited its size from the lost parent. ~keep
    pages.insert(
        "MediaBox".to_string(),
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(612),
            Object::Integer(792),
        ]),
    );
    Object::Dictionary(pages)
}

/// Rebuild a Catalog (and, if needed, a page-tree root) from the surviving page
/// objects of a truncated file, returning the Root reference and the SYNTHETIC
/// objects to inject.
///
/// Two cases, in order of fidelity:
///  1. A `/Type /Pages` node survived (the page-tree root, or any internal node).
///     Prefer a root - a `/Pages` with no `/Parent` - and point a synthesized
///     Catalog at it, preserving the file's own tree and its inherited attributes.
///  2. No `/Pages` survived: collect every `/Type /Page` object and hang them off
///     a synthesized flat `/Pages` node, then a Catalog. Page order follows object
///     number (the conventional page order). The flat node carries a default
///     `/MediaBox` so a page that relied on inheritance from its lost parent still
///     has a media box to fall back to.
///
/// Returns `Error::InvalidPdf` only when NEITHER a `/Pages` nor any `/Type /Page`
/// survived - there is genuinely nothing to show.
fn synthesize_catalog_from_pages<R: Read + Seek>(
    reader: &mut R,
    xref: &CrossRefTable,
) -> Result<(ObjectRef, Vec<(ObjectRef, Object)>)> {
    // Free object numbers for the objects we invent: above every surviving one. ~keep
    let max_obj = xref.all_object_numbers().max().unwrap_or(0);
    let catalog_num = max_obj + 1;

    // Look for a surviving /Type /Pages node - prefer a genuine ROOT (no /Parent). ~keep
    let (pages_root, pages_any, mut page_objs) = find_surviving_pages_and_page_objects(reader, xref);

    // Case 1: a /Pages node survived - point a Catalog at the best one. ~keep
    if let Some(root) = pages_root.or(pages_any) {
        tracing::warn!(
            pages_object_id = root,
            "recovery: synthesizing Catalog over surviving /Pages object"
        );
        let catalog = catalog_dict(ObjectRef::new(root, 0));
        return Ok((
            ObjectRef::new(catalog_num, 0),
            vec![(ObjectRef::new(catalog_num, 0), catalog)],
        ));
    }

    // Case 2: no uncompressed page survived. Before giving up, look inside any
    // surviving object streams (/Type /ObjStm): PDF 1.5+ files routinely pack the
    // Catalog, page-tree nodes and page dictionaries into ObjStms, which the
    // offset scan above cannot see (it only finds `N G obj` markers). The objects
    // parsed out carry their real numbers, so injecting them lets their refs -
    // including /Contents streams, which live UNCOMPRESSED in the rebuilt xref -
    // resolve normally. ~keep
    if page_objs.is_empty() {
        if let Some(result) = recover_from_objstms(reader, xref, catalog_num) {
            return Ok(result);
        }
        return Err(Error::InvalidPdf(
            "Could not find catalog or any page in reconstructed xref".to_string(),
        ));
    }
    page_objs.sort_unstable();
    tracing::warn!(
        count = page_objs.len(),
        "recovery: synthesizing flat /Pages over orphan pages"
    );
    let pages_num = max_obj + 2;
    let catalog = catalog_dict(ObjectRef::new(pages_num, 0));
    Ok((
        ObjectRef::new(catalog_num, 0),
        vec![
            (ObjectRef::new(pages_num, 0), build_orphan_pages_dict(&page_objs)),
            (ObjectRef::new(catalog_num, 0), catalog),
        ],
    ))
}

/// A minimal `<< /Type /Catalog /Pages ref >>`.
fn catalog_dict(pages: ObjectRef) -> Object {
    let mut d = HashMap::new();
    d.insert("Type".to_string(), Object::Name("Catalog".to_string()));
    d.insert("Pages".to_string(), Object::Reference(pages));
    Object::Dictionary(d)
}

/// Classify one object stream member by its `/Type` (`Catalog`, `Pages`, or
/// `Page`), recording it into `catalog`/`pages_root`/`pages_any`/`page_objs`.
/// Split out of [`recover_from_objstms`] to keep it short.
fn classify_objstm_member(
    num: u32,
    obj: &Object,
    catalog: &mut Option<u32>,
    pages_root: &mut Option<u32>,
    pages_any: &mut Option<u32>,
    page_objs: &mut Vec<u32>,
) {
    match obj.as_dict().and_then(|d| d.get("Type")).and_then(|t| t.as_name()) {
        Some("Catalog") => {
            catalog.get_or_insert(num);
        }
        Some("Pages") => {
            pages_any.get_or_insert(num);
            if obj.as_dict().and_then(|d| d.get("Parent")).is_none() {
                pages_root.get_or_insert(num);
            }
        }
        Some("Page") => page_objs.push(num),
        _ => {}
    }
}

/// Decompress every surviving `/Type /ObjStm` and inject all objects it
/// contains (at their real numbers), classifying each by type along the way.
/// Split out of [`recover_from_objstms`] to keep it short.
fn scan_objstms_for_catalog_and_pages<R: Read + Seek>(
    reader: &mut R,
    xref: &CrossRefTable,
) -> (
    Vec<(ObjectRef, Object)>,
    Option<u32>,
    Option<u32>,
    Option<u32>,
    Vec<u32>,
) {
    const MAX_SCAN: usize = 4096;
    let mut injected: Vec<(ObjectRef, Object)> = Vec::new();
    let mut catalog: Option<u32> = None;
    let mut pages_root: Option<u32> = None;
    let mut pages_any: Option<u32> = None;
    let mut page_objs: Vec<u32> = Vec::new();

    for obj_num in xref.smallest_object_numbers(MAX_SCAN) {
        let Some(entry) = xref.get(obj_num) else {
            continue;
        };
        if !entry.in_use {
            continue;
        }
        let Ok(container) = load_object_at_offset(reader, entry.offset) else {
            continue;
        };
        // Only /Type /ObjStm streams carry other objects. ~keep
        let is_objstm = container
            .as_dict()
            .and_then(|d| d.get("Type"))
            .and_then(|t| t.as_name())
            == Some("ObjStm");
        if !is_objstm {
            continue;
        }
        let Ok(contained) = crate::objstm::parse_object_stream(&container) else {
            continue;
        };
        // `get_or_insert` below keeps the first candidate it meets, and
        // `parse_object_stream` returns a `HashMap` whose iteration order Rust
        // randomizes per instance — so the walk has to be ordered. ~keep
        let mut contained: Vec<(u32, Object)> = contained.into_iter().collect();
        contained.sort_by_key(|(num, _)| *num);
        for (num, obj) in contained {
            classify_objstm_member(num, &obj, &mut catalog, &mut pages_root, &mut pages_any, &mut page_objs);
            injected.push((ObjectRef::new(num, 0), obj));
        }
    }

    (injected, catalog, pages_root, pages_any, page_objs)
}

/// Build a flat single-level `/Pages` dictionary over `page_objs` (ObjStm
/// recovery variant — see [`build_orphan_pages_dict`] for the uncompressed
/// scan's equivalent). Split out of [`recover_from_objstms`] to keep it short.
fn build_objstm_pages_dict(page_objs: &[u32]) -> Object {
    let kids: Vec<Object> = page_objs
        .iter()
        .map(|&n| Object::Reference(ObjectRef::new(n, 0)))
        .collect();
    let mut pages = HashMap::new();
    pages.insert("Type".to_string(), Object::Name("Pages".to_string()));
    pages.insert("Count".to_string(), Object::Integer(page_objs.len() as i64));
    pages.insert("Kids".to_string(), Object::Array(kids));
    pages.insert(
        "MediaBox".to_string(),
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(612),
            Object::Integer(792),
        ]),
    );
    Object::Dictionary(pages)
}

/// Recover pages packed inside object streams when no uncompressed page survived.
///
/// Decompresses every surviving `/Type /ObjStm` and injects ALL objects it
/// contains (at their real numbers) so their cross-references resolve, then
/// anchors a Root: a real Catalog if one was packed in, otherwise a synthesized
/// Catalog over the ObjStm's own `/Pages` root or a flat `/Pages` of the packed
/// page dictionaries. `None` when no ObjStm yields a page.
fn recover_from_objstms<R: Read + Seek>(
    reader: &mut R,
    xref: &CrossRefTable,
    catalog_num: u32,
) -> Option<(ObjectRef, Vec<(ObjectRef, Object)>)> {
    let (mut injected, catalog, pages_root, pages_any, mut page_objs) =
        scan_objstms_for_catalog_and_pages(reader, xref);

    // A real Catalog was packed in - use it directly. ~keep
    if let Some(cat) = catalog {
        tracing::warn!(object_id = cat, "recovery: Catalog recovered from an object stream");
        return Some((ObjectRef::new(cat, 0), injected));
    }

    // Free object numbers for anything we synthesize must clear EVERY injected
    // number too, not just the uncompressed max (compressed objects can outrank
    // it), or a synthetic Catalog could shadow a real recovered object. ~keep
    let free_base = injected
        .iter()
        .map(|(r, _)| r.id)
        .max()
        .map_or(catalog_num, |m| m.max(catalog_num - 1) + 1);

    if let Some(root) = pages_root.or(pages_any) {
        injected.push((ObjectRef::new(free_base, 0), catalog_dict(ObjectRef::new(root, 0))));
        return Some((ObjectRef::new(free_base, 0), injected));
    }
    if page_objs.is_empty() {
        return None;
    }
    page_objs.sort_unstable();
    tracing::warn!(
        count = page_objs.len(),
        "recovery: synthesizing flat /Pages over ObjStm-packed pages"
    );
    let synth_catalog = free_base;
    let pages_num = free_base + 1;
    injected.push((ObjectRef::new(pages_num, 0), build_objstm_pages_dict(&page_objs)));
    injected.push((
        ObjectRef::new(synth_catalog, 0),
        catalog_dict(ObjectRef::new(pages_num, 0)),
    ));
    Some((ObjectRef::new(synth_catalog, 0), injected))
}

/// Load an object at a specific byte offset.
///
/// This is a standalone function that doesn't require the full PdfDocument
/// context, used during trailer reconstruction.
pub(crate) fn load_object_at_offset<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<Object> {
    reader.seek(SeekFrom::Start(offset))?;

    const MAX_OBJECT_SIZE: usize = 1024 * 1024; // 1MB ~keep

    // `read_until` buffers a whole line, so the reader itself must be bounded. ~keep
    let mut buf_reader = BufReader::new(reader.take(MAX_OBJECT_SIZE as u64 + 1));
    let mut content = Vec::new();

    // Read up to 1MB or until we find endobj
    // This is a conservative limit to avoid memory issues ~keep
    let mut bytes_read = 0;

    // The keyword has no line break, so only the line just read can newly contain it. ~keep
    const ENDOBJ: &[u8] = b"endobj";

    loop {
        let mut line = Vec::new();
        match buf_reader.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(n) => {
                content.extend_from_slice(&line);
                bytes_read += n;

                if bytes_read > MAX_OBJECT_SIZE {
                    return Err(Error::InvalidPdf("Object too large".to_string()));
                }

                if memchr::memmem::find(&line, ENDOBJ).is_some() {
                    break;
                }
            }
            Err(e) => return Err(Error::Io(e)),
        }
    }

    use crate::lexer::token;

    let input = &content[..];

    let (rest, _) = token(input).map_err(|e| Error::ParseError {
        offset: 0,
        reason: format!("failed to parse object number: {}", e),
    })?;

    let (rest, _) = token(rest).map_err(|e| Error::ParseError {
        offset: 0,
        reason: format!("failed to parse generation: {}", e),
    })?;

    let (rest, _) = token(rest).map_err(|e| Error::ParseError {
        offset: 0,
        reason: format!("failed to parse 'obj' keyword: {}", e),
    })?;

    let (_, obj) = parse_object(rest).map_err(|e| Error::ParseError {
        offset: 0,
        reason: format!("failed to parse object: {}", e),
    })?;

    Ok(obj)
}

/// Check if an object is the document catalog.
///
/// The catalog has /Type /Catalog in its dictionary.
fn is_catalog(obj: &Object) -> bool {
    if let Some(dict) = obj.as_dict()
        && let Some(type_obj) = dict.get("Type")
        && let Some(type_name) = type_obj.as_name()
    {
        return type_name == "Catalog";
    }
    false
}

/// Search for an object near a given offset.
///
/// When the reconstructed xref has slightly incorrect offsets, this function
/// searches within a ±1KB window to find the actual object.
pub fn search_nearby_for_object<R: Read + Seek>(reader: &mut R, obj_id: u32, approx_offset: u64) -> Result<Object> {
    tracing::warn!(
        object_id = obj_id,
        approx_offset,
        "xref offset is wrong; searching nearby for object"
    );

    // `approx_offset` comes from an xref entry offset, which under a
    // maliciously wide `/W` (e.g. `[1 8 2]`) can be any u64 value up to
    // `u64::MAX`. `start` was already guarded with `saturating_sub`; `end` was
    // not, so `approx_offset + search_range` could overflow. In debug builds
    // that panics; in release, wrapping addition/subtraction is modular, so
    // `end.wrapping_sub(start)` still equals `2 * search_range` as long as
    // `start` didn't itself saturate — the two wraps cancel and release never
    // misbehaves. Using `saturating_add`/`saturating_sub` here makes that
    // safety explicit instead of relying on wraparound arithmetic to cancel
    // out correctly. ~keep
    let search_range = 1024u64;
    let start = approx_offset.saturating_sub(search_range);
    let end = approx_offset.saturating_add(search_range);

    reader.seek(SeekFrom::Start(start))?;
    let mut buffer = vec![0u8; end.saturating_sub(start) as usize];
    let bytes_read = reader.read(&mut buffer)?;
    let buffer = &buffer[..bytes_read];

    let pattern = format!(r"{} \d+ obj", obj_id);
    let re = match regex::bytes::Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return Err(Error::ObjectNotFound(obj_id, 0)),
    };

    if let Some(mat) = re.find(buffer) {
        let obj_offset = start + mat.start() as u64;
        tracing::warn!(
            object_id = obj_id,
            offset = obj_offset,
            approx_offset,
            "found object near expected offset"
        );

        return load_object_at_offset(reader, obj_offset);
    }

    Err(Error::ObjectNotFound(obj_id, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_reconstruct_simple_pdf() {
        let pdf_data = b"%PDF-1.4\n\
            1 0 obj\n\
            << /Type /Catalog /Pages 2 0 R >>\n\
            endobj\n\
            2 0 obj\n\
            << /Type /Pages /Count 0 /Kids [] >>\n\
            endobj\n\
            trailer\n\
            << /Root 1 0 R /Size 3 >>\n\
            startxref\n\
            0\n\
            %%EOF";

        let mut cursor = Cursor::new(pdf_data);
        let result = reconstruct_xref(&mut cursor);

        assert!(result.is_ok());
        let (xref, trailer, _synthetic) = result.unwrap();

        assert!(xref.contains(1));
        assert!(xref.contains(2));

        if let Some(dict) = trailer.as_dict() {
            assert!(dict.contains_key("Root"));
        } else {
            panic!("Trailer is not a dictionary");
        }
    }

    #[test]
    fn object_without_endobj_is_read_in_linear_time() {
        let mut body = b"1 0 obj\n".to_vec();
        body.extend(std::iter::repeat_n(b'\n', 900_000));
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(load_object_at_offset(&mut Cursor::new(body), 0).is_err());
        });
        let finished = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("load_object_at_offset did not finish");
        assert!(finished);
    }

    #[test]
    fn newline_free_object_is_not_read_past_the_size_cap() {
        struct Counting {
            inner: Cursor<Vec<u8>>,
            read: usize,
        }
        impl Read for Counting {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = self.inner.read(buf)?;
                self.read += n;
                Ok(n)
            }
        }
        impl Seek for Counting {
            fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
                self.inner.seek(pos)
            }
        }
        let mut body = b"1 0 obj\n<< /K (".to_vec();
        body.extend(std::iter::repeat_n(b'a', 4 * 1024 * 1024));
        let mut reader = Counting {
            inner: Cursor::new(body),
            read: 0,
        };
        assert!(load_object_at_offset(&mut reader, 0).is_err());
        assert!(reader.read <= 1024 * 1024 + 1, "read {} bytes", reader.read);
    }

    #[test]
    fn unclosed_trailers_do_not_reparse_to_end_of_file() {
        let mut pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog >>\nendobj\n".to_vec();
        for _ in 0..4_000 {
            pdf.extend_from_slice(b"trailer<</K (");
            pdf.extend(std::iter::repeat_n(b'a', 300));
            pdf.push(b'\n');
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(reconstruct_xref(&mut Cursor::new(pdf)).is_ok());
        });
        rx.recv_timeout(std::time::Duration::from_secs(20))
            .expect("reconstruct_xref did not finish");
    }

    #[test]
    fn trailer_cut_off_by_the_parse_window_is_not_accepted() {
        let head = b"<</Root 1 0 R /K (";
        let mut contents = b"%PDF-1.4\ntrailer".to_vec();
        contents.extend_from_slice(head);
        contents.extend(std::iter::repeat_n(b'a', MAX_TRAILER_WINDOW - head.len() - 1));
        contents.push(b')');
        contents.extend_from_slice(b" /Encrypt 5 0 R /ID [(a) (b)] >>\n");
        let mat = RE_TRAILER.find(&contents).expect("trailer keyword");
        let mut best = None;
        let mut salvaged = HashMap::new();
        process_trailer_match(&contents, mat, &mut best, &mut salvaged);
        assert!(best.is_none(), "a window-truncated trailer must not be accepted");
    }

    #[test]
    fn complete_trailer_inside_the_parse_window_is_accepted() {
        let contents = b"%PDF-1.4\ntrailer\n<< /Root 1 0 R /Size 2 >>\nstartxref\n0\n%%EOF\n".to_vec();
        let mat = RE_TRAILER.find(&contents).expect("trailer keyword");
        let mut best = None;
        let mut salvaged = HashMap::new();
        process_trailer_match(&contents, mat, &mut best, &mut salvaged);
        assert!(best.is_some());
    }

    #[test]
    fn test_is_catalog() {
        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("Catalog".to_string()));
        let catalog = Object::Dictionary(dict);

        assert!(is_catalog(&catalog));

        let not_catalog = Object::Integer(42);
        assert!(!is_catalog(&not_catalog));
    }

    #[test]
    fn test_reconstruct_no_objects() {
        let pdf_data = b"%PDF-1.4\n\
            This is not a valid PDF with objects\n\
            %%EOF";

        let mut cursor = Cursor::new(pdf_data);
        let result = reconstruct_xref(&mut cursor);

        assert!(result.is_err());
    }
}
