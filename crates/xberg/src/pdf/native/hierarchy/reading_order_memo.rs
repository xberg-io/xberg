//! Per-extraction memo of page reading orders.
//!
//! One extraction orders each page's spans up to three times: in the text pass, for the heuristic
//! tables (`ScriptGeometry::Flatten`) and for the structure pass (`ScriptGeometry::Preserve`). On a
//! page without raised marks, off-page text or RTL structure order all three hand
//! `reorder_page_reading_order` the same spans. The ordering reads nothing but the spans, the page
//! size and the page index, so a later call is served the stored order only when its spans equal
//! the stored input field for field, bit for bit, at the same page size: equal inputs prove an
//! equal result. Outside a [`ReadingOrderMemoScope`] every call orders its spans afresh.

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
    input: Vec<TextSpan>,
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
    let (served, admits) = MEMO.with(|slot| {
        let slot = slot.borrow();
        let Some(memo) = slot.as_ref() else {
            return (None, false);
        };
        let served = memo
            .pages
            .get(&page_index)
            .filter(|page| page.page_width == width && page.page_height == height && same_spans(&page.input, spans))
            .map(|page| page.ordered.clone());
        (served, memo.used < memo.budget)
    });
    if let Some(ordered) = served {
        *spans = ordered;
        return;
    }
    let input = admits.then(|| spans.clone());
    order(spans);
    #[cfg(test)]
    COMPUTED_ORDERS.with(|count| count.set(count.get() + 1));
    let Some(input) = input else {
        return;
    };
    let bytes = spans_bytes(&input) + spans_bytes(spans);
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

pub(super) fn same_spans(left: &[TextSpan], right: &[TextSpan]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(left, right)| same_span(left, right))
}

fn same_span(left: &TextSpan, right: &TextSpan) -> bool {
    // Destructured without `..`: a field added to `TextSpan` stops this compiling until it is
    // compared, so equality keeps covering every input the ordering can read. ~keep
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
    } = left;
    [
        *text == right.text,
        same_rect(bbox, &right.bbox),
        *font_name == right.font_name,
        same_bits(*font_size, right.font_size),
        *font_weight == right.font_weight,
        *is_italic == right.is_italic,
        *is_monospace == right.is_monospace,
        same_color(color, &right.color),
        *mcid == right.mcid,
        *mcid_scope == right.mcid_scope,
        *sequence == right.sequence,
        *split_boundary_before == right.split_boundary_before,
        *offset_semantic == right.offset_semantic,
        same_bits(*char_spacing, right.char_spacing),
        same_bits(*word_spacing, right.word_spacing),
        same_bits(*horizontal_scaling, right.horizontal_scaling),
        *primary_detected == right.primary_detected,
        *artifact_type == right.artifact_type,
        same_floats(char_widths, &right.char_widths),
        same_floats(char_x_offsets, &right.char_x_offsets),
        *heading_level == right.heading_level,
        same_bits(*rotation_degrees, right.rotation_degrees),
        *wmode == right.wmode,
        same_bits(*text_rise, right.text_rise),
        *rtl_draw_logical == right.rtl_draw_logical,
        *mirrored == right.mirrored,
        *page_rotation_applied == right.page_rotation_applied,
        *provenance == right.provenance,
        *ligature_glyph == right.ligature_glyph,
    ]
    .into_iter()
    .all(std::convert::identity)
}

fn same_bits(left: f32, right: f32) -> bool {
    left.to_bits() == right.to_bits()
}

fn same_floats(left: &[f32], right: &[f32]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(left, right)| same_bits(*left, *right))
}

fn same_rect(left: &Rect, right: &Rect) -> bool {
    let Rect { x, y, width, height } = *left;
    same_bits(x, right.x) && same_bits(y, right.y) && same_bits(width, right.width) && same_bits(height, right.height)
}

fn same_color(left: &Color, right: &Color) -> bool {
    let Color { r, g, b } = *left;
    same_bits(r, right.r) && same_bits(g, right.g) && same_bits(b, right.b)
}

#[cfg(test)]
mod tests;
