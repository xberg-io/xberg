//! Regression test for a `ToUnicode` CMap char-boundary panic caused by
//! `fonts::cmap`'s lossy UTF-8 decoding of the raw (attacker-controlled)
//! stream.
//!
//! `parse_tounicode_cmap` decodes the raw stream bytes with
//! `String::from_utf8_lossy`, which substitutes each invalid byte sequence
//! with the 3-byte U+FFFD replacement character. The capture regexes used
//! throughout the module (`<([^>]*)>`) match any character between angle
//! brackets, including U+FFFD, so a captured `dst_hex` can be a mix of
//! 1-byte ASCII characters and one or more 3-byte U+FFFD characters. Several
//! sites then branched on `dst_hex.len()` (a BYTE count) and sliced
//! `dst_hex` at fixed byte offsets, which panics with "byte index N is not a
//! char boundary" whenever a U+FFFD straddles the offset.
//!
//! Of the sites originally suspected, only two are actually reachable with
//! corrupted input:
//! - `parse_bfchar_line`'s `step_by(4)` loop (its final, longer-than-8-bytes
//!   branch), which slices `&dst_hex[i..end]` directly, with no upstream
//!   validation.
//! - the bfrange parser's array-form counterpart, the same `step_by(4)`
//!   shape over each array entry.
//!
//! The `dst_hex.len() == 8` branch in both functions (the UTF-16 surrogate
//! pair / two-BMP-char split) is NOT reachable this way, even though it also
//! slices `&dst_hex[0..4]` / `&dst_hex[4..8]`: that slicing is nested inside
//! an `else` that only runs after `u32::from_str_radix(&dst_hex, 16)` has
//! already succeeded on the FULL 8-byte string. `from_str_radix` rejects any
//! non-ASCII-hex byte (including every byte of a U+FFFD) as an invalid
//! digit, so a corrupted `dst_hex` is already rejected via `?` before the
//! nested slice is ever reached. This is verified below (see
//! `bfchar_dst_with_invalid_utf8_at_eight_byte_length_is_dropped_without_panic`).
//!
//! The fix validates each captured hex string is pure ASCII hex digits at
//! the capture boundary (`is_ascii_hex_digits` in `fonts::cmap`), before any
//! length check or slice — matching this crate's convention of clamping
//! attacker-controlled input at the parse boundary rather than patching each
//! downstream consumption site (see `tounicode_bfrange_array_range_overflow.rs`
//! for the same convention applied to a different defect in a neighboring
//! function). A malformed capture is now rejected the same way any other
//! malformed hex already was: the entry is dropped and a `tracing::warn!`
//! fires, degrading gracefully instead of panicking.

use xberg_native_pdf::fonts::cmap::parse_tounicode_cmap;

/// Reproduces the panic in `parse_bfchar_line`'s `step_by(4)` slicing loop.
///
/// Byte layout (post `from_utf8_lossy`): three ASCII bytes, then one U+FFFD
/// (3 bytes) straddling the first `step_by(4)` chunk boundary, then five more
/// ASCII bytes — 11 bytes total, landing in the `else` branch (neither `<=6`
/// nor `== 8`) that has no upstream ASCII validation. The raw stream carries
/// a single genuinely invalid byte (`0xFF`) to produce that U+FFFD; without
/// this fix, slicing `&dst_hex[0..4]` cuts through the middle of it and
/// panics.
#[test]
fn bfchar_dst_with_invalid_utf8_in_step_by_branch_does_not_panic() {
    let data = b"beginbfchar\n<0041> <AAA\xFFBBBBB>\n<0042> <0042>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).expect("a malformed dst entry is degraded, not fatal");

    assert_eq!(
        cmap.get(&0x41),
        None,
        "the entry with the corrupted dst hex must be dropped, not guessed at"
    );
    assert_eq!(
        cmap.get(&0x42).as_deref(),
        Some("B"),
        "the well-formed neighbor entry on the same line must still parse"
    );
}

/// Same defect shape, reproduced through the bfrange parser's ARRAY-form
/// `step_by(4)` loop — a distinct code path (`beginbfrange`'s
/// `[<...>]` array form) from the bfchar case above, so it needs its own
/// coverage. With the fix, the corrupted array entry never even reaches the
/// `step_by` loop: it is filtered out of `dst_hexes` at the capture
/// boundary, which is reported (correctly) as an array-size mismatch.
#[test]
fn bfrange_array_dst_with_invalid_utf8_in_step_by_branch_does_not_panic() {
    let data = b"beginbfrange\n<0010> <0010> [<AAA\xFFBBBBB>]\nendbfrange\n\
                beginbfchar\n<0011> <0058>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).expect("a malformed array entry is degraded, not fatal");

    assert_eq!(
        cmap.get(&0x10),
        None,
        "the range entry with the corrupted dst hex must be dropped, not guessed at"
    );
    assert_eq!(
        cmap.get(&0x11).as_deref(),
        Some("X"),
        "an unrelated, well-formed bfchar entry elsewhere in the stream must still parse"
    );
}

/// The exact worked byte layout that motivated this fix: two ASCII bytes,
/// then one U+FFFD (3 bytes) occupying byte offsets 2-4, then three more
/// ASCII bytes — 8 bytes total, satisfying the `dst_hex.len() == 8` check
/// that gates the UTF-16-surrogate-pair / two-BMP-char branch.
///
/// This does NOT panic even without the fix (see module doc comment for
/// why: the whole-string `from_str_radix` inside that branch already
/// rejects it), so this test documents and locks in that the branch is safe
/// — both before and after the capture-boundary guard — rather than proving
/// the guard fixed it.
#[test]
fn bfchar_dst_with_invalid_utf8_at_eight_byte_length_is_dropped_without_panic() {
    let data = b"beginbfchar\n<0041> <AB\xFFCDE>\n<0042> <0042>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).expect("a malformed 8-byte dst entry is degraded, not fatal");

    assert_eq!(
        cmap.get(&0x41),
        None,
        "the corrupted 8-byte dst hex must be dropped, not guessed at"
    );
    assert_eq!(
        cmap.get(&0x42).as_deref(),
        Some("B"),
        "the well-formed neighbor entry must still parse"
    );
}

/// POSITIVE CONTROL: an ordinary 4-hex-digit bfchar mapping must still
/// resolve exactly as before — proving the ASCII-hex guard does not reject
/// legitimate short hex codes.
#[test]
fn bfchar_four_hex_digit_form_still_maps() {
    let data = b"beginbfchar\n<0041> <0041>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x41).as_deref(), Some("A"));
}

/// POSITIVE CONTROL: an 8-hex-digit UTF-16 surrogate pair must still decode
/// to its combined supplementary-plane code point — proving the guard does
/// not disturb the branch it sits in front of. `D835DF0C` is high surrogate
/// `0xD835` + low surrogate `0xDF0C`, decoding to U+1D70C (MATHEMATICAL
/// ITALIC SMALL RHO).
#[test]
fn bfchar_eight_hex_digit_surrogate_pair_form_still_decodes() {
    let data = b"beginbfchar\n<0003> <D835DF0C>\nendbfchar";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x3).as_deref(), Some("\u{1D70C}"));
}

/// POSITIVE CONTROL: a bfrange ARRAY form with multi-char (ligature)
/// destinations must still map every code to its corresponding destination
/// — proving the `dst_hexes` filter in the array branch does not drop
/// legitimate multi-char entries.
#[test]
fn bfrange_array_multi_char_ligature_form_still_maps() {
    let data = b"beginbfrange\n<005F> <0061> [<00660066> <00660069> <00660066006C>]\nendbfrange";
    let cmap = parse_tounicode_cmap(data).unwrap();
    assert_eq!(cmap.get(&0x5F).as_deref(), Some("ff"));
    assert_eq!(cmap.get(&0x60).as_deref(), Some("fi"));
    assert_eq!(cmap.get(&0x61).as_deref(), Some("ffl"));
}
