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

use xberg_native_pdf::geometry::Rect;
use xberg_native_pdf::layout::{Color, TextSpan};

/// Upper bound on the memo's estimated footprint, in bytes.
const MEMO_MAX_BYTES: usize = 32 * 1024 * 1024;
/// Memo bytes allowed per source byte, so a small document never holds the full cap.
const MEMO_BYTES_PER_SOURCE_BYTE: usize = 32;

struct OrderedPage {
    page_width: u32,
    page_height: u32,
    input: SpansFingerprint,
    ordered: Vec<TextSpan>,
    bytes: usize,
}

struct Memo {
    budget: usize,
    used: usize,
    pages: HashMap<usize, OrderedPage>,
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
            return (None, None);
        };
        let stored = memo.pages.get(&page_index);
        let admits = memo.used < memo.budget;
        let input = (stored.is_some() || admits).then(|| fingerprint(spans));
        let served = stored
            .filter(|page| page.page_width == width && page.page_height == height && Some(page.input) == input)
            .map(|page| page.ordered.clone());
        (served, input.filter(|_| admits))
    });
    if let Some(ordered) = served {
        *spans = ordered;
        return;
    }
    order(spans);
    #[cfg(test)]
    COMPUTED_ORDERS.with(|count| count.set(count.get() + 1));
    let Some(input) = input else {
        return;
    };
    let bytes = size_of::<SpansFingerprint>() + spans_bytes(spans);
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
                ordered: spans.clone(),
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
                + span.font_name.capacity()
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

pub(super) fn fingerprint(spans: &[TextSpan]) -> SpansFingerprint {
    let mut hasher = blake3::Hasher::new();
    for span in spans {
        hash_span(&mut hasher, span);
    }
    let mut digest = [0; 16];
    digest.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    SpansFingerprint {
        count: spans.len(),
        digest,
    }
}

fn hash_span(hasher: &mut blake3::Hasher, span: &TextSpan) {
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
    put_str(hasher, text);
    put_floats(hasher, &[x, y, width, height]);
    put_str(hasher, font_name);
    put_floats(hasher, &[*font_size]);
    hasher.update(&(*font_weight as u16).to_le_bytes());
    put_flags(hasher, &[*is_italic, *is_monospace]);
    put_floats(hasher, &[r, g, b]);
    put_option(hasher, mcid.as_ref());
    put_str(hasher, &format!("{mcid_scope:?}"));
    hasher.update(&(*sequence as u64).to_le_bytes());
    put_flags(hasher, &[*split_boundary_before, *offset_semantic]);
    put_floats(hasher, &[*char_spacing, *word_spacing, *horizontal_scaling]);
    put_flags(hasher, &[*primary_detected]);
    put_str(hasher, &format!("{artifact_type:?}"));
    put_floats(hasher, char_widths);
    put_floats(hasher, char_x_offsets);
    put_option(hasher, heading_level.as_ref());
    put_floats(hasher, &[*rotation_degrees]);
    hasher.update(&[*wmode]);
    put_floats(hasher, &[*text_rise]);
    put_flags(hasher, &[*rtl_draw_logical, *mirrored]);
    hasher.update(&page_rotation_applied.to_le_bytes());
    put_str(hasher, &format!("{provenance:?}"));
    put_flags(hasher, &[*ligature_glyph]);
}

/// Length-prefixed, so adjacent variable-length fields cannot shift bytes between each other.
fn put_str(hasher: &mut blake3::Hasher, value: &str) {
    hasher.update(&(value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn put_floats(hasher: &mut blake3::Hasher, values: &[f32]) {
    hasher.update(&(values.len() as u64).to_le_bytes());
    for value in values {
        hasher.update(&value.to_bits().to_le_bytes());
    }
}

fn put_flags(hasher: &mut blake3::Hasher, flags: &[bool]) {
    for flag in flags {
        hasher.update(&[u8::from(*flag)]);
    }
}

fn put_option<T: Copy + Into<u64>>(hasher: &mut blake3::Hasher, value: Option<&T>) {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            let wide: u64 = (*value).into();
            hasher.update(&wide.to_le_bytes());
        }
        None => {
            hasher.update(&[0]);
        }
    }
}

#[cfg(test)]
mod tests;
