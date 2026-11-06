//! CCITTFaxDecode implementation.
//!
//! In-house ITU-T T.6 (Group 4, 2D) CCITT fax decoder for monochrome scanned
//! images, plus the pass-through filter used in the stream-filter chain.
//!
//! The image-decode path (`decode`) replaces a third-party crate that could not
//! honor `/EncodedByteAlign` (the filter has no such hook and its bit reader is
//! private), which made byte-aligned fax scanners decode to garbage/blank pages.
//! It also recovers partial content from truncated/damaged streams instead of
//! discarding the whole page.
//!
//! PDF Spec: ISO 32000-1:2008 §7.4.6 (CCITTFaxDecode); algorithm: ITU-T T.6/T.4.

use crate::decoders::ccitt_tables::{BLACK_CODES, WHITE_CODES};
use crate::decoders::{CcittParams, StreamDecoder};
use crate::error::{Error, Result};
use crate::extractors::ccitt_bilevel::{append_transition_row, transitions_to_bytes};

mod strict;
pub use strict::decode_complete_group4;

/// CCITTFaxDecode stream filter (pass-through).
///
/// The raw CCITT codestream is kept compressed in the filter chain; actual
/// image decompression happens in `decode` at image-extraction time.
pub struct CcittFaxDecoder;

impl StreamDecoder for CcittFaxDecoder {
    // `max_output_bytes` (GH#1764) is ignored: this filter is a pass-through here, so
    // output never exceeds input length. ~keep
    fn decode(&self, input: &[u8], _max_output_bytes: usize) -> Result<Vec<u8>> {
        tracing::debug!(filter = "CCITTFaxDecode", bytes = input.len(), "pass-through");
        Ok(input.to_vec())
    }

    fn name(&self) -> &str {
        "CCITTFaxDecode"
    }
}

/// Outcome of an image decode: packed bilevel rows plus how the stream ended,
/// so the caller can distinguish a clean decode from recovered-partial content.
pub struct CcittDecoded {
    /// Packed bilevel bytes, `ceil(columns/8)` per row, MSB = leftmost pixel,
    /// `0 = white, 1 = black` (BEFORE any `BlackIs1` inversion). Padded with
    /// white rows to `Rows` when `Rows` is known.
    pub data: Vec<u8>,
    /// Rows actually decoded from the codestream (excludes white padding).
    pub rows_decoded: usize,
    /// True when the stream was truncated/damaged and the tail is recovered
    /// (white-padded) rather than a clean full-height / EOFB decode.
    pub recovered_partial: bool,
}

/// Decode a CCITT codestream to packed bilevel bytes, dispatching on `/K`:
/// Group 4 (T.6, `K < 0`) here, Group 3 (T.4, `K >= 0`) in [`decode_g3`].
///
/// Returns `Err` only when zero usable rows could be produced (genuinely
/// undecodable) — the caller may then fall back.
/// `BlackIs1` is NOT applied here; the caller owns that inversion.
pub fn decode(data: &[u8], params: &CcittParams) -> Result<CcittDecoded> {
    let width = params.columns as u16;
    if width == 0 {
        return Err(Error::Decode("CCITT decode requires /Columns".to_string()));
    }
    if !params.is_group_4() {
        return decode_g3(data, params);
    }

    let bytes_per_row = (width as usize).div_ceil(8);
    let mut reader = BitReader::new(data);
    let mut reference = transition_buffer(width)?;
    let mut current = transition_buffer(width)?;
    let mut out = packed_output_buffer(bytes_per_row, params.rows)?;
    let mut decoded_rows = 0usize;
    let mut byte_align = params.encoded_byte_align;
    let mut recovered = false;

    loop {
        if let Some(h) = params.rows
            && decoded_rows >= h as usize
        {
            break;
        }
        // /EncodedByteAlign: each coded row begins on a byte boundary, so skip
        // the zero fill bits left over from the previous row before reading the
        // next row's first mode code (pdfium-guarded: if a skipped bit is 1 the
        // declared alignment is wrong for this stream — disable it rather than
        // corrupt every subsequent row). ~keep
        if byte_align && decoded_rows > 0 {
            byte_align_skip(&mut reader, &mut byte_align);
        }
        if reader.eod() {
            if let Some(h) = params.rows
                && decoded_rows < h as usize
            {
                recovered = true;
            }
            break;
        }

        current.clear();
        match decode_row_g4::<false>(&mut reader, &reference, width, &mut current) {
            RowStatus::EndOfBlock => break,
            RowStatus::AllocationFailed => {
                return Err(Error::Decode("Unable to grow CCITT row transition buffer".to_string()));
            }
            RowStatus::Error => {
                if decoded_rows >= 1 {
                    recovered = true;
                    break;
                }
                return Err(Error::Decode("CCITT: stream undecodable from first row".to_string()));
            }
            RowStatus::Ok => {
                append_transition_row(&mut out, &current, width as usize)?;
                std::mem::swap(&mut reference, &mut current);
                decoded_rows += 1;
            }
        }
    }

    pad_to_declared_rows(&mut out, bytes_per_row, width, params.rows)?;

    if out.is_empty() {
        return Err(Error::Decode("CCITT: no output produced".to_string()));
    }

    Ok(CcittDecoded {
        data: out,
        rows_decoded: decoded_rows,
        recovered_partial: recovered,
    })
}

/// Decode a CCITT **Group 3** (T.4) codestream. `params.k == 0` is pure 1-D
/// (Modified Huffman); `params.k > 0` is mixed 1-D/2-D where a tag bit after the
/// line's EOL selects that line's coding (`1` = 1-D, `0` = 2-D-relative). EOL
/// codes (`000000000001`, absorbing any leading fill bits) delimit lines and are
/// tolerated whether or not `/EndOfLine` is declared; six consecutive EOLs is the
/// return-to-control terminator. Shares the run tables, reference-line walk and
/// recovery contract with the Group 4 path. `BlackIs1` is the caller's to apply.
fn decode_g3(data: &[u8], params: &CcittParams) -> Result<CcittDecoded> {
    let width = params.columns as u16;
    let two_dimensional = params.k > 0;
    let bytes_per_row = (width as usize).div_ceil(8);
    let mut reader = BitReader::new(data);
    let mut reference = transition_buffer(width)?;
    let mut current = transition_buffer(width)?;
    let mut out = packed_output_buffer(bytes_per_row, params.rows)?;
    let mut decoded_rows = 0usize;
    let mut byte_align = params.encoded_byte_align;
    let mut recovered = false;

    loop {
        if let Some(h) = params.rows
            && decoded_rows >= h as usize
        {
            break;
        }
        if byte_align && decoded_rows > 0 {
            byte_align_skip(&mut reader, &mut byte_align);
        }
        let eols = skip_eols(&mut reader);
        if reader.eod() {
            if let Some(h) = params.rows
                && decoded_rows < h as usize
            {
                recovered = true;
            }
            break;
        }
        if eols >= 6 {
            break;
        }

        let Some(one_d) = decide_one_d(&mut reader, two_dimensional) else {
            break;
        };

        current.clear();
        let status = if one_d {
            decode_row_g3_1d(&mut reader, width, &mut current)
        } else {
            decode_row_g4::<false>(&mut reader, &reference, width, &mut current)
        };
        match status {
            RowStatus::EndOfBlock => break,
            RowStatus::AllocationFailed => {
                return Err(Error::Decode("Unable to grow CCITT row transition buffer".to_string()));
            }
            RowStatus::Error => {
                if decoded_rows >= 1 {
                    recovered = true;
                    break;
                }
                return Err(Error::Decode("CCITT G3: stream undecodable from first row".to_string()));
            }
            RowStatus::Ok => {
                append_transition_row(&mut out, &current, width as usize)?;
                std::mem::swap(&mut reference, &mut current);
                decoded_rows += 1;
            }
        }
    }

    pad_to_declared_rows(&mut out, bytes_per_row, width, params.rows)?;
    if out.is_empty() {
        return Err(Error::Decode("CCITT G3: no output produced".to_string()));
    }

    Ok(CcittDecoded {
        data: out,
        rows_decoded: decoded_rows,
        recovered_partial: recovered,
    })
}

fn packed_output_buffer(bytes_per_row: usize, rows: Option<u32>) -> Result<Vec<u8>> {
    let Some(rows) = rows else {
        return Ok(Vec::new());
    };
    let rows =
        usize::try_from(rows).map_err(|_| Error::Decode("CCITT row count exceeds platform limits".to_string()))?;
    let capacity = bytes_per_row
        .checked_mul(rows)
        .ok_or_else(|| Error::Decode("CCITT packed output size overflow".to_string()))?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(capacity)
        .map_err(|_| Error::Decode(format!("Unable to allocate {capacity} bytes for CCITT packed output")))?;
    Ok(output)
}

fn transition_buffer(width: u16) -> Result<Vec<u16>> {
    let capacity = width as usize;
    let mut transitions = Vec::new();
    transitions
        .try_reserve_exact(capacity)
        .map_err(|_| Error::Decode(format!("Unable to allocate {capacity} CCITT row transitions")))?;
    Ok(transitions)
}

/// Pad `out` with white rows until it holds `rows` (a no-op if `rows` is `None`, meaning the
/// declared height is unknown). Shared by [`decode`] and [`decode_g3`], which both decode fewer
/// rows than `/Rows` declares on a truncated/damaged stream and must fill the remainder.
fn pad_to_declared_rows(out: &mut Vec<u8>, bytes_per_row: usize, width: u16, rows: Option<u32>) -> Result<()> {
    let Some(rows) = rows else {
        return Ok(());
    };
    let white_row = transitions_to_bytes::<u16>(&[], width as usize)?;
    while out.len() / bytes_per_row.max(1) < rows as usize {
        out.try_reserve(white_row.len()).map_err(|_| {
            Error::Decode(format!(
                "Unable to grow CCITT output by {} padding bytes",
                white_row.len()
            ))
        })?;
        out.extend_from_slice(&white_row);
    }
    Ok(())
}

/// Determine whether the next Group 3 row uses 1-D or 2-D coding: always 1-D
/// when the stream isn't mixed (`K <= 0`), otherwise read the tag bit that
/// follows the row's EOL (`1` = 1-D, `0` = 2-D-relative). `None` means the
/// stream ended before the tag bit could be read.
fn decide_one_d(reader: &mut BitReader, two_dimensional: bool) -> Option<bool> {
    if !two_dimensional {
        return Some(true);
    }
    let b = reader.peek(1)?;
    reader.consume(1);
    Some(b == 1)
}

/// Decode one Group 3 1-D (Modified Huffman) row: runs alternate white→black
/// from the left edge, each a make-up+terminating MH code. Pushes color-flip
/// columns (first run white) into `current`, matching the G4 row contract so the
/// shared `transitions_to_bytes` packs it identically.
fn decode_row_g3_1d(reader: &mut BitReader, width: u16, current: &mut Vec<u16>) -> RowStatus {
    let mut a0: u16 = 0;
    let mut white = true;
    let mut any = false;
    while a0 < width {
        let run = match read_run(reader, white) {
            Some(v) => v,
            // The run tables can't read an EOL/EOFB or fill: a clean line ends
            // here if we already placed a run, otherwise the row is undecodable. ~keep
            None => return if any { RowStatus::Ok } else { RowStatus::Error },
        };
        let a1 = match a0.checked_add(run) {
            Some(v) => v,
            None => return RowStatus::Error,
        };
        any = true;
        if a1 >= width {
            break;
        }
        if !try_push_transition(current, a1) {
            return RowStatus::AllocationFailed;
        }
        a0 = a1;
        white = !white;
    }
    RowStatus::Ok
}

/// Consume a maximal run of EOL codes at the cursor (`>= 11` zero bits — fill
/// included — followed by a `1`), returning how many were eaten. Leaves the
/// reader untouched when the next bits are not an EOL; when a trailing fill of
/// `>= 11` zeros runs into the end of data it is treated as a terminator and
/// the reader is left at EOD.
fn skip_eols(reader: &mut BitReader) -> usize {
    let mut count = 0usize;
    loop {
        let save = reader.bit;
        let mut zeros = 0u32;
        loop {
            match reader.peek(1) {
                Some(0) => {
                    reader.consume(1);
                    zeros += 1;
                }
                Some(_) => break,
                None => {
                    if zeros < 11 {
                        reader.bit = save;
                    }
                    return count;
                }
            }
        }
        if zeros >= 11 {
            reader.consume(1);
            count += 1;
        } else {
            reader.bit = save;
            return count;
        }
    }
}

// ---------------------------------------------------------------------------
// Bit reader (MSB-first over a byte slice)
// --------------------------------------------------------------------------- ~keep

struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, bit: 0 }
    }

    /// Peek the next `n` bits (≤16) MSB-first, or `None` if fewer remain.
    fn peek(&self, n: u8) -> Option<u16> {
        if n == 0 {
            return Some(0);
        }
        if n > 16 || self.bit + n as usize > self.data.len() * 8 {
            return None;
        }
        let mut v: u16 = 0;
        for i in 0..n as usize {
            let bp = self.bit + i;
            let bit = (self.data[bp / 8] >> (7 - (bp % 8))) & 1;
            v = (v << 1) | bit as u16;
        }
        Some(v)
    }

    fn consume(&mut self, n: u8) {
        self.bit += n as usize;
    }

    fn bits_to_byte_boundary(&self) -> u8 {
        ((8 - (self.bit % 8)) % 8) as u8
    }

    fn eod(&self) -> bool {
        self.bit >= self.data.len() * 8
    }
}

/// Skip zero fill to the next byte boundary; disable byte-align if a non-zero
/// fill bit shows the flag is mis-declared for this stream (pdfium behavior).
fn byte_align_skip(reader: &mut BitReader, active: &mut bool) {
    if !*active {
        return;
    }
    let pad = reader.bits_to_byte_boundary();
    for _ in 0..pad {
        match reader.peek(1) {
            Some(0) => reader.consume(1),
            Some(_) => {
                *active = false;
                return;
            }
            None => return,
        }
    }
}

#[derive(Copy, Clone)]
enum Mode {
    Pass,
    Horizontal,
    Vertical(i8),
    Extension,
    Eof,
}

/// T.6 2D mode prefix codes (prefix-free, shortest first). Verified against
/// ITU-T T.6 and the `fax` crate tables.
static MODE_CODES: &[(u8, u16, Mode)] = &[
    (1, 0b1, Mode::Vertical(0)),
    (3, 0b001, Mode::Horizontal),
    (3, 0b010, Mode::Vertical(-1)),
    (3, 0b011, Mode::Vertical(1)),
    (4, 0b0001, Mode::Pass),
    (6, 0b000010, Mode::Vertical(-2)),
    (6, 0b000011, Mode::Vertical(2)),
    (7, 0b0000001, Mode::Extension),
    (7, 0b0000010, Mode::Vertical(-3)),
    (7, 0b0000011, Mode::Vertical(3)),
    (12, 0b0000_0000_0001, Mode::Eof),
];

fn read_mode(reader: &mut BitReader) -> Option<Mode> {
    for &(len, code, mode) in MODE_CODES {
        if reader.peek(len) == Some(code) {
            reader.consume(len);
            return Some(mode);
        }
    }
    None
}

/// Read one Modified-Huffman run length for the given color (accumulating
/// make-up codes until a terminating code `< 64`).
fn read_run(reader: &mut BitReader, white: bool) -> Option<u16> {
    let table = if white { WHITE_CODES } else { BLACK_CODES };
    let mut sum: u16 = 0;
    loop {
        let mut n = None;
        for &(len, code, val) in table {
            if reader.peek(len) == Some(code) {
                reader.consume(len);
                n = Some(val);
                break;
            }
        }
        let n = n?;
        sum = sum.checked_add(n)?;
        if n < 64 {
            return Some(sum);
        }
    }
}

// ---------------------------------------------------------------------------
// Reference-line changing-element walk (ported from the verified `fax` crate)
// --------------------------------------------------------------------------- ~keep

/// `edges[k]` is a color flip on the reference line; the run before `edges[0]`
/// is white, so `edges[k]` starts black when `k` is even, white when odd.
/// Find the first reference edge right of `start` whose run color is `want_black`
/// (the color opposite the current coding color). Advances `pos` past it.
fn next_color(edges: &[u16], pos: &mut usize, start: u16, want_black: bool, start_of_row: bool) -> Option<u16> {
    if start_of_row {
        if want_black {
            *pos = 1;
            return edges.first().copied();
        }
        *pos = 2;
        return edges.get(1).copied();
    }
    while *pos < edges.len() {
        if edges[*pos] <= start {
            *pos += 1;
            continue;
        }
        if (*pos).is_multiple_of(2) != want_black {
            *pos += 1;
        }
        break;
    }
    if *pos < edges.len() {
        let v = edges[*pos];
        *pos += 1;
        Some(v)
    } else {
        None
    }
}

fn ref_next(edges: &[u16], pos: &mut usize) -> Option<u16> {
    if *pos < edges.len() {
        let v = edges[*pos];
        *pos += 1;
        Some(v)
    } else {
        None
    }
}

fn seek_back(edges: &[u16], pos: &mut usize, start: u16) {
    *pos = (*pos).min(edges.len().saturating_sub(1));
    while *pos > 0 && start < edges[*pos - 1] {
        *pos -= 1;
    }
}

enum RowStatus {
    Ok,
    EndOfBlock,
    Error,
    AllocationFailed,
}

fn try_push_transition(transitions: &mut Vec<u16>, position: u16) -> bool {
    if transitions.try_reserve(1).is_err() {
        return false;
    }
    transitions.push(position);
    true
}

/// Decode one G4 (T.6) row relative to `reference`, pushing color-flip column
/// positions (first run white) into `current`. Ported from the verified `fax`
/// crate `Group4Decoder::advance`.
fn decode_row_g4<const STRICT: bool>(
    reader: &mut BitReader,
    reference: &[u16],
    width: u16,
    current: &mut Vec<u16>,
) -> RowStatus {
    let mut pos = 0usize;
    let mut a0: u16 = 0;
    let mut black = false;
    let mut start_of_row = true;

    loop {
        let mode = match read_mode(reader) {
            Some(m) => m,
            None => return RowStatus::Error,
        };
        match mode {
            Mode::Pass => {
                if start_of_row && !black {
                    pos += 1;
                } else if next_color(reference, &mut pos, a0, !black, false).is_none() {
                    return RowStatus::Error;
                }
                match ref_next(reference, &mut pos) {
                    Some(b2) if !STRICT || b2 > a0 => a0 = b2,
                    _ if STRICT => return RowStatus::Error,
                    _ => {}
                }
            }
            Mode::Vertical(delta) => {
                let b1 = next_color(reference, &mut pos, a0, !black, start_of_row).unwrap_or(width);
                let a1i = b1 as i32 + delta as i32;
                if a1i < 0 || a1i > width as i32 {
                    if STRICT {
                        return RowStatus::Error;
                    }
                    break;
                }
                let a1 = a1i as u16;
                if STRICT && !advances_at_row_edge(a0, a1, start_of_row) {
                    return RowStatus::Error;
                }
                if a1 < width && !try_push_transition(current, a1) {
                    return RowStatus::AllocationFailed;
                }
                black = !black;
                a0 = a1;
                if delta < 0 {
                    seek_back(reference, &mut pos, a0);
                }
            }
            Mode::Horizontal => match decode_horizontal::<STRICT>(reader, current, width, black, a0, start_of_row) {
                HorizontalStep::Advance(new_a0) => a0 = new_a0,
                HorizontalStep::EndOfRow => break,
                HorizontalStep::Status(status) => return status,
            },
            Mode::Extension => return RowStatus::Error,
            Mode::Eof => return RowStatus::EndOfBlock,
        }
        start_of_row = false;
        if a0 >= width {
            break;
        }
    }
    RowStatus::Ok
}

fn advances_at_row_edge(previous: u16, next: u16, start_of_row: bool) -> bool {
    next > previous || (start_of_row && next == previous)
}

/// Outcome of decoding one Horizontal-mode code pair within a G4/G3 row.
enum HorizontalStep {
    /// Row continues; `a0` advances to this position.
    Advance(u16),
    /// The second run reached or passed the right edge — the row is complete.
    EndOfRow,
    /// A terminal row failure (bad run code or buffer growth failure).
    Status(RowStatus),
}

/// T.6/T.4 Horizontal mode: read two runs (opposite-color-first) and push up
/// to two transition columns. Split out of [`decode_row_g4`] purely to bring
/// that function's cyclomatic complexity under the repo's linter cap; the
/// logic is unchanged from the inline `Mode::Horizontal` arm.
fn decode_horizontal<const STRICT: bool>(
    reader: &mut BitReader,
    current: &mut Vec<u16>,
    width: u16,
    black: bool,
    a0: u16,
    start_of_row: bool,
) -> HorizontalStep {
    let r1 = match read_run(reader, !black) {
        Some(v) => v,
        None => return HorizontalStep::Status(RowStatus::Error),
    };
    let r2 = match read_run(reader, black) {
        Some(v) => v,
        None => return HorizontalStep::Status(RowStatus::Error),
    };
    let a1 = match a0.checked_add(r1) {
        Some(v) => v,
        None => return HorizontalStep::Status(RowStatus::Error),
    };
    let a2 = match a1.checked_add(r2) {
        Some(v) => v,
        None => return HorizontalStep::Status(RowStatus::Error),
    };
    if STRICT && (a2 > width || (r1 == 0 && !start_of_row) || (r2 == 0 && a1 != width)) {
        return HorizontalStep::Status(RowStatus::Error);
    }
    if a1 < width && !try_push_transition(current, a1) {
        return HorizontalStep::Status(RowStatus::AllocationFailed);
    }
    if a2 >= width {
        return HorizontalStep::EndOfRow;
    }
    if !try_push_transition(current, a2) {
        return HorizontalStep::Status(RowStatus::AllocationFailed);
    }
    HorizontalStep::Advance(a2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ccitt_decode_passthrough() {
        let decoder = CcittFaxDecoder;
        let ccitt_data = b"\x00\x01\x02\x03";
        assert_eq!(decoder.decode(ccitt_data, 0).unwrap(), ccitt_data);
    }

    #[test]
    fn test_ccitt_decoder_name() {
        assert_eq!(CcittFaxDecoder.name(), "CCITTFaxDecode");
    }

    #[test]
    fn tables_are_prefix_free() {
        for table in [WHITE_CODES, BLACK_CODES] {
            for &(la, ca, _) in table {
                for &(lb, cb, _) in table {
                    if lb > la && (cb >> (lb - la)) == ca {
                        panic!("non-prefix-free pair: ({la},{ca:b}) vs ({lb},{cb:b})");
                    }
                }
            }
        }
    }

    fn p(params: CcittParams, bits: &str) -> Result<CcittDecoded> {
        decode(&pack_bits(bits), &params)
    }

    fn pack_bits(bits: &str) -> Vec<u8> {
        // Pack an MSB-first bit string into bytes (zero-padded to a byte). ~keep
        let mut bytes = Vec::new();
        let mut acc = 0u8;
        let mut n = 0u8;
        for ch in bits.chars().filter(|c| *c == '0' || *c == '1') {
            acc = (acc << 1) | (ch == '1') as u8;
            n += 1;
            if n == 8 {
                bytes.push(acc);
                acc = 0;
                n = 0;
            }
        }
        if n > 0 {
            bytes.push(acc << (8 - n));
        }
        bytes
    }

    #[test]
    fn strict_group4_requires_exact_rows_trailer_and_padding() {
        let valid = pack_bits("001 1000 11 1 000000000001 000000000001");
        assert_eq!(decode_complete_group4(&valid, 8, 1, 1024).unwrap(), [0x18]);
        assert!(decode_complete_group4(&valid, 8, 2, 1024).is_err());
        assert!(decode_complete_group4(&valid, 8, 1, 32).is_err());
        assert!(decode_complete_group4(&valid[..valid.len() - 1], 8, 1, 1024).is_err());
        let mut nonzero_padding = valid.clone();
        *nonzero_padding.last_mut().unwrap() |= 1;
        assert!(decode_complete_group4(&nonzero_padding, 8, 1, 1024).is_err());
        let mut extra = valid;
        extra.push(0);
        assert!(decode_complete_group4(&extra, 8, 1, 1024).is_err());
    }

    #[test]
    fn strict_group4_rejects_rows_the_renderer_can_recover() {
        let params = CcittParams {
            k: -1,
            columns: 8,
            rows: Some(1),
            ..Default::default()
        };
        for row in ["011", "001 10100 010", "001 00110101 0000110111 1"] {
            let stream = pack_bits(&format!("{row} 000000000001 000000000001"));
            let recovered = decode(&stream, &params).unwrap();
            assert_eq!(recovered.rows_decoded, 1);
            assert!(decode_complete_group4(&stream, 8, 1, 1024).is_err(), "{row}");
        }
    }

    #[test]
    fn g4_all_white_row_v0() {
        // 8-wide, 1 row. V0 against the imaginary white ref → all-white row. ~keep
        let params = CcittParams {
            k: -1,
            columns: 8,
            rows: Some(1),
            ..Default::default()
        };
        let d = p(params, "1").unwrap();
        assert_eq!(d.rows_decoded, 1);
        assert_eq!(d.data, vec![0u8]);
        assert!(!d.recovered_partial);
    }

    #[test]
    fn g4_horizontal_white3_black2() {
        // 8-wide row: Horizontal (001) + white run 3 (1000) + black run 2 (11)
        // ⇒ transitions [3,5], leaving a0=5; a trailing V0 (1) extends the final
        // white run to the right edge, completing the row. ~keep
        let params = CcittParams {
            k: -1,
            columns: 8,
            rows: Some(1),
            ..Default::default()
        };
        let d = p(params, "001 1000 11 1").unwrap();
        // pixels: white[0..3) black[3..5) white[5..8) ⇒ 0b00011000 = 0x18 ~keep
        assert_eq!(d.data, vec![0b0001_1000]);
    }

    #[test]
    fn g4_negative_k_with_unknown_rows_grows_output_fallibly() {
        let params = CcittParams {
            k: -2,
            columns: 8,
            rows: None,
            ..Default::default()
        };
        let d = p(params, "001 1000 11 1 000000000001").unwrap();
        assert_eq!(d.rows_decoded, 1);
        assert_eq!(d.data, vec![0b0001_1000]);
        assert!(!d.recovered_partial);
    }

    #[test]
    fn g4_encoded_byte_align() {
        // Two all-white rows, each coded as a single V0 (`1`). Row 0's 1-bit
        // code is byte-padded, so the stream is [0x80, 0x80]. WITH
        // /EncodedByteAlign both rows decode; WITHOUT it the fill zeros after
        // row 0 mis-read as an invalid mode and the 2nd row is unrecoverable —
        // exactly a real-world fax-scanner failure mode. ~keep
        let aligned = CcittParams {
            k: -1,
            columns: 8,
            rows: Some(2),
            encoded_byte_align: true,
            ..Default::default()
        };
        let d = decode(&[0x80, 0x80], &aligned).unwrap();
        assert_eq!(d.rows_decoded, 2);
        assert_eq!(d.data, vec![0u8, 0u8]);
        assert!(!d.recovered_partial);

        let unaligned = CcittParams {
            encoded_byte_align: false,
            ..aligned
        };
        let d2 = decode(&[0x80, 0x80], &unaligned).unwrap();
        // Without alignment the 2nd row can't be found → 1 row decoded, the rest
        // recovered (white-padded) — NOT a silently-blank full page. ~keep
        assert_eq!(d2.rows_decoded, 1);
        assert!(d2.recovered_partial);
    }

    #[test]
    fn g4_zero_rows_errs_not_white() {
        // Garbage that cannot start a row → Err, NOT an all-white Ok buffer. ~keep
        let params = CcittParams {
            k: -1,
            columns: 8,
            rows: Some(4),
            ..Default::default()
        };
        let d = p(params, "000000000001");
        // EOFB on the very first read → EndOfBlock with 0 rows → padded white.
        // That is a legitimately-blank scan, so it returns Ok(all white). A
        // truly undecodable stream errors instead: ~keep
        assert!(d.is_ok());
        let bad = CcittParams {
            k: -1,
            columns: 8,
            rows: Some(4),
            ..Default::default()
        };
        // 0b0000001 = Extension as the very first mode → Error, 0 rows → Err. ~keep
        assert!(p(bad, "0000001").is_err());
    }

    #[test]
    fn g3_1d_all_white_row() {
        // 8-wide K=0 row: a single white run of 8 (MH code 10011). ~keep
        let params = CcittParams {
            k: 0,
            columns: 8,
            rows: Some(1),
            ..Default::default()
        };
        let d = p(params, "10011").unwrap();
        assert_eq!(d.rows_decoded, 1);
        assert_eq!(d.data, vec![0u8]);
        assert!(!d.recovered_partial);
    }

    #[test]
    fn g3_1d_white3_black2() {
        // 8-wide K=0 row: white run 3 (1000) + black run 2 (11) + white run 3
        // (1000) ⇒ transitions [3,5] ⇒ 0b0001_1000, identical to the G4 case. ~keep
        let params = CcittParams {
            k: 0,
            columns: 8,
            rows: Some(1),
            ..Default::default()
        };
        let d = p(params, "1000 11 1000").unwrap();
        assert_eq!(d.data, vec![0b0001_1000]);
    }

    #[test]
    fn g3_1d_eol_delimited_rows() {
        // Two all-white rows, each preceded by an EOL (000000000001) as a real
        // T.4 stream emits. The EOLs must be swallowed, not mis-read as runs. ~keep
        let params = CcittParams {
            k: 0,
            columns: 8,
            rows: Some(2),
            end_of_line: true,
            ..Default::default()
        };
        let d = p(params, "000000000001 10011 000000000001 10011").unwrap();
        assert_eq!(d.rows_decoded, 2);
        assert_eq!(d.data, vec![0u8, 0u8]);
        assert!(!d.recovered_partial);
    }

    #[test]
    fn g3_1d_rtc_terminates() {
        // One white row then a return-to-control (6 EOLs) ⇒ stop cleanly; the
        // declared 3rd/4th rows are white-padded, not reported as decoded. ~keep
        let params = CcittParams {
            k: 0,
            columns: 8,
            rows: Some(4),
            ..Default::default()
        };
        let rtc = "000000000001".repeat(6);
        let d = p(params, &format!("10011 {rtc}")).unwrap();
        assert_eq!(d.rows_decoded, 1);
        assert_eq!(d.data, vec![0u8; 4]);
    }

    // Real libtiff output for one 128×64 bilevel image, encoded both ways. The
    // G4 path is already trusted, so decoding the G3 (K=0) stream to the *same*
    // bitmap proves the new Modified-Huffman path — this is the codestream shape
    // behind the blank-page reports (K=0 / Group 3 1-D), which the in-house
    // decoder previously rejected outright. ~keep
    const G3_1D_STREAM: &[u8] = &[
        0, 17, 192, 240, 103, 0, 24, 3, 193, 104, 0, 77, 120, 3, 192, 172, 0, 77, 92, 1, 224, 176, 0, 38, 165, 0, 120,
        19, 128, 9, 168, 176, 7, 129, 184, 0, 154, 132, 128, 60, 20, 192, 4, 212, 60, 1, 224, 202, 0, 38, 162, 137, 1,
        224, 160, 0, 77, 69, 18, 3, 193, 64, 0, 154, 138, 36, 7, 130, 128, 1, 53, 20, 72, 15, 5, 0, 2, 106, 40, 144,
        30, 10, 0, 4, 212, 81, 32, 60, 20, 0, 9, 168, 162, 64, 120, 40, 0, 19, 81, 68, 128, 240, 80, 0, 38, 162, 137,
        1, 224, 160, 0, 77, 69, 18, 3, 193, 64, 0, 154, 138, 36, 7, 130, 128, 1, 53, 20, 72, 15, 5, 0, 2, 106, 40, 144,
        30, 10, 0, 4, 212, 81, 64, 60, 54, 0, 9, 168, 162, 74, 5, 138, 0, 252, 0, 77, 69, 18, 160, 199, 221, 15, 14, 7,
        160, 1, 53, 20, 72, 227, 29, 142, 99, 129, 232, 0, 77, 69, 18, 29, 138, 224, 126, 0, 38, 162, 137, 9, 10, 12,
        112, 61, 0, 9, 168, 162, 65, 45, 14, 135, 135, 3, 208, 0, 154, 138, 36, 20, 117, 8, 120, 112, 61, 0, 9, 168,
        162, 65, 97, 112, 31, 128, 9, 168, 162, 64, 196, 1, 240, 0, 154, 138, 36, 25, 224, 15, 64, 2, 106, 40, 144,
        108, 176, 102, 0, 19, 81, 68, 128, 188, 1, 96, 0, 154, 138, 36, 25, 80, 11, 32, 2, 106, 40, 144, 102, 64, 20,
        0, 9, 168, 162, 64, 209, 0, 112, 0, 38, 162, 137, 3, 84, 1, 32, 0, 154, 138, 36, 26, 80, 10, 64, 2, 106, 40,
        144, 106, 64, 50, 0, 9, 168, 162, 65, 173, 0, 172, 0, 38, 160, 120, 49, 0, 168, 0, 38, 160, 120, 103, 128, 218,
        0, 19, 80, 60, 54, 64, 54, 0, 9, 168, 30, 10, 32, 53, 128, 4, 212, 15, 3, 16, 26, 128, 2, 106, 7, 134, 92, 6,
        144, 0, 154, 129, 225, 155, 0, 212, 0, 38, 160, 120, 52, 192, 52, 0, 9, 168, 30, 13, 112, 25, 128, 2, 106, 7,
        134, 156, 6, 80, 0, 154, 129, 225, 171, 0, 92, 0, 77, 64, 240, 215, 128, 110, 0, 38, 160, 120, 54, 192, 104, 0,
        19, 80, 60, 54, 224, 8, 0, 19, 80, 60, 21, 96, 23, 0, 19, 80, 60, 21, 224, 28, 0, 77, 64, 240, 101, 128, 224,
        2, 106, 7, 130, 156, 4, 0, 19, 80, 60, 13, 224, 80, 1, 53, 3, 192, 158, 8, 0, 77, 64, 240, 88, 134, 0, 38, 160,
        120, 21, 198, 0, 38, 160, 120, 45, 64,
    ];
    const G4_STREAM: &[u8] = &[
        35, 129, 224, 206, 28, 17, 227, 12, 60, 48, 240, 195, 195, 15, 12, 60, 138, 37, 255, 255, 255, 255, 255, 255,
        195, 225, 21, 4, 88, 52, 120, 97, 160, 71, 116, 8, 195, 248, 97, 164, 227, 248, 99, 16, 151, 134, 202, 128,
        200, 224, 122, 236, 66, 40, 114, 135, 253, 132, 16, 255, 226, 16, 66, 15, 196, 53, 225, 135, 225, 131, 240, 97,
        248, 97, 248, 97, 248, 97, 248, 97, 248, 97, 248, 97, 248, 97, 226, 24, 120, 97, 225, 135, 134, 30, 24, 120,
        97, 225, 135, 134, 30, 24, 120, 97, 225, 135, 134, 30, 24, 120, 97, 225, 135, 134, 30, 24, 120, 97, 225, 135,
        134, 30, 24, 120, 97, 225, 134, 0, 32, 2,
    ];

    #[test]
    fn g3_1d_matches_g4_for_same_image() {
        let g3 = decode(
            G3_1D_STREAM,
            &CcittParams {
                k: 0,
                columns: 128,
                rows: Some(64),
                end_of_line: true,
                ..Default::default()
            },
        )
        .unwrap();
        let g4 = decode(
            G4_STREAM,
            &CcittParams {
                k: -1,
                columns: 128,
                rows: Some(64),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(g3.rows_decoded, 64);
        assert!(!g3.recovered_partial);
        assert_eq!(g3.data.len(), 16 * 64);
        // Both encode the identical bitmap; the trusted G4 path is the oracle. ~keep
        assert_eq!(g3.data, g4.data);
        // And it is not a degenerate all-white decode. ~keep
        assert!(g3.data.iter().any(|&b| b != 0));
    }

    #[test]
    fn packed_output_buffer_rejects_size_overflow() {
        let result = packed_output_buffer(usize::MAX, Some(2));
        assert!(matches!(result, Err(Error::Decode(message)) if message.contains("size overflow")));
    }
}
