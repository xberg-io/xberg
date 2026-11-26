//! Per-extraction memo of page reading orders.
//!
//! One extraction orders each page's spans up to three times: in the text pass, for the heuristic
//! tables (`ScriptGeometry::Flatten`) and for the structure pass (`ScriptGeometry::Preserve`). On a
//! page without raised marks, off-page text or RTL structure order all three hand
//! `reorder_page_reading_order` the same spans. The ordering reads nothing but the spans, the page
//! size and the page index, so a later call is served the stored order only when its spans hash to
//! the stored input's fingerprint (every field, floats bit for bit) at the same page size. Outside a [`ReadingOrderMemoScope`] every call orders its spans afresh.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasher, RandomState};

use xberg_native_pdf::extractors::text::{ArtifactType, PaginationSubtype};
use xberg_native_pdf::fonts::MappingProvenance;
use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::{Color, TextSpan};
use xberg_native_pdf::object::ObjectRef;
use xberg_native_pdf::structure::McidScope;

/// Upper bound on the memo's estimated footprint, in bytes.
const MEMO_MAX_BYTES: usize = 32 * 1024 * 1024;
/// Memo bytes allowed per source byte, so a small document never holds the full cap.
const MEMO_BYTES_PER_SOURCE_BYTE: usize = 32;

struct OrderedPage {
    page_width: u32,
    page_height: u32,
    input: SpansFingerprint,
    ordered: StoredOrder,
    bytes: usize,
}

/// A page's computed order: the output position of each input span when the ordering only
/// rearranged them, else the ordered spans themselves.
enum StoredOrder {
    Positions(Vec<u32>),
    Spans(Vec<TextSpan>),
}

impl StoredOrder {
    fn bytes(&self) -> usize {
        match self {
            Self::Positions(positions) => positions.len() * size_of::<u32>(),
            Self::Spans(spans) => spans_bytes(spans),
        }
    }

    /// Replace `spans`, an input equal to the stored one, with its stored order.
    fn serve(&self, spans: &mut Vec<TextSpan>) {
        match self {
            Self::Positions(positions) => {
                let mut placed: Vec<(u32, TextSpan)> = positions.iter().copied().zip(spans.drain(..)).collect();
                placed.sort_unstable_by_key(|(position, _)| *position);
                spans.extend(placed.into_iter().map(|(_, span)| span));
            }
            Self::Spans(ordered) => spans.clone_from(ordered),
        }
    }
}

struct Memo {
    budget: usize,
    used: usize,
    pages: HashMap<usize, OrderedPage>,
    /// Keys the hash that buckets spans when matching ordered spans back to their inputs; the
    /// match itself compares encodings. Random per memo, so crafted content cannot fill one bucket.
    span_keys: RandomState,
}

thread_local! {
    static MEMO: RefCell<Option<Memo>> = const { RefCell::new(None) };
}

#[cfg(test)]
thread_local! {
    static COMPUTED_ORDERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Page orders computed (not served from the memo) on this thread so far.
#[cfg(test)]
pub(super) fn computed_orders() -> usize {
    COMPUTED_ORDERS.with(std::cell::Cell::get)
}

/// Keeps page reading orders on this thread until dropped. The text, table and structure passes
/// of one extraction run on the calling thread, so they share one memo; a nested scope gets its
/// own and restores the outer one when it ends.
#[must_use = "the memo lives only while the scope is held"]
pub(crate) struct ReadingOrderMemoScope {
    previous: Option<Memo>,
}

impl ReadingOrderMemoScope {
    /// Start a memo holding at most an estimated `min(32 MiB, 32 x source_len)` bytes; pages past
    /// that are ordered on every call, as without it.
    pub(crate) fn begin(source_len: usize) -> Self {
        let budget = MEMO_MAX_BYTES.min(source_len.saturating_mul(MEMO_BYTES_PER_SOURCE_BYTE));
        let memo = Memo {
            budget,
            used: 0,
            pages: HashMap::new(),
            span_keys: RandomState::new(),
        };
        let previous = MEMO.with(|slot| slot.replace(Some(memo)));
        Self { previous }
    }
}

impl Drop for ReadingOrderMemoScope {
    fn drop(&mut self) {
        let previous = self.previous.take();
        let _ = MEMO.try_with(|slot| slot.replace(previous));
    }
}

/// Order `spans` with `order`, or serve the order stored for an identical input of this page.
pub(super) fn reorder(
    spans: &mut Vec<TextSpan>,
    page_width: f32,
    page_height: f32,
    page_index: usize,
    order: impl FnOnce(&mut Vec<TextSpan>),
) {
    let (width, height) = (page_width.to_bits(), page_height.to_bits());
    let (served, input) = MEMO.with(|slot| {
        let slot = slot.borrow();
        let Some(memo) = slot.as_ref() else {
            return (false, None);
        };
        let stored = memo
            .pages
            .get(&page_index)
            .filter(|page| page.page_width == width && page.page_height == height);
        let admits = memo.used < memo.budget;
        let input = (stored.is_some() || admits).then(|| fingerprint(spans));
        if let Some(page) = stored.filter(|page| Some(page.input) == input) {
            page.ordered.serve(spans);
            return (true, None);
        }
        (
            false,
            input.filter(|_| admits).map(|print| (print, memo.span_keys.clone())),
        )
    });
    if served {
        return;
    }
    // Encoded only on a page the memo will keep, so a served page pays for the fingerprint alone. ~keep
    let input_encodings = input.as_ref().map(|_| SpanEncodings::of(spans));
    order(spans);
    #[cfg(test)]
    COMPUTED_ORDERS.with(|count| count.set(count.get() + 1));
    let (Some((input, keys)), Some(input_encodings)) = (input, input_encodings) else {
        return;
    };
    let ordered = match output_positions(&input_encodings, &SpanEncodings::of(spans), |encoded| {
        keys.hash_one(encoded)
    }) {
        Some(positions) => StoredOrder::Positions(positions),
        None => StoredOrder::Spans(spans.clone()),
    };
    let bytes = size_of::<SpansFingerprint>() + ordered.bytes();
    MEMO.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(memo) = slot.as_mut() else {
            return;
        };
        let released = memo.pages.get(&page_index).map_or(0, |page| page.bytes);
        let used = memo.used - released + bytes;
        if used > memo.budget {
            return;
        }
        memo.used = used;
        memo.pages.insert(
            page_index,
            OrderedPage {
                page_width: width,
                page_height: height,
                input,
                ordered,
                bytes,
            },
        );
    });
}

fn spans_bytes(spans: &[TextSpan]) -> usize {
    spans
        .iter()
        .map(|span| {
            size_of::<TextSpan>()
                + span.text.capacity()
                + span.font_name.len()
                + (span.char_widths.capacity() + span.char_x_offsets.capacity()) * size_of::<f32>()
        })
        .sum()
}

/// Identity of a page's input spans: their count and the first 128 bits of a BLAKE3 digest over
/// every field, floats by bit pattern. A hit is served on equal fingerprints, so a false hit needs
/// a 128-bit collision against the one stored digest of that page (probability 2^-128 per
/// comparison); BLAKE3 is cryptographic, so span content taken from a PDF cannot be crafted to
/// reach one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct SpansFingerprint {
    count: usize,
    digest: [u8; 16],
}

/// Bytes buffered before they are handed to the hasher, so BLAKE3 sees large contiguous inputs.
const FLUSH_BYTES: usize = 64 * 1024;

struct Digest {
    hasher: blake3::Hasher,
    buf: Vec<u8>,
}

impl Digest {
    fn flush_if_full(&mut self) {
        if self.buf.len() >= FLUSH_BYTES {
            self.hasher.update(&self.buf);
            self.buf.clear();
        }
    }

    fn u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    fn flags(&mut self, flags: &[bool]) {
        self.buf.extend(flags.iter().map(|flag| u8::from(*flag)));
    }

    fn u32(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    /// Length-prefixed, so adjacent variable-length fields cannot shift bytes between each other.
    fn str(&mut self, value: &str) {
        self.u64(value.len() as u64);
        self.buf.extend_from_slice(value.as_bytes());
    }

    fn floats(&mut self, values: &[f32]) {
        self.u64(values.len() as u64);
        for value in values {
            self.u32(value.to_bits());
        }
    }

    fn object_ref(&mut self, value: ObjectRef) {
        self.u32(value.id);
        self.buf.extend_from_slice(&value.generation.to_le_bytes());
    }

    fn mcid_scope(&mut self, scope: Option<&McidScope>) {
        match scope {
            None => self.u8(0),
            Some(McidScope::Page(page)) => {
                self.u8(1);
                self.u32(*page);
            }
            Some(McidScope::Form(object)) => {
                self.u8(2);
                self.object_ref(*object);
            }
            Some(McidScope::Pattern(object)) => {
                self.u8(3);
                self.object_ref(*object);
            }
        }
    }

    fn artifact_type(&mut self, artifact: Option<&ArtifactType>) {
        self.u8(match artifact {
            None => 0,
            Some(ArtifactType::Layout) => 1,
            Some(ArtifactType::Page) => 2,
            Some(ArtifactType::Background) => 3,
            Some(ArtifactType::Pagination(PaginationSubtype::Header)) => 4,
            Some(ArtifactType::Pagination(PaginationSubtype::Footer)) => 5,
            Some(ArtifactType::Pagination(PaginationSubtype::Watermark)) => 6,
            Some(ArtifactType::Pagination(PaginationSubtype::PageNumber)) => 7,
            Some(ArtifactType::Pagination(PaginationSubtype::Other)) => 8,
        });
    }
}

pub(super) fn fingerprint(spans: &[TextSpan]) -> SpansFingerprint {
    let mut digest = Digest {
        hasher: blake3::Hasher::new(),
        buf: Vec::with_capacity(FLUSH_BYTES + 1024),
    };
    for span in spans {
        hash_span(&mut digest, span);
        digest.flush_if_full();
    }
    digest.hasher.update(&digest.buf);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest.hasher.finalize().as_bytes()[..16]);
    SpansFingerprint {
        count: spans.len(),
        digest: bytes,
    }
}

/// Each span's fingerprint encoding, which covers every field (floats by bit pattern), so two
/// spans encode to equal bytes exactly when they are equal.
pub(super) struct SpanEncodings {
    bytes: Vec<u8>,
    ends: Vec<usize>,
}

impl SpanEncodings {
    pub(super) fn of(spans: &[TextSpan]) -> Self {
        let mut digest = Digest {
            hasher: blake3::Hasher::new(),
            buf: Vec::new(),
        };
        let mut ends = Vec::with_capacity(spans.len());
        for span in spans {
            hash_span(&mut digest, span);
            ends.push(digest.buf.len());
        }
        Self {
            bytes: digest.buf,
            ends,
        }
    }

    fn len(&self) -> usize {
        self.ends.len()
    }

    fn get(&self, index: usize) -> &[u8] {
        let start = index.checked_sub(1).map_or(0, |previous| self.ends[previous]);
        &self.bytes[start..self.ends[index]]
    }
}

/// The output position of each input span, when the output is the input rearranged: every output
/// span claims a distinct input span with byte-equal encoding. `bucket` only narrows the search;
/// equality is always decided on the encodings. `None` when the ordering added, dropped or
/// changed a span, or a page has more spans than a position can name.
pub(super) fn output_positions(
    input: &SpanEncodings,
    output: &SpanEncodings,
    bucket: impl Fn(&[u8]) -> u64,
) -> Option<Vec<u32>> {
    if input.len() != output.len() || u32::try_from(input.len()).is_err() {
        return None;
    }
    let mut unclaimed: HashMap<u64, Vec<usize>> = HashMap::with_capacity(input.len());
    for index in 0..input.len() {
        unclaimed.entry(bucket(input.get(index))).or_default().push(index);
    }
    let mut positions = vec![0u32; input.len()];
    for position in 0..output.len() {
        let encoded = output.get(position);
        let candidates = unclaimed.get_mut(&bucket(encoded))?;
        let found = candidates.iter().position(|&index| input.get(index) == encoded)?;
        let index = candidates.remove(found);
        positions[index] = u32::try_from(position).ok()?;
    }
    Some(positions)
}

#[cfg(test)]
fn stored_positions(page_index: usize) -> Option<bool> {
    MEMO.with(|slot| {
        let slot = slot.borrow();
        let page = slot.as_ref()?.pages.get(&page_index)?;
        Some(matches!(page.ordered, StoredOrder::Positions(_)))
    })
}

fn hash_span(out: &mut Digest, span: &TextSpan) {
    // Destructured without `..`: a field added to `TextSpan` stops this compiling until it is
    // hashed, so the fingerprint keeps covering every input the ordering can read. ~keep
    let TextSpan {
        text,
        bbox,
        font_name,
        font_size,
        font_weight,
        is_italic,
        is_monospace,
        color,
        mcid,
        mcid_scope,
        sequence,
        split_boundary_before,
        offset_semantic,
        char_spacing,
        word_spacing,
        horizontal_scaling,
        primary_detected,
        artifact_type,
        char_widths,
        char_x_offsets,
        heading_level,
        rotation_degrees,
        wmode,
        text_rise,
        rtl_draw_logical,
        mirrored,
        page_rotation_applied,
        provenance,
        ligature_glyph,
    } = span;
    let Rect { x, y, width, height } = *bbox;
    let Color { r, g, b } = *color;
    out.str(text);
    out.floats(&[x, y, width, height]);
    out.str(font_name);
    out.floats(&[*font_size]);
    out.buf.extend_from_slice(&(*font_weight as u16).to_le_bytes());
    out.flags(&[*is_italic, *is_monospace]);
    out.floats(&[r, g, b]);
    match mcid {
        Some(value) => {
            out.u8(1);
            out.u32(*value);
        }
        None => out.u8(0),
    }
    out.mcid_scope(mcid_scope.as_ref());
    out.u64(*sequence as u64);
    out.flags(&[*split_boundary_before, *offset_semantic]);
    out.floats(&[*char_spacing, *word_spacing, *horizontal_scaling]);
    out.flags(&[*primary_detected]);
    out.artifact_type(artifact_type.as_ref());
    out.floats(char_widths);
    out.floats(char_x_offsets);
    match heading_level {
        Some(level) => {
            out.u8(1);
            out.u8(*level);
        }
        None => out.u8(0),
    }
    out.floats(&[*rotation_degrees]);
    out.u8(*wmode);
    out.floats(&[*text_rise]);
    out.flags(&[*rtl_draw_logical, *mirrored]);
    out.buf.extend_from_slice(&page_rotation_applied.to_le_bytes());
    out.u8(provenance.map_or(u8::MAX, MappingProvenance::rank));
    out.flags(&[*ligature_glyph]);
}

#[cfg(test)]
mod tests;
