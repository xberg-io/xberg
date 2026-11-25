// Not derived from upstream fontdb: this replaces the whole-file read in
// `system_fonts::load_font_file`. Upstream fontdb (`fs` + `memmap` features)
// memory-maps each font file so the face scan touches only the pages it
// parses; this copy dropped the memmap path to keep the crate free of
// `unsafe`, which left a plain `std::fs::read` of every system font (~565 MB
// on macOS, Apple Color Emoji alone 192 MB) on the first page render. Reading
// only the table directories and the four tables the face parse consults gets
// upstream's I/O profile back without `unsafe`. ~keep

//! System-font face scan that reads only the bytes `parse::parse_face_info`
//! consults: the collection header, each face's table directory and its
//! `OS/2`, `head`, `name` and `post` tables.

// `system_fonts::load_font_file` is the only caller and is itself unreachable
// on wasm32 and Android (see `load_fonts_dir_impl`). ~keep
#![cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use skrifa::raw::{CollectionRef, ReadError};
use skrifa::{FontRef, Tag};

use super::parse::{self, LoadError};
use super::{FaceInfo, Source};

/// The tables `parse::parse_face_info` reads: `name` through
/// `localized_strings`, and `OS/2`, `post` and `head` through skrifa's
/// `Attributes::new` and the italic-angle promotion. Sorted by tag, so the
/// compact directory built from them is sorted as well. ~keep
const FACE_INFO_TABLES: [Tag; 4] = [
    Tag::new(b"OS/2"),
    Tag::new(b"head"),
    Tag::new(b"name"),
    Tag::new(b"post"),
];

/// First prefix read for the collection header and table directories. A
/// single face's directory is 12 + 16 * numTables bytes, so one read covers
/// nearly every file; the prefix doubles when a directory lies past it.
const DIRECTORY_PREFIX_BYTES: usize = 4096;

const SFNT_HEADER_LEN: usize = 12;
const TABLE_RECORD_LEN: usize = 16;

/// One face's sfnt version and, for each of [`FACE_INFO_TABLES`] it has, the
/// `(tag, offset, length)` record a whole-file `FontRef::table_data` resolves.
struct FaceDirectory {
    sfnt_version: u32,
    tables: Vec<(Tag, u32, u32)>,
}

impl FaceDirectory {
    fn new(font: &FontRef) -> Self {
        let directory = font.table_directory();
        let records = directory.table_records();
        // `FontRef::table_data` binary-searches a sorted directory, which holds
        // each tag once, and takes the first match in an unsorted one: either
        // way the first record with the tag is the one read. ~keep
        let tables = FACE_INFO_TABLES
            .iter()
            .filter_map(|&tag| {
                records
                    .iter()
                    .find(|record| record.tag() == tag)
                    .map(|record| (tag, record.offset(), record.length()))
            })
            .collect();
        Self {
            sfnt_version: directory.sfnt_version(),
            tables,
        }
    }
}

/// Parses the face directories in `prefix`, the first bytes of a font file,
/// in `FontRef::fonts` order. `None` means `prefix` ends before a structure
/// the parse needs and the file is longer, so the caller reads more.
/// `complete` means `prefix` is the whole file, and every error is then the
/// file's own, reported exactly as `FontRef::fonts` would.
fn face_directories(prefix: &[u8], complete: bool) -> Option<Vec<Option<FaceDirectory>>> {
    let truncated = |error: &ReadError| !complete && matches!(error, ReadError::OutOfBounds);
    // `FileRef::new`'s order: a collection first, then a single font; a file
    // that is neither has no faces and no per-face error. ~keep
    let fonts: Vec<Result<FontRef<'_>, ReadError>> = match CollectionRef::new(prefix) {
        Ok(collection) => collection.iter().collect(),
        Err(error) if truncated(&error) => return None,
        Err(_) => match FontRef::new(prefix) {
            Ok(font) => vec![Ok(font)],
            Err(error) if truncated(&error) => return None,
            Err(_) => Vec::new(),
        },
    };

    let mut directories = Vec::with_capacity(fonts.len());
    for font in fonts {
        match font {
            Ok(font) => {
                // `table_records` reads as empty, not as an error, when the
                // records run past the data, so truncation is checked here. ~keep
                let directory = font.table_directory();
                if !complete && directory.table_records_byte_range().end > directory.offset_data().len() {
                    return None;
                }
                directories.push(Some(FaceDirectory::new(&font)));
            }
            Err(error) if truncated(&error) => return None,
            Err(_) => directories.push(None),
        }
    }
    Some(directories)
}

/// Reads the faces of the font file at `path` as a whole-file
/// `FontRef::fonts` + `parse::parse_face_info` pass would, reading only the
/// table directories and each face's [`FACE_INFO_TABLES`]. Each face is parsed
/// from a compact single-face font holding those tables byte for byte.
pub(super) fn read_face_infos(path: &Path) -> io::Result<Vec<Result<FaceInfo, LoadError>>> {
    let mut file = File::open(path)?;
    let file_len = usize::try_from(file.metadata()?.len())
        .map_err(|_| io::Error::other("font file is larger than the address space"))?;

    let mut prefix_len = file_len.min(DIRECTORY_PREFIX_BYTES);
    let directories = loop {
        let prefix = read_range(&mut file, 0, prefix_len)?;
        if let Some(directories) = face_directories(&prefix, prefix_len == file_len) {
            break directories;
        }
        prefix_len = prefix_len.saturating_mul(2).min(file_len);
    };

    let source = Source::File(path.to_path_buf());
    // Collection faces commonly share tables; read each byte range once. ~keep
    let mut table_bytes: HashMap<(u32, u32), Vec<u8>> = HashMap::new();
    let mut faces = Vec::with_capacity(directories.len());
    for (index, directory) in directories.into_iter().enumerate() {
        let index = index as u32;
        let Some(directory) = directory else {
            faces.push(Err(LoadError::Malformed));
            continue;
        };

        // `FontRef::table_data` reads a null offset, or a range past the end
        // of the file, as a missing table. ~keep
        let present: Vec<(Tag, (u32, u32))> = directory
            .tables
            .into_iter()
            .filter(|&(_, offset, length)| {
                offset != 0
                    && (offset as usize)
                        .checked_add(length as usize)
                        .is_some_and(|end| end <= file_len)
            })
            .map(|(tag, offset, length)| (tag, (offset, length)))
            .collect();
        for &(_, (offset, length)) in &present {
            if let Entry::Vacant(slot) = table_bytes.entry((offset, length)) {
                slot.insert(read_range(&mut file, offset as usize, length as usize)?);
            }
        }

        let tables: Vec<(Tag, &[u8])> = present
            .iter()
            .map(|(tag, range)| (*tag, table_bytes[range].as_slice()))
            .collect();
        let parsed = match compact_font(directory.sfnt_version, &tables) {
            Some(compact) => match FontRef::new(&compact) {
                Ok(font) => parse::parse_face_info(&font, source.clone(), index),
                Err(_) => Err(LoadError::Malformed),
            },
            None => Err(LoadError::Malformed),
        };
        faces.push(parsed);
    }
    Ok(faces)
}

/// Lays `tables` out as a single-face sfnt: a table directory whose records
/// point at the tables packed behind it on four-byte boundaries. Checksums
/// and the binary-search header fields stay zero; read-fonts reads neither.
fn compact_font(sfnt_version: u32, tables: &[(Tag, &[u8])]) -> Option<Vec<u8>> {
    let num_tables = u16::try_from(tables.len()).ok()?;
    let mut offset = SFNT_HEADER_LEN + TABLE_RECORD_LEN * tables.len();
    let data_len: usize = tables.iter().map(|(_, data)| data.len().next_multiple_of(4)).sum();
    let mut font = Vec::with_capacity(offset + data_len);
    font.extend_from_slice(&sfnt_version.to_be_bytes());
    font.extend_from_slice(&num_tables.to_be_bytes());
    font.extend_from_slice(&[0; 6]);
    for (tag, data) in tables {
        font.extend_from_slice(&tag.to_be_bytes());
        font.extend_from_slice(&[0; 4]);
        font.extend_from_slice(&u32::try_from(offset).ok()?.to_be_bytes());
        font.extend_from_slice(&u32::try_from(data.len()).ok()?.to_be_bytes());
        offset += data.len().next_multiple_of(4);
    }
    for (_, data) in tables {
        font.extend_from_slice(data);
        font.resize(font.len().next_multiple_of(4), 0);
    }
    Some(font)
}

fn read_range(file: &mut File, start: usize, len: usize) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(start as u64))?;
    let mut bytes = vec![0; len];
    file.read_exact(&mut bytes)?;
    Ok(bytes)
}

/// The whole-file read `read_face_infos` replaced, kept as the reference the
/// tests compare it against.
#[cfg(test)]
fn read_face_infos_whole_file(path: &Path) -> io::Result<Vec<Result<FaceInfo, LoadError>>> {
    let data = std::fs::read(path)?;
    let source = Source::File(path.to_path_buf());
    Ok(FontRef::fonts(&data)
        .enumerate()
        .map(|(index, font)| match font {
            Ok(font) => parse::parse_face_info(&font, source.clone(), index as u32),
            Err(_) => Err(LoadError::Malformed),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::{Database, Family, Query, Stretch, Style, Weight};
    use super::*;

    type FaceSummary = Result<(u32, Vec<(String, Option<String>)>, Style, Weight, Stretch), String>;

    fn summarize(faces: Vec<Result<FaceInfo, LoadError>>) -> Vec<FaceSummary> {
        faces
            .into_iter()
            .map(|face| {
                face.map(|info| (info.index, info.families, info.style, info.weight, info.stretch))
                    .map_err(|error| error.to_string())
            })
            .collect()
    }

    fn scanned(path: &Path) -> Vec<FaceSummary> {
        summarize(read_face_infos(path).expect("scan font file"))
    }

    fn whole_file(path: &Path) -> Vec<FaceSummary> {
        summarize(read_face_infos_whole_file(path).expect("read font file"))
    }

    fn write_font(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, bytes).expect("write font fixture");
        path
    }

    /// A TrueType collection of `fonts` whose table directories start `gap`
    /// bytes after the header, each font's table offsets rebased onto the
    /// position of its copy.
    fn collection(fonts: &[&[u8]], gap: usize) -> Vec<u8> {
        let header_len = 12 + 4 * fonts.len();
        let mut bases = Vec::with_capacity(fonts.len());
        let mut base = header_len + gap;
        for font in fonts {
            bases.push(base);
            base += font.len().next_multiple_of(4);
        }

        let mut ttc = b"ttcf".to_vec();
        ttc.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
        ttc.extend_from_slice(&u32::try_from(fonts.len()).unwrap().to_be_bytes());
        for base in &bases {
            ttc.extend_from_slice(&u32::try_from(*base).unwrap().to_be_bytes());
        }
        ttc.resize(header_len + gap, 0);
        for (font, base) in fonts.iter().zip(&bases) {
            let mut copy = font.to_vec();
            let num_tables = usize::from(u16::from_be_bytes([copy[4], copy[5]]));
            for record in 0..num_tables {
                let at = SFNT_HEADER_LEN + record * TABLE_RECORD_LEN + 8;
                let offset = u32::from_be_bytes(copy[at..at + 4].try_into().unwrap());
                let rebased = offset + u32::try_from(*base).unwrap();
                copy[at..at + 4].copy_from_slice(&rebased.to_be_bytes());
            }
            ttc.extend_from_slice(&copy);
            ttc.resize(ttc.len().next_multiple_of(4), 0);
        }
        ttc
    }

    #[test]
    fn scan_matches_whole_file_read_for_single_fonts() {
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in [
            ("DejaVuSans.ttf", crate::fonts::bundled::DEJAVU_SANS),
            ("DejaVuSans-Bold.ttf", crate::fonts::bundled::DEJAVU_SANS_BOLD),
        ] {
            let path = write_font(&dir, name, bytes);
            let faces = scanned(&path);
            assert_eq!(faces, whole_file(&path), "{name}");
            let (index, families, ..) = faces[0].as_ref().expect("bundled face parses");
            assert_eq!((faces.len(), *index, families[0].0.as_str()), (1, 0, "DejaVu Sans"));
        }
    }

    /// The directories sit past the first prefix read, so the scan must reread
    /// a longer prefix instead of reporting the faces as tableless.
    #[test]
    fn scan_rereads_a_longer_prefix_for_collection_directories_past_it() {
        let dir = tempfile::tempdir().unwrap();
        let ttc = collection(
            &[
                crate::fonts::bundled::DEJAVU_SANS,
                crate::fonts::bundled::DEJAVU_SANS_BOLD,
            ],
            3 * DIRECTORY_PREFIX_BYTES,
        );
        let path = write_font(&dir, "DejaVu.ttc", &ttc);
        let faces = scanned(&path);
        assert_eq!(faces, whole_file(&path));
        let weights: Vec<u16> = faces
            .iter()
            .map(|face| face.as_ref().expect("collection face parses").3.0)
            .collect();
        assert_eq!(weights, [400, 700]);
    }

    #[test]
    fn scan_matches_whole_file_read_for_damaged_files() {
        let dir = tempfile::tempdir().unwrap();
        let font = crate::fonts::bundled::DEJAVU_SANS;
        let mut wrong_count = collection(&[font], 0);
        wrong_count[8..12].copy_from_slice(&3_u32.to_be_bytes());
        let cases: [(&str, Vec<u8>); 6] = [
            ("empty.ttf", Vec::new()),
            ("header-only.ttf", font[..8].to_vec()),
            ("tables-missing.ttf", font[..600].to_vec()),
            ("half.ttf", font[..font.len() / 2].to_vec()),
            ("garbage.ttf", vec![0xA5; 9000]),
            ("wrong-count.ttc", wrong_count),
        ];
        for (name, bytes) in cases {
            let path = write_font(&dir, name, &bytes);
            assert_eq!(scanned(&path), whole_file(&path), "{name}");
        }
    }

    fn font_files(dir: &Path, files: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                font_files(&path, files);
            } else if file_type.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "ttc" | "otf" | "otc"))
            {
                files.push(path);
            }
        }
    }

    /// Every installed font in the main system directories, including files
    /// the scan finds no face in, parses to the same faces both ways.
    #[test]
    fn scan_matches_whole_file_read_for_installed_system_fonts() {
        let mut files = Vec::new();
        for dir in [
            "/System/Library/Fonts",
            "/Library/Fonts",
            "/usr/share/fonts",
            "/usr/local/share/fonts",
        ] {
            font_files(Path::new(dir), &mut files);
        }
        if cfg!(target_os = "macos") {
            assert!(!files.is_empty(), "macOS ships fonts in /System/Library/Fonts");
        }
        for path in &files {
            assert_eq!(scanned(path), whole_file(path), "{}", path.display());
        }
    }

    fn file_path(source: &Source) -> Option<&Path> {
        match source {
            Source::File(path) => Some(path),
            Source::Binary(_) => None,
        }
    }

    /// Lookups against the scanned system database resolve to the same face
    /// as against a database built from whole-file reads of the same files.
    #[test]
    fn system_font_lookups_match_whole_file_database() {
        let mut scanned_db = Database::new();
        scanned_db.load_system_fonts();
        if scanned_db.faces.is_empty() {
            // Only a host without installed fonts gets here. ~keep
            assert!(!cfg!(target_os = "macos"), "macOS ships system fonts");
            return;
        }

        // fontconfig may list overlapping directories, loading a file twice;
        // the reference reads each file once, in first-seen order. ~keep
        let mut files: Vec<&Path> = Vec::new();
        let mut unique_faces = std::collections::HashSet::new();
        for face in &scanned_db.faces {
            if let Some(path) = file_path(&face.source) {
                if !files.contains(&path) {
                    files.push(path);
                }
                unique_faces.insert((path, face.index));
            }
        }
        let mut reference = Database::new();
        for path in files {
            reference.faces.extend(
                read_face_infos_whole_file(path)
                    .expect("read font file")
                    .into_iter()
                    .flatten(),
            );
        }
        assert_eq!(reference.faces.len(), unique_faces.len());

        let names = [
            "Helvetica",
            "Times",
            "Times New Roman",
            "Courier",
            "Arial",
            "DejaVu Sans",
            "Liberation Sans",
            "Noto Sans",
            "FreeSans",
        ];
        let mut resolved = 0;
        for name in names {
            for (weight, style) in [(Weight::NORMAL, Style::Normal), (Weight::BOLD, Style::Italic)] {
                let query = Query {
                    families: &[Family::Name(name)],
                    weight,
                    stretch: Stretch::Normal,
                    style,
                };
                let face = |db: &Database| {
                    db.query(&query)
                        .and_then(|id| db.face(id))
                        .map(|face| (file_path(&face.source).map(Path::to_path_buf), face.index))
                };
                let found = face(&scanned_db);
                assert_eq!(found, face(&reference), "{name} {weight:?} {style:?}");
                resolved += usize::from(found.is_some());
            }
        }
        assert!(resolved > 0, "no common font family resolved among {names:?}");
        if cfg!(target_os = "macos") {
            let helvetica = Query {
                families: &[Family::Name("Helvetica")],
                weight: Weight::NORMAL,
                stretch: Stretch::Normal,
                style: Style::Normal,
            };
            assert!(scanned_db.query(&helvetica).is_some(), "macOS ships Helvetica");
        }
    }
}
