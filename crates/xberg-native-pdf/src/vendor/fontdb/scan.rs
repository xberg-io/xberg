// Not derived from upstream fontdb: this replaces the whole-file read in
// `system_fonts::load_font_file`. Upstream fontdb (`fs` + `memmap` features)
// memory-maps each font file so the face scan touches only the pages it
// parses; this copy dropped the memmap path to keep the crate free of
// `unsafe`, which left a plain `std::fs::read` of every system font (813 MB
// across 369 files on macOS, Apple Color Emoji alone 192 MB) on the first page
// render. Reading
// only the table directories and the four tables the face parse consults gets
// upstream's I/O profile back without `unsafe`. ~keep

//! System-font face scan that reads only the bytes `parse::parse_face_info`
//! consults: the collection header, each face's table directory and its
//! `OS/2`, `head`, `name` and `post` tables.

// `system_fonts::load_font_file` is the only caller and is itself unreachable
// on wasm32 and Android (see `load_fonts_dir_impl`). ~keep
#![cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]

use std::collections::HashMap;
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
/// `FontRef::fonts` + `parse::parse_face_info` pass would. Reads only the
/// table directories and each face's [`FACE_INFO_TABLES`] unless those tables
/// add up to more than the file itself, and then reads the file once instead.
pub(super) fn read_face_infos(path: &Path) -> io::Result<Vec<Result<FaceInfo, LoadError>>> {
    match read_face_tables_within_budget(path)? {
        Some(faces) => Ok(faces),
        None => read_face_infos_whole_file(path),
    }
}

/// A face's sfnt version and its present tables as `(tag, (offset, length))`.
type FaceTables = (u32, Vec<(Tag, (u32, u32))>);

/// The distinct table byte ranges of `faces`, sorted, or `None` when they sum
/// to more than `file_len`. Collection faces commonly share tables, so each
/// range is read once. ~keep
fn distinct_ranges_within(faces: &[Option<FaceTables>], file_len: usize) -> Option<Vec<(u32, u32)>> {
    let mut ranges: Vec<(u32, u32)> = faces
        .iter()
        .flatten()
        .flat_map(|(_, present)| present.iter().map(|&(_, range)| range))
        .collect();
    ranges.sort_unstable();
    ranges.dedup();
    let total = ranges.iter().try_fold(0_usize, |total, &(_, length)| {
        total.checked_add(length as usize).filter(|&total| total <= file_len)
    });
    total.map(|_| ranges)
}

/// The partial read behind [`read_face_infos`]: each face is parsed from a
/// compact single-face font holding its [`FACE_INFO_TABLES`] byte for byte.
///
/// `None` when the distinct table ranges sum to more than the file length.
/// Collection faces can point at different large, overlapping ranges, so
/// reading per range could otherwise cost a multiple of the file. The
/// directory prefix reads double up to the file length, so the partial path
/// never reads more than about twice the file. ~keep
fn read_face_tables_within_budget(path: &Path) -> io::Result<Option<Vec<Result<FaceInfo, LoadError>>>> {
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

    // `FontRef::table_data` reads a null offset, or a range past the end of
    // the file, as a missing table. ~keep
    let faces: Vec<Option<FaceTables>> = directories
        .into_iter()
        .map(|directory| {
            directory.map(|directory| {
                let present = directory
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
                (directory.sfnt_version, present)
            })
        })
        .collect();

    let Some(ranges) = distinct_ranges_within(&faces, file_len) else {
        return Ok(None);
    };
    let mut table_bytes: HashMap<(u32, u32), Vec<u8>> = HashMap::with_capacity(ranges.len());
    for (offset, length) in ranges {
        let bytes = read_range(&mut file, offset as usize, length as usize)?;
        table_bytes.insert((offset, length), bytes);
    }

    let source = Source::File(path.to_path_buf());
    let mut parsed_faces = Vec::with_capacity(faces.len());
    for (index, face) in faces.into_iter().enumerate() {
        let index = index as u32;
        let Some((sfnt_version, present)) = face else {
            parsed_faces.push(Err(LoadError::Malformed));
            continue;
        };
        let tables: Vec<(Tag, &[u8])> = present
            .iter()
            .map(|(tag, range)| (*tag, table_bytes[range].as_slice()))
            .collect();
        let parsed = match compact_font(sfnt_version, &tables) {
            Some(compact) => match FontRef::new(&compact) {
                Ok(font) => parse::parse_face_info(&font, source.clone(), index),
                Err(_) => Err(LoadError::Malformed),
            },
            None => Err(LoadError::Malformed),
        };
        parsed_faces.push(parsed);
    }
    Ok(Some(parsed_faces))
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

/// The whole-file read: upstream's parse over the full file bytes. The
/// fallback for files whose tables exceed the partial-read budget, and the
/// reference the tests compare the partial read against.
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

    /// The partial read alone; panics if the file falls back to a whole read,
    /// so a comparison against [`whole_file`] cannot pass by taking that path.
    fn scanned(path: &Path) -> Vec<FaceSummary> {
        summarize(
            read_face_tables_within_budget(path)
                .expect("scan font file")
                .expect("tables fit the partial-read budget"),
        )
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

    /// Byte position of the `tag` record in the table directory at `base`.
    fn record_position(font: &[u8], base: usize, tag: &[u8; 4]) -> usize {
        let num_tables = usize::from(u16::from_be_bytes([font[base + 4], font[base + 5]]));
        (0..num_tables)
            .map(|record| base + SFNT_HEADER_LEN + record * TABLE_RECORD_LEN)
            .find(|&at| &font[at..at + 4] == tag)
            .expect("table record present")
    }

    fn be_u32(bytes: &[u8], at: usize) -> u32 {
        u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    /// Each face's `OS/2` record reaches almost to the end of the file, with a
    /// distinct length per face, so the distinct ranges sum to more than the
    /// file: the scan must read the file once instead of once per range.
    #[test]
    fn scan_reads_the_whole_file_once_when_collection_tables_exceed_it() {
        let dir = tempfile::tempdir().unwrap();
        let font = crate::fonts::bundled::DEJAVU_SANS;
        let mut ttc = collection(&[font, font, font, font], 0);
        let file_len = u32::try_from(ttc.len()).unwrap();
        for face in 0..4_u32 {
            let base = be_u32(&ttc, 12 + 4 * face as usize) as usize;
            let os2 = record_position(&ttc, base, b"OS/2");
            let length = file_len - be_u32(&ttc, os2 + 8) - 4 * face;
            ttc[os2 + 12..os2 + 16].copy_from_slice(&length.to_be_bytes());
        }
        let path = write_font(&dir, "overlapping.ttc", &ttc);

        assert!(
            read_face_tables_within_budget(&path).unwrap().is_none(),
            "overlapping tables must exceed the partial-read budget"
        );
        let faces = summarize(read_face_infos(&path).expect("read font file"));
        assert_eq!(faces, whole_file(&path));
        let weights: Vec<u16> = faces
            .iter()
            .map(|face| face.as_ref().expect("collection face parses").3.0)
            .collect();
        assert_eq!(weights, [400; 4]);
    }

    /// Lookup order matters once a directory is unsorted or repeats a tag:
    /// `FontRef::table_data` then takes the first matching record. An `OTTO`
    /// sfnt version must carry through to the compact font.
    #[test]
    fn scan_matches_whole_file_read_for_unusual_directories() {
        let dir = tempfile::tempdir().unwrap();
        let font = crate::fonts::bundled::DEJAVU_SANS;
        let num_tables = usize::from(u16::from_be_bytes([font[4], font[5]]));
        let last = SFNT_HEADER_LEN + (num_tables - 1) * TABLE_RECORD_LEN;
        let name = record_position(font, 0, b"name");
        let head = record_position(font, 0, b"head");
        let bogus_name = |at: usize| {
            let mut copy = font.to_vec();
            copy.copy_within(head..head + TABLE_RECORD_LEN, at);
            copy[at..at + 4].copy_from_slice(b"name");
            copy
        };

        let mut unsorted = font.to_vec();
        let first: Vec<u8> = unsorted[SFNT_HEADER_LEN..SFNT_HEADER_LEN + TABLE_RECORD_LEN].to_vec();
        unsorted.copy_within(last..last + TABLE_RECORD_LEN, SFNT_HEADER_LEN);
        unsorted[last..last + TABLE_RECORD_LEN].copy_from_slice(&first);
        let mut cff_version = font.to_vec();
        cff_version[..4].copy_from_slice(b"OTTO");

        for (file, bytes, parses) in [
            ("unsorted.ttf", unsorted, true),
            ("duplicate-name-after.ttf", bogus_name(last), true),
            ("duplicate-name-before.ttf", bogus_name(name - TABLE_RECORD_LEN), false),
            ("otto.otf", cff_version, true),
        ] {
            let path = write_font(&dir, file, &bytes);
            let faces = scanned(&path);
            assert_eq!(faces, whole_file(&path), "{file}");
            if parses {
                let (_, families, ..) = faces[0].as_ref().expect("face parses");
                assert_eq!(families[0].0, "DejaVu Sans", "{file}");
            }
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
