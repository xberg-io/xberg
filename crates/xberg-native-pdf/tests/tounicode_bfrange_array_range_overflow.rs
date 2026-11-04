//! Regression test for a `ToUnicode` CMap `bfrange` ARRAY-form parser
//! underflow/overflow.
//!
//! The `fonts::cmap` bfrange parser's array branch used to compute
//! `let range_size = (end - start + 1) as usize;` directly from `start`/`end`,
//! which are `u32::from_str_radix` results parsed from attacker-controlled hex
//! in the font's `/ToUnicode` stream (ISO 32000-1:2008 §9.10.3). A reversed
//! range (`end < start`, e.g. `<0100> <0000> [...]`) underflowed the `u32`
//! subtraction, and `end == u32::MAX` (e.g. `<0000> <FFFFFFFF> [...]`)
//! overflowed the `+ 1`. Both panicked with "attempt to subtract/add with
//! overflow" under `overflow-checks` — on by default for `cargo test` and
//! `[profile.dev]` builds (this crate's `[profile.release]` does not enable
//! `overflow-checks`, so a release build would not have panicked here; it
//! would have silently wrapped to a bogus count instead).
//!
//! The fix mirrors the already-guarded sequential branch
//! (`end.saturating_sub(start).min(10000)`), but as an entry *count* rather
//! than a 0-based loop offset: `end.saturating_sub(start).saturating_add(1)`.
//! A reversed range therefore clamps to a single entry at `start`, matching
//! the sequential branch's single-entry treatment of the same malformed
//! input.

use xberg_native_pdf::fonts::cmap::parse_tounicode_cmap;

/// A reversed range (`end < start`) must not panic, and must clamp to a
/// single mapping at `start` rather than being treated as empty or walked
/// backward.
#[test]
fn bfrange_array_with_reversed_range_clamps_to_single_entry_instead_of_panicking() {
    let data = b"beginbfrange\n<0100> <0000> [<0041>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).expect("reversed bfrange array must not error");

    assert_eq!(cmap.get(&0x100).as_deref(), Some("A"));
    // The descending "range" must not be walked backward from start to end.
    assert_eq!(cmap.get(&0x0FF), None);
}

/// `end == u32::MAX` must not overflow computing `end - start + 1`.
#[test]
fn bfrange_array_with_end_at_u32_max_does_not_overflow() {
    let data = b"beginbfrange\n<0000> <FFFFFFFF> [<0041>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).expect("u32::MAX bfrange array must not error");

    // Only one destination is actually declared in the array; it maps to
    // the range start regardless of how large the declared range is.
    assert_eq!(cmap.get(&0x0).as_deref(), Some("A"));
}

/// Positive control: an ordinary, well-formed bfrange ARRAY form must still
/// map every code in the range to exactly the corresponding destination —
/// proving the overflow guard above did not change behavior for the common
/// case.
#[test]
fn bfrange_array_ordinary_range_maps_every_code_to_its_destination() {
    let data = b"beginbfrange\n<0010> <0014> [<0041> <0042> <0043> <0044> <0045>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).expect("well-formed bfrange array must parse");

    assert_eq!(cmap.get(&0x10).as_deref(), Some("A"));
    assert_eq!(cmap.get(&0x11).as_deref(), Some("B"));
    assert_eq!(cmap.get(&0x12).as_deref(), Some("C"));
    assert_eq!(cmap.get(&0x13).as_deref(), Some("D"));
    assert_eq!(cmap.get(&0x14).as_deref(), Some("E"));

    // Codes just outside the declared range remain unmapped.
    assert_eq!(cmap.get(&0x0F), None);
    assert_eq!(cmap.get(&0x15), None);
}
