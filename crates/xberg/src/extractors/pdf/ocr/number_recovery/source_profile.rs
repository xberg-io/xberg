use super::{Failure, ProofResult, flate_filter, require};
use std::collections::HashSet;
use std::io::Read;
use xberg_native_pdf::{
    PdfDocument,
    layout::TextChar,
    object::{Object, ObjectRef},
};

const MAX_CONTENT_BYTES: usize = 16 * 1024;
const MAX_CONTENT_OBJECTS: usize = 32;
const MAX_CONTENT_DEPTH: usize = 8;
const MAX_TOKENS: usize = 4096;
const MAX_OPERATIONS: usize = 1024;
const MAX_TEXT_BYTES: usize = 4096;
const MAX_ARRAY_OPERANDS: usize = 1024;
const MAX_OPERANDS: usize = 64;
const MAX_TOKEN_BYTES: usize = 32;
const MAX_SCALARS_PER_BYTE: usize = 8;
const CHARACTER_WORK_FACTOR: usize = 8;
const CHARACTER_SCRATCH_BYTES: usize = 32;
pub(super) const MAX_NAME_BYTES: usize = 128;
// Covers decoded input, bounded parser operands/AST, graphics state and extractor setup. ~keep
const FIXED_WORK_BYTES: usize = 2 * 1024 * 1024;

pub(super) struct Admitted {
    pub(super) bytes: Vec<u8>,
    pub(super) working_bytes: usize,
    pub(super) max_chars: usize,
}

pub(super) fn admit(document: &PdfDocument, page: usize, max_bytes: usize) -> ProofResult<Admitted> {
    require(!document.is_encrypted(), "encrypted source content")?;
    require(max_bytes >= FIXED_WORK_BYTES, "source working byte budget")?;
    require(
        document
            .trailer()
            .as_dict()
            .and_then(|dict| dict.get("Root"))
            .and_then(Object::as_reference)
            .is_some(),
        "missing source catalog reference",
    )?;
    let catalog = document.catalog()?;
    let catalog = catalog.as_dict().ok_or(Failure::Declined("source catalog"))?;
    require(!catalog.contains_key("StructTreeRoot"), "tagged source content")?;
    let resources = document.get_page_resources(page)?;
    let resources = resources.as_dict().ok_or(Failure::Declined("source resources"))?;
    require(
        !resources.contains_key("Properties") && !resources.contains_key("ExtGState"),
        "source property resources",
    )?;
    let page = document.get_page(page)?;
    let page = page.as_dict().ok_or(Failure::Declined("source page dictionary"))?;
    require(
        !page.contains_key("StructParents") && !page.contains_key("ActualText"),
        "tagged source page",
    )?;
    let mut decoder = Contents::new(document);
    decoder.visit(
        page.get("Contents")
            .ok_or(Failure::Declined("missing source Contents"))?,
        0,
    )?;
    let max_chars = scan(&decoder.bytes)?;
    let working_bytes = reserve_characters(max_chars, max_bytes)?;
    Ok(Admitted {
        bytes: decoder.bytes,
        working_bytes,
        max_chars,
    })
}
fn reserve_characters(max_chars: usize, max_bytes: usize) -> ProofResult<usize> {
    require(
        max_chars <= super::super::MAX_NATIVE_CHARS,
        "source character emission bound",
    )?;
    // Eight scalars per byte plus TJ offset spaces bound emission BEFORE native sort/dedup. ~keep
    // Charge vector growth, names, candidates and scratch alongside copied font/PDF storage. ~keep
    let per_char = std::mem::size_of::<TextChar>()
        + MAX_NAME_BYTES
        + std::mem::size_of::<super::Number>()
        + CHARACTER_SCRATCH_BYTES;
    let working_bytes =
        FIXED_WORK_BYTES.saturating_add(max_chars.saturating_mul(per_char).saturating_mul(CHARACTER_WORK_FACTOR));
    require(working_bytes <= max_bytes, "source character byte reservation")?;
    Ok(working_bytes)
}

struct Contents<'a> {
    document: &'a PdfDocument,
    bytes: Vec<u8>,
    active: HashSet<ObjectRef>,
    objects: usize,
    raw_bytes: usize,
}
impl<'a> Contents<'a> {
    fn new(document: &'a PdfDocument) -> Self {
        Self {
            document,
            bytes: Vec::with_capacity(MAX_CONTENT_BYTES + 1),
            active: HashSet::new(),
            objects: 0,
            raw_bytes: 0,
        }
    }
    fn visit(&mut self, object: &Object, depth: usize) -> ProofResult<()> {
        require(depth <= MAX_CONTENT_DEPTH, "source Contents depth")?;
        self.objects += 1;
        require(self.objects <= MAX_CONTENT_OBJECTS, "source Contents count")?;
        match object {
            Object::Reference(reference) => {
                require(self.active.insert(*reference), "source Contents cycle")?;
                let resolved = self.document.resolve_object(object)?;
                self.visit(&resolved, depth + 1)?;
                self.active.remove(reference);
            }
            Object::Array(objects) => {
                require(objects.len() <= MAX_CONTENT_OBJECTS, "source Contents array count")?;
                for object in objects {
                    self.visit(object, depth + 1)?;
                }
            }
            Object::Stream { dict, data } => self.stream(dict, data)?,
            _ => return Err(Failure::Declined("source Contents object")),
        }
        Ok(())
    }
    fn stream(&mut self, dict: &std::collections::HashMap<String, Object>, data: &[u8]) -> ProofResult<()> {
        require(
            !["DecodeParms", "F", "FFilter", "FDecodeParms"]
                .iter()
                .any(|key| dict.contains_key(*key)),
            "source stream parameters",
        )?;
        self.raw_bytes = self.raw_bytes.saturating_add(data.len());
        require(self.raw_bytes <= MAX_CONTENT_BYTES, "source raw byte budget")?;
        let remaining = MAX_CONTENT_BYTES.saturating_sub(self.bytes.len());
        match dict.get("Filter") {
            None => {
                require(data.len() < remaining, "source decoded byte budget")?;
                self.bytes.extend_from_slice(data);
            }
            Some(filter) if flate_filter(filter) => {
                flate2::read::ZlibDecoder::new(data)
                    .take(remaining as u64)
                    .read_to_end(&mut self.bytes)?;
                require(self.bytes.len() < MAX_CONTENT_BYTES, "source decoded byte budget")?;
            }
            _ => return Err(Failure::Declined("source stream filter")),
        }
        self.bytes.push(b'\n');
        Ok(())
    }
}

// A nonrecursive lexical guard runs BEFORE Native's recursive operand parser. It accepts one array ~keep
// level (TJ/dashes), no dictionaries, bounded literal nesting and an explicit operator vocabulary. ~keep
fn scan(bytes: &[u8]) -> ProofResult<usize> {
    let mut lexer = Lexer {
        bytes,
        cursor: 0,
        strings: 0,
        tokens: 0,
        operations: 0,
        array: false,
        operands: 0,
        graphics: 0,
        offsets: 0,
    };
    while lexer.cursor < bytes.len() {
        lexer.next()?;
    }
    require(
        !lexer.array && lexer.graphics == 0 && lexer.operands == 0,
        "unfinished source operands",
    )?;
    Ok(lexer
        .strings
        .saturating_mul(MAX_SCALARS_PER_BYTE)
        .saturating_add(lexer.offsets))
}
struct Lexer<'a> {
    bytes: &'a [u8],
    cursor: usize,
    strings: usize,
    tokens: usize,
    operations: usize,
    array: bool,
    operands: usize,
    graphics: usize,
    offsets: usize,
}
impl Lexer<'_> {
    fn next(&mut self) -> ProofResult<()> {
        let byte = self.bytes[self.cursor];
        if byte.is_ascii_whitespace() || byte == 0 {
            self.cursor += 1;
            return Ok(());
        }
        if byte == b'%' {
            return Err(Failure::Declined("source comment"));
        }

        self.tokens += 1;
        require(self.tokens <= MAX_TOKENS, "source token count")?;
        self.operands += 1;
        match byte {
            b'(' => self.literal()?,
            b'<' => self.hex()?,
            b'[' => {
                require(!self.array, "nested source array")?;
                self.array = true;
                self.cursor += 1;
            }
            b']' => {
                require(self.array, "unbalanced source array")?;
                self.array = false;
                self.cursor += 1;
            }
            b'/' => self.name()?,
            b'>' | b')' | b'{' | b'}' => return Err(Failure::Declined("source delimiter")),
            _ => self.word()?,
        }
        require(self.strings <= MAX_TEXT_BYTES, "source text show byte count")?;
        require(
            self.operands <= MAX_ARRAY_OPERANDS && (self.array || self.operands <= MAX_OPERANDS),
            "source operand count",
        )
    }
    fn literal(&mut self) -> ProofResult<()> {
        self.cursor += 1;
        let start = self.cursor;
        let mut depth = 1;
        while let Some(&byte) = self.bytes.get(self.cursor) {
            self.cursor += 1;
            match byte {
                b'\\' => {
                    require(self.cursor < self.bytes.len(), "source string escape")?;
                    self.cursor += 1;
                }
                b'(' => {
                    depth += 1;
                    require(depth <= MAX_CONTENT_DEPTH, "source literal depth")?;
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        self.strings += self.cursor - start - 1;
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        Err(Failure::Declined("unfinished source literal"))
    }
    fn hex(&mut self) -> ProofResult<()> {
        self.cursor += 1;
        let mut digits: usize = 0;
        while let Some(&byte) = self.bytes.get(self.cursor) {
            self.cursor += 1;
            if byte == b'>' {
                self.strings += digits.div_ceil(2);
                return Ok(());
            }
            require(
                byte.is_ascii_hexdigit() || byte.is_ascii_whitespace(),
                "source hex or dictionary",
            )?;
            digits += usize::from(byte.is_ascii_hexdigit());
        }
        Err(Failure::Declined("unfinished source hex"))
    }
    fn name(&mut self) -> ProofResult<()> {
        self.cursor += 1;
        let start = self.cursor;
        while self.cursor < self.bytes.len() && !delimiter(self.bytes[self.cursor]) {
            self.cursor += 1;
        }
        require(
            self.cursor - start <= MAX_NAME_BYTES && self.bytes[start..self.cursor].is_ascii(),
            "source resource name length or encoding",
        )
    }
    fn word(&mut self) -> ProofResult<()> {
        let start = self.cursor;
        while self.cursor < self.bytes.len() && !delimiter(self.bytes[self.cursor]) {
            self.cursor += 1;
        }
        let token = &self.bytes[start..self.cursor];
        require(
            !token.is_empty() && token.len() <= MAX_TOKEN_BYTES,
            "source token length",
        )?;
        if matches!(token[0], b'0'..=b'9' | b'+' | b'-' | b'.') {
            require(
                token
                    .iter()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.')),
                "source number grammar",
            )?;
            self.offsets += usize::from(self.array);
            let number = std::str::from_utf8(token)
                .ok()
                .and_then(|text| text.parse::<f32>().ok());
            return require(number.is_some_and(f32::is_finite), "source finite operand");
        }
        require(!self.array && allowed(token), "unsupported source operator")?;
        self.operations += 1;
        require(self.operations <= MAX_OPERATIONS, "source operation count")?;
        if token == b"q" {
            self.graphics += 1;
        }
        if token == b"Q" {
            self.graphics = self
                .graphics
                .checked_sub(1)
                .ok_or(Failure::Declined("source graphics stack"))?;
        }
        require(self.graphics <= MAX_CONTENT_DEPTH, "source graphics depth")?;
        self.operands = 0;
        Ok(())
    }
}
fn delimiter(byte: u8) -> bool {
    byte.is_ascii_whitespace()
        || matches!(
            byte,
            0 | b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
        )
}
fn allowed(token: &[u8]) -> bool {
    const OPERATORS: &[&[u8]] = &[
        b"q", b"Q", b"cm", b"Do", b"BT", b"ET", b"Tf", b"Tm", b"Td", b"TD", b"T*", b"Tc", b"Tw", b"Tz", b"TL", b"Tr",
        b"Ts", b"Tj", b"TJ", b"'", b"\"", b"g", b"G", b"rg", b"RG", b"k", b"K", b"w", b"J", b"j", b"M", b"d", b"ri",
        b"i", b"m", b"l", b"c", b"v", b"y", b"h", b"re", b"S", b"s", b"f", b"F", b"f*", b"B", b"B*", b"b", b"b*", b"n",
        b"W", b"W*",
    ];
    OPERATORS.contains(&token)
}
