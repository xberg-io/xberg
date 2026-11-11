// Tests run under the workspace's `unsafe_code = "deny"`. These stream bodies carry
// binary sample data, so they are not valid UTF-8 -- that is why the original used
// `from_utf8_unchecked`. A test has no performance reason to skip validation, and the
// assertions below only ever match ASCII substrings, so a lossy conversion is exact
// for their purposes.
//! Group A probes for DeviceN /Process polish.
//!
//! Closes the deferred items round 4 surfaced and adds spec coverage
//! that no prior fixture exercised:
//!
//!  - A1: ICCBased /Process /ColorSpace overprint path. The fallback
//!    documented as `HONEST_GAP_DEVICEN_PROCESS_ICC_OVERPRINT`
//!    treated /Process /ColorSpace [/ICCBased <stream>] as
//!    unresolved, falling through to a lossy §10.3.5 RGB inverse
//!    and zeroing K. Round 5 reads the ICC profile's input-channel
//!    count via the existing `IccProfile::parse` cross-check
//!    infrastructure; when N=4, the source tints are accepted as
//!    the destination CMYK directly (§8.6.6.5 "values associated
//!    with the process components shall be stored in their natural
//!    form"). When N=3 or N=1 the embedded profile's CMM converts
//!    through sRGB and a §10.3.5 inverse recovers CMYK — same shape
//!    as the inline /DeviceRGB / /DeviceGray /Process arms.
//!
//!  - A2: /NChannel + /Process /DeviceRGB. No fixture pinned the
//!    §8.6.6.5 RGB process attribution arm under /NChannel.
//!
//!  - A3: Mixed DeviceN with process prefix + spot tail. /Cyan
//!    /Magenta /Yellow /Black process prefix + /PMS185 spot tail,
//!    scn 5-arg. Verifies process and spot lanes both receive the
//!    correct tints under overprint.
//!
//!  - A4: /DeviceN /Process initial colour per §8.6.8. `cs /CS_N` with
//!    a /Process /CMYK + 4-component /Components must populate the
//!    CMYK identity from the initial tint values (all 1.0) so an
//!    overprint between `cs` and `scn` sees (1, 1, 1, 1) source
//!    rather than the post-round-4 stale None.
//!
//!  - A5: /Process /Components mismatched-names policy. When a
//!    /Components name does not appear in /Names, the implementation
//!    returns None and the call site falls through to the §10.3.5
//!    RGB inverse. Round 5 pins this as a HONEST_GAP and emits a
//!    log warning per the round-1 spot extraction precedent.
//!
//! Spec citations:
//!  - ISO 32000-1 §8.6.5.5 — ICCBased colour spaces
//!  - ISO 32000-1 §8.6.6.5 — DeviceN /Process + /Components +
//!    /Subtype /NChannel
//!  - ISO 32000-1 §8.6.8   — initial colour values per colour space
//!  - ISO 32000-1 §10.3.5  — additive-clamp colour conversion
//!  - ISO 32000-1 §11.3.3  — single shape / opacity per pixel
//!  - ISO 32000-1 §11.7.4.3 — CompatibleOverprint blend function
//!    (Table 149 row 2: "any other process colour space" path)

#![allow(dead_code)]

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::render_separations;

/// /Process /ColorSpace [/ICCBased <stream>] where the embedded ICC
/// profile is NOT identical to the document's `/OutputIntents`
/// /DestOutputProfile. Round 5 takes the ICC source tints "in their
/// natural form" (§8.6.6.5) and uses them directly as the destination
/// CMYK. When the embedded profile and the OutputIntent profile model
/// different inks (different paper white, different TVI curves, …),
/// the source tints in the embedded profile's CMYK space are NOT the
/// same press values as the same tints in the OutputIntent's space.
///
/// qcms 0.3.0 supports CMYK→RGB but not CMYK→CMYK transforms, so a
/// proper profile-to-profile re-targeting is not currently available
/// through the linked CMM. Round 5 chooses the "natural form" reading
/// because real-world prepress PDFs overwhelmingly embed the
/// OutputIntent profile as the DeviceN /Process /ColorSpace (or omit
/// the /Process /ColorSpace altogether) — the divergent-profiles case
/// is rare. The alternate reading would round-trip through sRGB and
/// recover CMYK via §10.3.5, which destroys the K channel.
///
/// See `HONEST_GAP_DEVICEN_PROCESS_ICC_PROFILE_MISMATCH_R7` in
/// `tests/test_46_round7_icc_retargeting.rs` for the `icc-lcms2`
/// closure path — the `CmykRetargetTransform` pipeline (CMYK → Lab PCS
/// → CMYK with BPC on, intent threaded from `gs.rendering_intent`)
/// runs the §8.6.5.5 retarget end-to-end. The qcms-only state below
/// remains documented as the no-closure baseline.
pub const HONEST_GAP_DEVICEN_PROCESS_ICC_PROFILE_MISMATCH: &str = "HONEST_GAP_DEVICEN_PROCESS_ICC_PROFILE_MISMATCH: when a DeviceN \
     /Process /ColorSpace [/ICCBased <N=4 stream>] declaration uses an \
     ICC profile distinct from the document's OutputIntent CMYK \
     /DestOutputProfile, the round-5 implementation accepts the source \
     tints as destination CMYK directly (§8.6.6.5 'natural form' \
     reading). A CMM-driven CMYK→CMYK retargeting is not currently \
     available through the linked qcms 0.3.0 backend (qcms supports \
     CMYK→RGB only). The defensible alternate is to round-trip the \
     source CMYK through sRGB via the embedded profile and recover \
     destination CMYK via §10.3.5 — but this discards K. The chosen \
     reading preserves K and matches the common production case where \
     the embedded profile IS the OutputIntent profile.";

/// /Process /Components names that don't appear in the parent /Names
/// array. §8.6.6.5 specifies /Components MUST be a leading prefix of
/// /Names (the natural order of the process colorants). A malformed
/// PDF where a /Components entry is not in /Names is unspecified
/// reader behaviour.
///
/// Round 5 treats the entire /Process attribution as INERT in that
/// case (no /Process /ColorSpace lookup, no /Components filtering of
/// the spot set). `extract_process_paint_cmyk` returns None (with a
/// `log::warn!` for downstream tooling) and `extract_paint_spot_inks`
/// surfaces every /Names entry as a spot colorant. The dispatcher
/// then routes through the SeparationOrDeviceN class (process lanes
/// preserve backdrop; named spot lanes get the tint).
///
/// The defensible alternates:
///  - silently substitute a 0 tint for the missing name (masks the
///    source defect; declined).
///  - drop the whole DeviceN paint (over-aggressive — well-formed
///    /Names + malformed /Components is recoverable).
///  - take /Components as authoritative and treat /Names as a
///    superset (spec-incorrect — /Names is the colorant identity).
///
/// The "treat /Process as inert" reading preserves the source paint
/// (spot tints land on their plates) and aligns with the
/// `extract_process_paint_cmyk` None-on-mismatch contract.
pub const HONEST_GAP_DEVICEN_PROCESS_MISMATCHED_NAMES: &str = "HONEST_GAP_DEVICEN_PROCESS_MISMATCHED_NAMES: a DeviceN /Process \
     /Components entry that does not appear in /Names violates \
     §8.6.6.5 ('the components shall appear in the colorants in the \
     same order they appear in the Names array'). Round 5 treats the \
     whole /Process attribution as inert: `extract_process_paint_cmyk` \
     returns None (with a `log::warn!`) and `extract_paint_spot_inks` \
     surfaces every /Names entry as a spot colorant (no /Components \
     filtering). The dispatcher routes through SeparationOrDeviceN: \
     process lanes preserve backdrop, named spot lanes receive the \
     tint via the spot mirror. The defensible alternate readings — \
     silent zero substitution for missing names, or §10.3.5 \
     RGB-inverse fallback — are declined as either masking the \
     defect or destroying the K channel.";

// ===========================================================================
// Synthetic PDF builder — re-uses the round-4 shape for corpus uniformity.
// =========================================================================== ~keep

fn build_pdf_with_output_intent(
    content: &str,
    resources_inner: &str,
    icc_profile: &[u8],
    // Raw object bytes, not `&str`: these streams carry binary payloads (ICC profiles,
    // image-mask samples) that are not valid UTF-8. A `String` round-trip would either be
    // UB (`from_utf8_unchecked`) or corrupt them (`from_utf8_lossy` rewrites each invalid
    // byte as U+FFFD, changing the samples and desynchronising the `/Length` they declare).
    extra_objs: &[&[u8]],
) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /OutputIntents [<< /Type /OutputIntent /S /GTS_PDFX /OutputCondition (Synthetic Non-Linear CMYK) /DestOutputProfile 5 0 R >>] >>\nendobj\n",
    );
    let pages_off = buf.len();
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page_off = buf.len();
    let page = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << {} >> /Contents 4 0 R >>\nendobj\n",
        resources_inner
    );
    buf.extend_from_slice(page.as_bytes());
    let stream_off = buf.len();
    let stream_hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(stream_hdr.as_bytes());
    buf.extend_from_slice(content.as_bytes());
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let icc_off = buf.len();
    let icc_hdr = format!("5 0 obj\n<< /N 4 /Length {} >>\nstream\n", icc_profile.len());
    buf.extend_from_slice(icc_hdr.as_bytes());
    buf.extend_from_slice(icc_profile);
    buf.extend_from_slice(b"\nendstream\nendobj\n");

    let mut extra_offs: Vec<usize> = Vec::new();
    for obj in extra_objs {
        extra_offs.push(buf.len());
        buf.extend_from_slice(obj);
    }

    let xref_off = buf.len();
    let total_objs = 5 + extra_objs.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total_objs + 1).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off, icc_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    for off in extra_offs {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objs + 1,
            xref_off
        )
        .as_bytes(),
    );
    buf
}

/// Mirror of `tests/test_46_round4_overprint_spec.rs`'s helper. Produces
/// a constant-Lab CMYK ICC LUT profile. The constant `l_byte` lets us
/// pin per-test L-channel output for visual-pixmap probes; for plate
/// probes the LUT is consumed only as the OutputIntent and the plate
/// bytes ARE the sidecar's subtractive tints.
fn build_constant_cmyk_icc(l_byte: u8) -> Vec<u8> {
    let in_chan: u8 = 4;
    let out_chan: u8 = 3;
    let grid: u8 = 2;
    let mut lut = Vec::with_capacity(2048);

    lut.extend_from_slice(&0x6d66_7431u32.to_be_bytes());
    lut.extend_from_slice(&0u32.to_be_bytes());
    lut.push(in_chan);
    lut.push(out_chan);
    lut.push(grid);
    lut.push(0);

    let identity: [i32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x0001_0000];
    for v in identity {
        lut.extend_from_slice(&(v as u32).to_be_bytes());
    }
    for _ in 0..in_chan {
        for i in 0..256u16 {
            lut.push(i as u8);
        }
    }
    let grid_size = (grid as usize).pow(in_chan as u32);
    for _ in 0..grid_size {
        lut.push(l_byte);
        lut.push(128);
        lut.push(128);
    }
    for _ in 0..out_chan {
        for i in 0..256u16 {
            lut.push(i as u8);
        }
    }

    let mut profile = vec![0u8; 128];
    let total_size: u32 = 128 + 4 + 12 + lut.len() as u32;
    profile[0..4].copy_from_slice(&total_size.to_be_bytes());
    profile[8..12].copy_from_slice(&0x0240_0000u32.to_be_bytes());
    profile[12..16].copy_from_slice(b"prtr");
    profile[16..20].copy_from_slice(b"CMYK");
    profile[20..24].copy_from_slice(b"Lab ");
    profile[36..40].copy_from_slice(b"acsp");
    profile[64..68].copy_from_slice(&0u32.to_be_bytes());
    profile[68..72].copy_from_slice(&0x0000_F6D6u32.to_be_bytes());
    profile[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    profile[76..80].copy_from_slice(&0x0000_D32Du32.to_be_bytes());

    profile.extend_from_slice(&1u32.to_be_bytes());
    profile.extend_from_slice(&0x4132_4230u32.to_be_bytes());
    profile.extend_from_slice(&144u32.to_be_bytes());
    profile.extend_from_slice(&(lut.len() as u32).to_be_bytes());
    profile.extend_from_slice(&lut);

    profile
}

fn plate<'a>(
    plates: &'a [xberg_native_pdf::rendering::SeparationPlate],
    name: &str,
) -> &'a xberg_native_pdf::rendering::SeparationPlate {
    plates
        .iter()
        .find(|p| p.ink_name == name)
        .unwrap_or_else(|| panic!("no plate named {}", name))
}

fn centre(plate: &xberg_native_pdf::rendering::SeparationPlate) -> u8 {
    let off = ((plate.height / 2) * plate.width + plate.width / 2) as usize;
    plate.data[off]
}

// ===========================================================================
// A1 — /DeviceN /Process /ColorSpace [/ICCBased <N=4>] overprint path.
//
// The PDF declares an additional ICCBased stream (object 6, /N 4)
// holding a constant-CMYK profile, distinct from the document
// OutputIntent profile (object 5, /N 4). The DeviceN /Process
// /ColorSpace points to object 6. Source tints (0.5, 0.2, 0.7, 0.1)
// are accepted as destination CMYK directly per the round-5
// "natural form" reading (HONEST_GAP_DEVICEN_PROCESS_ICC_PROFILE_MISMATCH).
//
// Setup mirrors `devicen_process_subtype_routes_to_process_class`:
//   Backdrop: DeviceCMYK (0.4, 0, 0, 0), opaque.
//   Foreground: /CS_N /scn (0.5, 0.2, 0.7, 0.1), /OP true, /ca 0.5,
//     under DeviceN [/Cyan /Magenta /Yellow /Black] /DeviceCMYK
//     <tint transform> /Attributes /Process [...].
//
// Round 4 pre-fix produced C=u8 102 (backdrop preserved, K zero, no
// reconstruction). Round 4 post-fix for /Process /DeviceCMYK
// produced C=u8 115, M=u8 26, Y=u8 89, K=u8 13. Round 5 for
// /Process /ICCBased (N=4) reproduces the same plate output —
// proving the ICCBased N=4 path no longer falls back to the lossy
// RGB inverse.
//
// Byte-exact computation (Table 149 row 2 "any other process colour
// space", §11.3.3 composite at α=0.5, backdrop (0.4, 0, 0, 0)):
//   C: c_s=0.5, c_b=0.4 → c_r = 0.5·0.5 + 0.5·0.4 = 0.45 → u8 115.
//   M: c_s=0.2, c_b=0.0 → c_r = 0.5·0.2 + 0.5·0.0 = 0.10 → u8 26.
//   Y: c_s=0.7, c_b=0.0 → c_r = 0.5·0.7 + 0.5·0.0 = 0.35 → u8 89.
//   K: c_s=0.1, c_b=0.0 → c_r = 0.5·0.1 + 0.5·0.0 = 0.05 → u8 13.
// =========================================================================== ~keep

#[test]
fn a1_devicen_process_iccbased_n4_overprint_byte_exact() {
    let icc = build_constant_cmyk_icc(135);
    // Second ICC profile bytes — different L_byte so the content_hash
    // differs from the document OutputIntent profile. This proves the
    // round-5 reconstruction does NOT require the embedded process
    // /ColorSpace and the OutputIntent profile to be identical. ~keep
    let process_icc = build_constant_cmyk_icc(200);
    let psfunc = "<< /FunctionType 2 /Domain [0 1 0 1 0 1 0 1] \
                  /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0 0 0 0] /C1 [1 1 1 1] /N 1 >>";
    let content = "0.4 0 0 0 k\n0 0 100 100 re\nf\n\
                   /CS_N cs\n/Ov gs\n0.5 0.2 0.7 0.1 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Ov << /Type /ExtGState /OP true /ca 0.5 >> >> \
         /ColorSpace << /CS_N [/DeviceN [/Cyan /Magenta /Yellow /Black] \
            /DeviceCMYK {} \
            << /Process << /ColorSpace [/ICCBased 6 0 R] \
                          /Components [/Cyan /Magenta /Yellow /Black] >> >> \
         ] >>",
        psfunc
    );
    let process_icc_obj = format!("6 0 obj\n<< /N 4 /Length {} >>\nstream\n", process_icc.len());
    let mut process_icc_obj_bytes = Vec::from(process_icc_obj.as_bytes());
    process_icc_obj_bytes.extend_from_slice(&process_icc);
    process_icc_obj_bytes.extend_from_slice(b"\nendstream\nendobj\n");
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[&process_icc_obj_bytes]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let c = centre(plate(&plates, "Cyan"));
    let m = centre(plate(&plates, "Magenta"));
    let y = centre(plate(&plates, "Yellow"));
    let k = centre(plate(&plates, "Black"));

    assert_eq!(
        c, 115,
        "ISO 32000-1 §8.6.6.5 + §11.7.4.3 Table 149 row 2: a DeviceN \
         /Process /ColorSpace [/ICCBased <N=4>] declaration carries \
         source CMYK in the ICC profile's CMYK space; round 5 accepts \
         the tints in their natural form. C lane: c_s=0.5, c_b=0.4, \
         α=0.5: c_r = 0.5·0.5 + 0.5·0.4 = 0.45 → u8 115. Got u8 {}. \
         A regression to u8 102 means the K-zeroing RGB-inverse \
         fallback is still active.",
        c
    );
    assert_eq!(
        m, 26,
        "ICCBased N=4 /Process: M lane c_s=0.2, c_b=0, α=0.5: c_r = \
         0.10 → u8 26. Got u8 {}.",
        m
    );
    assert_eq!(
        y, 89,
        "ICCBased N=4 /Process: Y lane c_s=0.7, c_b=0, α=0.5: c_r = \
         0.35 → u8 89. Got u8 {}.",
        y
    );
    assert_eq!(
        k, 13,
        "ICCBased N=4 /Process: K lane c_s=0.1, c_b=0, α=0.5: c_r = \
         0.05 → u8 13. A regression to u8 0 indicates the RGB-inverse \
         fallback (which zeroes K via §10.3.5) is still firing. Got \
         u8 {}.",
        k
    );

    // Cross-check: a HONEST_GAP for the profile-mismatch reading is
    // pinned by source string presence — the constant declared above
    // must exist in this test file so a textual grep across `tests/`
    // sees the open question by name. ~keep
    let source = include_str!("test_46_round5_devicen_process_polish.rs");
    assert!(
        source.contains("HONEST_GAP_DEVICEN_PROCESS_ICC_PROFILE_MISMATCH"),
        "round 5's profile-mismatch HONEST_GAP constant declaration must \
         remain present in the source so future readers can locate the \
         open question by name."
    );
}

// ===========================================================================
// A2 — /NChannel + /Process /DeviceRGB.
//
// No prior fixture exercised /Subtype /NChannel with a /Process
// /ColorSpace /DeviceRGB. §8.6.6.5 admits this combination: the
// /Components prefix declares the RGB process channels (R, G, B in
// that name order — note that "Red", "Green", "Blue" are NOT the
// names used in PDF /DeviceRGB, which is unnamed; §8.6.6.5 says the
// /Components names "shall be" the colorants in /Names, leading
// prefix, in the order corresponding to the /Process /ColorSpace's
// channel order). So for /Process /ColorSpace /DeviceRGB, the
// /Components names map name-by-position to (R, G, B).
//
// Setup:
//   DeviceN colorants: [/Red /Green /Blue /PMS185]
//   Subtype: /NChannel
//   Process /ColorSpace /DeviceRGB, /Components [/Red /Green /Blue]
//   Paint scn (0.2, 0.6, 0.8, 0.5):
//     Red tint = 0.2 → R = 0.2
//     Green tint = 0.6 → G = 0.6
//     Blue tint = 0.8 → B = 0.8
//     PMS185 tint = 0.5
//   /OP true, /ca 0.5
//   Backdrop: (0.4, 0, 0, 0) DeviceCMYK
//
// §10.3.5 RGB → CMYK at the /Process boundary (per `sidecar.rs`
// `extract_process_paint_cmyk` /DeviceRGB arm):
//   C = 1 - R = 1 - 0.2 = 0.8
//   M = 1 - G = 1 - 0.6 = 0.4
//   Y = 1 - B = 1 - 0.8 = 0.2
//   K = 0
//
// §11.7.4.3 Table 149 row 2 "any other process colour space", OPM=0:
//   B = c_s for every process channel, composite c_r = α·B + (1−α)·c_b.
//   C: α·0.8 + (1−α)·0.4 = 0.5·0.8 + 0.5·0.4 = 0.6 → u8 round(153) = 153.
//   M: α·0.4 + (1−α)·0   = 0.5·0.4 + 0.5·0   = 0.2 → u8 round(51) = 51.
//   Y: α·0.2 + (1−α)·0   = 0.5·0.2 + 0.5·0   = 0.1 → u8 round(25.5) = ?
//      Floating-point f32 detail: 1 - 0.8_f32 = 0.19999999 (not 0.2),
//      so 0.5 × 0.19999999 × 255 = 25.499998 → u8 round = 25. The
//      byte-exact reference is 25 — the §10.3.5 inverse's `1 - B`
//      step happens in f32 before the round, picking up the inexact
//      0.8 representation.
//   K: α·0   + (1−α)·0   = 0 → u8 0.
// =========================================================================== ~keep

#[test]
fn a2_nchannel_process_devicergb_overprint_byte_exact() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1 0 1 0 1 0 1] \
                  /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0 0 0 0] /C1 [1 1 1 1] /N 1 >>";
    let content = "0.4 0 0 0 k\n0 0 100 100 re\nf\n\
                   /CS_N cs\n/Ov gs\n0.2 0.6 0.8 0.5 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Ov << /Type /ExtGState /OP true /ca 0.5 >> >> \
         /ColorSpace << /CS_N [/DeviceN [/Red /Green /Blue /PMS185] \
            /DeviceCMYK {} \
            << /Subtype /NChannel \
               /Process << /ColorSpace /DeviceRGB \
                          /Components [/Red /Green /Blue] >> >> \
         ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let c = centre(plate(&plates, "Cyan"));
    let m = centre(plate(&plates, "Magenta"));
    let y = centre(plate(&plates, "Yellow"));
    let k = centre(plate(&plates, "Black"));

    assert_eq!(
        c, 153,
        "ISO 32000-1 §8.6.6.5 + §10.3.5 + §11.7.4.3 Table 149 row 2: \
         /NChannel /Process /DeviceRGB takes /Components-mapped tints \
         as RGB then inverts to CMYK at the /Process boundary. R=0.2 \
         → C = 1 - R = 0.8. Composite over c_b=0.4 at α=0.5: c_r = \
         0.6 → u8 153. Got u8 {}.",
        c
    );
    assert_eq!(
        m, 51,
        "/NChannel /Process /DeviceRGB: G=0.6 → M = 1 - G = 0.4. \
         c_r = 0.5·0.4 + 0.5·0 = 0.2 → u8 51. Got u8 {}.",
        m
    );
    assert_eq!(
        y, 25,
        "/NChannel /Process /DeviceRGB: B=0.8 → Y = 1 - B = 0.2 in \
         exact math; in f32 the §10.3.5 inverse 1 - 0.8_f32 yields \
         0.19999999 (not 0.2). c_r = 0.5·0.19999999 + 0.5·0 = \
         0.0999999... → c_r*255 = 25.499998 → u8 round = 25. The \
         inexact 0.8 representation in f32 is the source of the 25 \
         vs 26 difference. Got u8 {}.",
        y
    );
    assert_eq!(
        k, 0,
        "/NChannel /Process /DeviceRGB: §10.3.5 K = 0 (additive-clamp \
         inverse never produces K). Got u8 {}.",
        k
    );
}

// ===========================================================================
// A3 — Mixed DeviceN: process prefix + spot tail.
//
// 5-name DeviceN: [/Cyan /Magenta /Yellow /Black /PMS185] with
// /Process /ColorSpace /DeviceCMYK + /Components [/Cyan /Magenta
// /Yellow /Black]. Paint scn (c, m, y, k, spot) = (0.5, 0.2, 0.7, 0.1,
// 0.6).
//
// Per §8.6.6.5: the leading-prefix /Components feeds the /Process
// /CMYK source; the tail name(s) are spot colorants whose tints write
// to the named spot lane via the round-2 spot mirror. The named-spot
// (PMS185) survives as one tint per spot lane.
//
// Setup:
//   Backdrop: (0.4, 0, 0, 0) DeviceCMYK.
//   Foreground: /CS_N5 /Ov gs scn (0.5, 0.2, 0.7, 0.1, 0.6),
//               /OP true, /ca 0.5.
//
// Process lanes (§11.7.4.3 Table 149 row 2, OPM=0, α=0.5):
//   C: c_s=0.5, c_b=0.4 → c_r = 0.5·0.5 + 0.5·0.4 = 0.45 → u8 115.
//   M: c_s=0.2, c_b=0.0 → c_r = 0.10 → u8 26.
//   Y: c_s=0.7, c_b=0.0 → c_r = 0.35 → u8 89.
//   K: c_s=0.1, c_b=0.0 → c_r = 0.05 → u8 13.
//
// Spot lane PMS185 (§11.3.3 + §11.7.4.2 spot dispatch, /Normal blend):
//   c_s = 0.6, c_b = 0.0 (initial backdrop), α = 0.5.
//   t_r = (1 − α)·t_b + α·c_s = 0.5·0 + 0.5·0.6 = 0.3 in exact math.
//   In f32 the constant 0.6 quantises to 0.6000000238 (round-to-
//   nearest); 0.5 × 0.6000000238 = 0.3000000119; ×255 = 76.500003 →
//   u8 round = 77 (Rust's `f32::round` rounds 0.5 away from zero, so
//   any value strictly above 76.5 rounds to 77). Byte-exact reference
//   is 77, not 76.
// =========================================================================== ~keep

#[test]
fn a3_mixed_devicen_process_prefix_plus_spot_tail_byte_exact() {
    let icc = build_constant_cmyk_icc(135);
    // Five-input tint transform: process channels go through /Process
    // /CMYK; alternate is /DeviceCMYK so the tint transform converts
    // five inputs to four CMYK outputs. We just need the alt to be
    // syntactically valid — round 5 reads /Process directly, not the
    // alternate, for the process source CMYK. ~keep
    let psfunc = "<< /FunctionType 2 /Domain [0 1 0 1 0 1 0 1 0 1] \
                  /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0 0 0 0] /C1 [1 1 1 1] /N 1 >>";
    let content = "0.4 0 0 0 k\n0 0 100 100 re\nf\n\
                   /CS_N5 cs\n/Ov gs\n0.5 0.2 0.7 0.1 0.6 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Ov << /Type /ExtGState /OP true /ca 0.5 >> >> \
         /ColorSpace << /CS_N5 [/DeviceN [/Cyan /Magenta /Yellow /Black /PMS185] \
            /DeviceCMYK {} \
            << /Process << /ColorSpace /DeviceCMYK \
                          /Components [/Cyan /Magenta /Yellow /Black] >> >> \
         ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let c = centre(plate(&plates, "Cyan"));
    let m = centre(plate(&plates, "Magenta"));
    let y = centre(plate(&plates, "Yellow"));
    let k = centre(plate(&plates, "Black"));
    let pms185 = centre(plate(&plates, "PMS185"));

    assert_eq!(
        c, 115,
        "ISO 32000-1 §8.6.6.5 mixed DeviceN: /Process /CMYK + spot tail. \
         Process prefix tint (0.5, 0.2, 0.7, 0.1) reads as source CMYK. \
         C lane: c_s=0.5, c_b=0.4, α=0.5: c_r = 0.45 → u8 115. Got u8 {}.",
        c
    );
    assert_eq!(m, 26, "Mixed DeviceN M lane: u8 26. Got u8 {}.", m);
    assert_eq!(y, 89, "Mixed DeviceN Y lane: u8 89. Got u8 {}.", y);
    assert_eq!(k, 13, "Mixed DeviceN K lane: u8 13. Got u8 {}.", k);

    assert_eq!(
        pms185, 77,
        "ISO 32000-1 §11.3.3 + §8.6.6.5 mixed DeviceN: spot tail's \
         /PMS185 tint = 0.6 lands on the PMS185 lane via the round-2 \
         spot mirror. /Normal blend over initial backdrop 0 at α=0.5: \
         in EXACT math t_r = 0.3 → u8 round(76.5) = 77 (Rust f32 \
         `round` is half-away-from-zero) but 0.6 quantises to \
         0.60000002 in f32 giving 76.500003 → u8 round = 77. The \
         byte-exact reference is 77. Got u8 {}. Regression to 0 \
         would mean the process-prefix tints are consumed but the \
         spot tail is filtered out by `extract_paint_spot_inks`. \
         Regression to 255 would mean the tint is not attenuated by \
         the gs alpha.",
        pms185
    );
}

// ===========================================================================
// A4 — /DeviceN /Process initial colour per §8.6.8.
//
// §8.6.8: /DeviceN initial colour is tint 1.0 for each colorant. So a
// /CS_N declaration with /Process /CMYK + /Components [/Cyan /Magenta
// /Yellow /Black] entered via `cs /CS_N` (without an explicit `scn`)
// must populate the GS CMYK identity as (1, 1, 1, 1). A subsequent
// paint with /OP true under transparency would then route through the
// CompatibleOverprint dispatcher with source CMYK (1, 1, 1, 1).
//
// Setup (carefully crafted to fire the initial-colour path):
//   Backdrop: (0.0, 0.0, 0.0, 0.5) DeviceCMYK — K=0.5.
//   Foreground: /CS_N cs (no scn — initial tint applies)
//               /Ov gs (/OP true, /ca 0.5)
//               0 0 100 100 re; f.
//
// Expected source CMYK from §8.6.8 + §8.6.6.5: (1, 1, 1, 1).
//
// §11.7.4.3 Table 149 row 2 + §11.3.3 at α=0.5 over backdrop
// (0, 0, 0, 0.5):
//   C: c_s=1, c_b=0 → c_r = 0.5·1 + 0.5·0   = 0.5 → u8 round(127.5) = 128.
//   M: c_s=1, c_b=0 → c_r = 0.5 → u8 128.
//   Y: c_s=1, c_b=0 → c_r = 0.5 → u8 128.
//   K: c_s=1, c_b=0.5 → c_r = 0.5·1 + 0.5·0.5 = 0.75 → in EXACT
//      math u8 round(191.25) = 191. In the implementation the
//      compose-first path quantises the backdrop K to u8 128
//      (round(0.5·255) = 128) and reads it back as 128/255 =
//      0.50196078..., producing c_r = 0.5 + 0.5·0.50196078 =
//      0.75098039... → c_r × 255 = 191.5 → u8 round = 192 (Rust f32
//      `round()` rounds 0.5 away from zero). The byte-exact
//      reference is 192. This is the deliberate consequence of the
//      sidecar's 8-bit per-channel quantisation; the spec does not
//      define a precision for the §11.7.4.3 backdrop read, and 8-bit
//      plate storage is the press-target reality.
//
// Round 4's `initial_colour_for_space` left DeviceN's `cmyk` at None;
// `source_for_overprint`'s SeparationOrDeviceN branch then preserved
// backdrop and the K=0.5 would survive but C=M=Y would stay at 0 (and
// the source K of 1.0 the initial tint declared would be lost). A
// regression: K=u8 64 (the round-4 pre-fix result; backdrop K=0.5
// composed with no-K source via RGB inverse at α=0.5 gives
// (1-0.5)·0.5 = 0.25 → u8 64), or C=M=Y=0 (process-channels-preserve-
// backdrop arm fired).
// =========================================================================== ~keep

#[test]
fn a4_devicen_process_initial_colour_cmyk_byte_exact() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1 0 1 0 1 0 1] \
                  /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0 0 0 0] /C1 [1 1 1 1] /N 1 >>";
    // Note: no `scn` operator. `cs /CS_N` enters the space and the
    // §8.6.8 initial tint (1, 1, 1, 1) applies. The paint that follows
    // uses that initial tint as source. ~keep
    let content = "0 0 0 0.5 k\n0 0 100 100 re\nf\n\
                   /CS_N cs\n/Ov gs\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Ov << /Type /ExtGState /OP true /ca 0.5 >> >> \
         /ColorSpace << /CS_N [/DeviceN [/Cyan /Magenta /Yellow /Black] \
            /DeviceCMYK {} \
            << /Process << /ColorSpace /DeviceCMYK \
                          /Components [/Cyan /Magenta /Yellow /Black] >> >> \
         ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let c = centre(plate(&plates, "Cyan"));
    let m = centre(plate(&plates, "Magenta"));
    let y = centre(plate(&plates, "Yellow"));
    let k = centre(plate(&plates, "Black"));

    assert_eq!(
        c, 128,
        "ISO 32000-1 §8.6.8: /DeviceN initial tint is 1.0 per colorant. \
         For /CS_N with /Process /CMYK + 4-component /Components, the \
         initial source CMYK is (1, 1, 1, 1). Under §11.7.4.3 Table \
         149 row 2 + §11.3.3 at α=0.5 over backdrop (0, 0, 0, 0.5): \
         C lane: c_s=1, c_b=0 → c_r = 0.5 → u8 round(127.5) = 128. \
         Got u8 {}. A regression to 0 means the initial-colour CMYK \
         is not being populated for /DeviceN /Process — the round-4 \
         `initial_colour_for_space` returned `cmyk: None` and the \
         overprint dispatcher fell into the SeparationOrDeviceN \
         preserve-backdrop arm (which keeps c_b=0 on C/M/Y).",
        c
    );
    assert_eq!(m, 128, "/DeviceN /Process initial colour: M lane u8 128. Got u8 {}.", m);
    assert_eq!(y, 128, "/DeviceN /Process initial colour: Y lane u8 128. Got u8 {}.", y);
    assert_eq!(
        k, 192,
        "/DeviceN /Process initial colour: K lane c_s=1, c_b=0.5, α=0.5: \
         in EXACT math c_r = 0.5·1 + 0.5·0.5 = 0.75 → u8 191. The \
         compose-first path quantises the backdrop K to u8 128 \
         (round(0.5·255) = 128) and reads it back as 0.50196078; \
         c_r = 0.5 + 0.5·0.50196078 = 0.75098039 → c_r × 255 = 191.5 \
         → u8 round = 192. Got u8 {}. A regression to 64 indicates \
         the initial-colour CMYK was not populated and the §10.3.5 \
         RGB-inverse path fired with stale (0,0,0) RGB (source CMYK \
         (1,1,1,0) — K dropped). A regression to 128 indicates the \
         SeparationOrDeviceN preserve-backdrop arm fired (no source K).",
        k
    );
}

// ===========================================================================
// A5 — /Process /Components mismatched-names HONEST_GAP policy.
//
// §8.6.6.5: /Components names MUST appear in /Names as a leading
// prefix. A malformed PDF where /Components contains a name absent
// from /Names is unspecified reader behaviour. Round 5 treats the
// /Process attribution as INERT in that case: both
// `extract_process_paint_cmyk` and the `process_names` filter inside
// `extract_paint_spot_inks` skip the /Process entry, and a
// `log::warn!` is emitted at extraction.
//
// Effect on this fixture:
//   /Names = [/Cyan /Magenta /Yellow /Black]
//   /Process /Components = [/Cyan /Magenta /Yellow /Iridescent]
//   /Iridescent NOT in /Names → malformed.
//   `extract_paint_spot_inks` returns ALL four /Names entries as spot
//   inks (no /Components filtering): [(C, 0.5), (M, 0.2), (Y, 0.7),
//   (K, 0.1)].
//   `extract_process_paint_cmyk` returns None (logs warning).
//   `source_for_overprint`: color_cmyk=None, spot_inks non-empty →
//   SeparationOrDeviceN class. Process lanes preserve backdrop.
//
// The spot mirror writes 0.5 to the "Cyan" spot plane, 0.2 to
// "Magenta", etc — but those are SPOT planes (distinct from process
// plates). The Cyan PROCESS plate (which `render_separations`
// returns under the "Cyan" name when the sidecar exposes a process
// channel) reads the CMYK sidecar's C channel, which preserved
// backdrop = 0.4 → u8 102.
//
// Expected plate bytes (SeparationOrDeviceN preserve-backdrop on
// process lanes, §11.3.3 composite c_r = α·c_b + (1−α)·c_b = c_b):
//   C plate = backdrop C = 0.4 → u8 round(102) = 102.
//   M plate = backdrop M = 0   → u8 0.
//   Y plate = backdrop Y = 0   → u8 0.
//   K plate = backdrop K = 0   → u8 0.
//
// (Alternate readings rejected by round 5:
//   (a) Silent zero substitution for missing names → C=115 (the
//       round-4 broad-read result with the malformed source treated
//       as valid). Rejected: masks the defect.
//   (b) §10.3.5 RGB-inverse fallback → C=121, M=36, Y=93, K=0.
//       Rejected: requires spot_inks=[] which discards the valid
//       spot-tail intent embedded in the source.
// See HONEST_GAP_DEVICEN_PROCESS_MISMATCHED_NAMES for the chosen
// reading's rationale.)
// =========================================================================== ~keep

#[test]
fn a5_devicen_process_mismatched_names_treats_process_as_inert_byte_exact() {
    let icc = build_constant_cmyk_icc(135);
    // 4-input tint transform feeding the /DeviceCMYK alternate. ~keep
    let psfunc = "<< /FunctionType 2 /Domain [0 1 0 1 0 1 0 1] \
                  /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0 0 0 0] /C1 [1 1 1 1] /N 1 >>";
    let content = "0.4 0 0 0 k\n0 0 100 100 re\nf\n\
                   /CS_N cs\n/Ov gs\n0.5 0.2 0.7 0.1 scn\n0 0 100 100 re\nf\n";
    // Mismatched /Components: /Iridescent is NOT in /Names. ~keep
    let resources = format!(
        "/ExtGState << /Ov << /Type /ExtGState /OP true /ca 0.5 >> >> \
         /ColorSpace << /CS_N [/DeviceN [/Cyan /Magenta /Yellow /Black] \
            /DeviceCMYK {} \
            << /Process << /ColorSpace /DeviceCMYK \
                          /Components [/Cyan /Magenta /Yellow /Iridescent] >> >> \
         ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let c = centre(plate(&plates, "Cyan"));
    let m = centre(plate(&plates, "Magenta"));
    let y = centre(plate(&plates, "Yellow"));
    let k = centre(plate(&plates, "Black"));

    // Byte-exact references: malformed /Process is INERT. The
    // SeparationOrDeviceN class preserves backdrop on process lanes;
    // the C plate carries the backdrop's 0.4 tint, M/Y/K stay at 0. ~keep
    assert_eq!(
        c, 102,
        "ISO 32000-1 §8.6.6.5 malformed /Components: round 5 treats \
         /Process as inert. spot_inks include all four /Names entries \
         (no /Components filtering), source_for_overprint routes \
         through SeparationOrDeviceN (preserve backdrop on process \
         lanes). Process C plate = backdrop C = 0.4 → u8 102. Got \
         u8 {}. A regression to 115 indicates silent zero \
         substitution fired (the alternate reading that masks the \
         defect). A regression to 121 indicates the §10.3.5 \
         RGB-inverse fallback fired (which would discard the valid \
         spot-tail intent).",
        c
    );
    assert_eq!(
        m, 0,
        "Malformed /Components: M process plate = backdrop M = 0. \
         Got u8 {}.",
        m
    );
    assert_eq!(
        y, 0,
        "Malformed /Components: Y process plate = backdrop Y = 0. \
         Got u8 {}.",
        y
    );
    assert_eq!(
        k, 0,
        "Malformed /Components: K process plate = backdrop K = 0. \
         Got u8 {}.",
        k
    );

    // Pin the HONEST_GAP constant declaration so a textual grep finds
    // the open question. ~keep
    let source = include_str!("test_46_round5_devicen_process_polish.rs");
    assert!(
        source.contains("HONEST_GAP_DEVICEN_PROCESS_MISMATCHED_NAMES"),
        "round 5's mismatched-names HONEST_GAP constant must be \
         declared in source for grepability."
    );
}
