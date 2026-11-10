// Tests run under the workspace's `unsafe_code = "deny"`. These stream bodies carry
// binary sample data, so they are not valid UTF-8 -- that is why the original used
// `from_utf8_unchecked`. A test has no performance reason to skip validation, and the
// assertions below only ever match ASCII substrings, so a lossy conversion is exact
// for their purposes.
//! Round-2 QA pass for spot-lane paint writes.
//!
//! These probes scrutinise the round-2 design+impl commit (`f5bdb9b`)
//! along all six self-flagged scrutiny areas and the additional surface
//! the QA brief enumerated. Each probe pins a byte-exact observation;
//! probes marked `#[ignore]` carry a `QA_BUG_*` constant naming the
//! exact misbehaviour, the spec citation that grounds the correct
//! behaviour, and the value the fix agent must achieve.
//!
//! Probes marked active (no `#[ignore]`) pin behaviour the impl
//! already gets right — they are regression guards.
//!
//! Methodology references:
//!  - `docs/research/2026-06-06-nonsep-blends-in-devicen.md` —
//!    architectural decision (CMYK is the blend space; spots ride
//!    alongside; §11.7.4.2 splits BM per lane class).
//!  - `tests/test_46_round2_spot_paint_writes.rs` — round-2 design+impl
//!    probes; this QA file augments without overlap.
//!  - `tests/test_46_round1_qa_pass.rs` — round-1 QA shape this file
//!    mirrors.
//!
//! Spec citations:
//!  - ISO 32000-1 §8.6.6.3 reserved `/All` / `/None`
//!  - ISO 32000-1 §8.6.6.4 `/Separation`
//!  - ISO 32000-1 §8.6.6.5 `/DeviceN` + `/Process` attributes
//!  - ISO 32000-1 §8.6.8 `/cs` operator: resets current colour to
//!    initial value
//!  - ISO 32000-1 §11.3.3 basic compositing formula (α applies to
//!    every lane symmetrically)
//!  - ISO 32000-1 §11.3.5.2 separable blend modes + Note 2
//!  - ISO 32000-1 §11.3.5.3 non-separable blend modes
//!  - ISO 32000-1 §11.4.7 soft masks (modulate the alpha of the
//!    object being painted)
//!  - ISO 32000-1 §11.6.3 `/BM` array first-recognised rule
//!  - ISO 32000-1 §11.6.5.2 SMask group's `/G` colour space (spots
//!    revert to alternate inside the soft-mask group)
//!  - ISO 32000-1 §11.6.6 Group XObjects (group `/CS` excludes
//!    Separation/DeviceN)
//!  - ISO 32000-1 §11.7.3 spot colours and transparency (sidecar)
//!  - ISO 32000-1 §11.7.4.2 BM split per lane class
//!  - ISO 32000-1 §11.7.4.3 CompatibleOverprint

#![allow(dead_code)]

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

// ===========================================================================
// QA bug markers — pin the exact misbehaviour with spec citation.
// =========================================================================== ~keep

/// SMask must modulate spot lanes the same way it modulates process
/// channels. ISO 32000-1 §11.4.7 says the soft mask produces an
/// additional alpha that combines with `α_s` for the object being
/// painted; §11.3.3's basic compositing formula uses a SINGLE α per
/// pixel that applies to every component lane (§11.7.3 carries this
/// over to spot lanes: "Only a single shape value and opacity value
/// shall be maintained at each point in the computed group results;
/// they shall apply to both process and spot colour components.").
///
/// The round 2 impl runs the spot mirror BEFORE
/// `apply_smask_after_paint`, so the spot lane composes at α' =
/// coverage·gs.fill_alpha without the SMask attenuation. The visible
/// pixmap is then attenuated by SMask, but the spot lane retains its
/// pre-SMask tint — producing over-dense plate output relative to the
/// visible composite. For a uniform SMask = 0.5 over a /Separation
/// /InkA at tint 0.6, the spot lane stores tint 0.6 instead of the
/// spec-correct 0.3.
pub const QA_BUG_SMASK_DOES_NOT_MODULATE_SPOT_LANE: &str = "QA_BUG_SMASK_DOES_NOT_MODULATE_SPOT_LANE: ISO 32000-1 §11.4.7 + \
     §11.7.3 + §11.3.3: a single (shape, opacity) per pixel applies to \
     every lane. The SMask's alpha modulation must apply to the spot \
     lane the same as to process lanes. The round-2 impl runs the spot \
     mirror before `apply_smask_after_paint`, so the spot lane gets \
     composed at α' = coverage·gs.fill_alpha with NO SMask attenuation. \
     Result: spot lanes over-dense relative to the visible pixmap.";

/// The snapshot-vs-post-paint diff used by combo / text / Do / sh
/// paint sites treats every changed pixel as full coverage (255) on
/// the spot lane. At AA edges where the visible alpha-contribution is
/// fractional (1..254), the spot lane gets full ink. ISO 32000-1
/// §11.7.3 requires the SAME shape and opacity per pixel on every
/// lane — the diff branch violates that requirement at edges.
///
/// For the simple path-Fill / path-Stroke sites the impl uses the
/// pre-rasterised coverage mask (correct). The diff branch fires at:
/// `B`/`b`/`B*`/`b*` (FillStroke combos), text-show ops (Tj/TJ/'/"),
/// `Do` (form / image XObject), and `sh` (shading).
pub const QA_BUG_SPOT_MIRROR_AA_EDGE_BINARY_COVERAGE: &str = "QA_BUG_SPOT_MIRROR_AA_EDGE_BINARY_COVERAGE: ISO 32000-1 §11.7.3 \
     + §11.3.3 require a single per-pixel (shape, opacity) on every \
     lane. The round-2 spot mirror's snapshot-vs-post diff treats \
     every changed pixel as coverage = 255, so AA edges on combo / \
     text / Do / sh paint sites get full-ink tint on the spot lane \
     while the visible pixmap has fractional alpha. Fix: rasterise an \
     actual coverage mask for these paint sites the same way the \
     path-Fill / path-Stroke arms do.";

/// `cs` (SetFillColorSpace) does not reset `fill_color_components`
/// nor `fill_spot_inks`. Per ISO 32000-1 §8.6.8 the operator "shall
/// set the current colour to its initial value" — for a /Separation
/// or /DeviceN space §8.6.6.4 / §8.6.6.5 pin the initial tint at
/// **1.0** for each colorant (not 0.0 — the §8.6.6.4 text reads "The
/// initial value for both the stroking and nonstroking colour in the
/// graphics state shall be 1.0"). In every case the active spot
/// identity should reflect the NEW colour space's colorant list at
/// the new initial tint, not the prior one.
///
/// Concretely: after `cs /CS_Sep_A scn 0.5 cs /CS_Sep_B f`, the
/// pre-fix impl wrote lane A at tint 0.5 (stale `fill_spot_inks`).
/// Spec-correct: the f uses /CS_Sep_B at its initial tint 1.0 —
/// lane B writes at tint 1.0, lane A is unsourced (preserved at
/// backdrop zero under HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_
/// BACKDROP).
pub const QA_BUG_CS_DOES_NOT_RESET_SPOT_IDENTITY: &str = "QA_BUG_CS_DOES_NOT_RESET_SPOT_IDENTITY: ISO 32000-1 §8.6.8: the \
     `cs` operator sets the current colour to its initial value. The \
     pre-fix SetFillColorSpace handler did not clear `fill_spot_inks` \
     or reset `fill_color_components`, so a paint operator that ran \
     after `cs /CS_B` without an intervening `scn` used the prior \
     /Separation's colorant identity at the prior tint. Fix: \
     SetFillColorSpace / SetStrokeColorSpace must reset the \
     corresponding `*_spot_inks` to the NEW space's colorant list at \
     initial tint 1.0 (Separation / DeviceN per §8.6.6.4 / §8.6.6.5) \
     and reset `*_color_components` to (0, 0, 0, 1) for DeviceCMYK / \
     (0,…,0) for device-family RGB-Gray / 1.0-per-colorant for \
     Separation and DeviceN.";

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

/// Constant-output CMYK→Lab ICC profile.
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

fn tint_to_u8(t: f32) -> u8 {
    (t.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn compose_normal(t_b: f32, t_s: f32, alpha: f32) -> f32 {
    (1.0 - alpha) * t_b + alpha * t_s
}

// ===========================================================================
// PROBE QA-1: scrutiny area (b) — explicit zero tint vs unsourced lane
// asymmetry under /Normal at α=1.
// ===========================================================================
//
// HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP says the round-2
// impl preserves the backdrop on lanes NOT named by the source. A
// strict §11.7.3 reading would erase the backdrop under /Normal at
// α=1 because unsourced lanes expand to subtractive tint 0.0. The
// agent chose preserve. So far so good — that is documented.
//
// BUT: when the source DOES name the lane explicitly at tint 0
// (e.g., `/CS_InkA cs 0 scn` on a /Separation /InkA space), the
// impl DOES write to the lane via compose_normal(t_b, 0, 1) = 0 — it
// ERASES the backdrop. This produces an asymmetry between
// "InkA named at tint 0" (erases) and "InkA not named" (preserves).
//
// The asymmetry is genuine and defensible (the source author
// explicitly painted InkA at tint 0 — they may have meant to erase
// it). But it is NOT spelled out in the HONEST_GAP comment. This
// probe pins both shapes and confirms they differ. ~keep

/// EXPLICIT `/Separation /InkA scn 0` over an InkA backdrop of 0.6
/// erases the backdrop. The impl writes tint 0 via compose_normal
/// at α=1: `t_r = 0`.
#[test]
fn qa1_explicit_zero_tint_separation_erases_inka_backdrop_under_normal() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    // First paint: InkA at tint 0.6 lays down backdrop.
    // Second paint: same /CS_PMS Separation /InkA at tint 0 →
    // EXPLICIT zero-tint write. Trigger via /ca 0.5 to allocate sidecar. ~keep
    let content = "/Half gs\n\
                   /CS_PMS cs\n0.6 scn\n0 0 100 100 re\nf\n\
                   0.0 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    // After paint 1 at α=0.5: t_r1 = (1-0.5)·0 + 0.5·0.6 = 0.30196...
    // Quantised to u8 = round(0.30196·255) = 77.
    // After paint 2 reads 77/255 = 0.30196 then composes 0.5·0.30196 +
    // 0.5·0 = 0.15098 → u8 = round(0.15098·255) = 39 (the impl's
    // quantise-between-paints cascade).
    //
    // The probe pins the byte-exact value the spec produces under the
    // mirror's quantised cascade; the precise value depends on f32
    // rounding through the (·255 → u8 → /255) round-trip. We compute
    // it the same way the impl does. ~keep
    let after_paint1_quant = tint_to_u8(compose_normal(0.0, 0.6, 0.5));
    let t_b_paint2 = after_paint1_quant as f32 / 255.0;
    let after_paint2_quant = tint_to_u8(compose_normal(t_b_paint2, 0.0, 0.5));
    let expected = after_paint2_quant;
    assert_eq!(after_paint1_quant, 77);
    assert_eq!(expected, 39);
    assert_eq!(
        plane[centre], expected,
        "ISO 32000-1 §11.7.3 + §11.3.3: an EXPLICIT /Separation /InkA \
         tint 0 composes via the mirror's Normal-substitution path. \
         t_r = (1-α)·t_b + α·t_s with t_s = 0 attenuates the backdrop. \
         After paint 1 quantised to u8 ({}) then paint 2 at α=0.5: \
         expected {} → got {}. The /InkA-not-named comparison probe \
         (qa1_unsourced_inka_lane_preserves_backdrop_under_normal_at_\
         full_alpha) shows the asymmetry: not-named preserves backdrop \
         at u8 77, explicitly-zero erases to u8 39.",
        after_paint1_quant, expected, plane[centre]
    );
}

/// Compared with the previous probe: when InkA is NOT named in the
/// source (a DeviceCMYK paint instead), the InkA backdrop is
/// preserved at the prior tint. The asymmetry between "explicit zero
/// tint" and "not named" is real.
#[test]
fn qa1_unsourced_inka_lane_preserves_backdrop_under_normal_at_full_alpha() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    // First paint: InkA at tint 0.6 lays down backdrop.
    // Second paint: DeviceCMYK (0,0,0,0.3) — InkA is NOT named.
    // Per the HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP policy
    // the InkA lane stays at the post-paint-1 value (0.3 → u8 76). ~keep
    let content = "/Half gs\n\
                   /CS_PMS cs\n0.6 scn\n0 0 100 100 re\nf\n\
                   0 0 0 0.3 k\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    // After paint 1: 0.5·0.6 = 0.3 → u8 = 77 (round). ~keep
    let after_paint1 = compose_normal(0.0, 0.6, 0.5);
    let expected = tint_to_u8(after_paint1);
    assert_eq!(expected, 77);
    assert_eq!(
        plane[centre], expected,
        "HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP: a DeviceCMYK \
         paint that does not name /InkA leaves the InkA lane at its \
         post-paint-1 value of {} (not erased). Got {} at centre.",
        expected, plane[centre]
    );
}

/// `/SMask /S /Luminosity` with a uniform 0.5 luminosity over a
/// /Separation /InkA paint at tint 0.6: per ISO 32000-1 §11.4.7 +
/// §11.7.3 + §11.3.3, a single α value applies to every lane
/// (process AND spot). The SMask attenuates the paint contribution
/// the same way on every lane.
///
/// Why /Luminosity instead of /Alpha: the SMask form's content
/// stream `0.5 g 0 0 100 100 re f` paints opaque grey 0.5. The form
/// pixmap's ALPHA channel is therefore 1.0 across the footprint
/// (the `f` paint is opaque), so `/S /Alpha` extracts mask = 1.0
/// uniformly — no attenuation. To get a uniform 0.5 mask we use
/// `/S /Luminosity` which extracts `Lum((0.5, 0.5, 0.5)) = 0.5`
/// from the form's RGB.
///
/// Byte-exact computation in the impl's quantise-after-mirror
/// cascade:
///  - Mirror writes lane[centre] = post = Normal(0, 0.6) at α=1
///    = 0.6 → u8 = 153.
///  - SMask materialises m = 0.5 at every pixel of the form
///    footprint.
///  - SMask attenuation: out = m·post + (1-m)·pre =
///    0.5·153 + 0.5·0 = 76.5 → round = u8 77.
///
/// Pre-fix the impl ran the spot mirror BEFORE apply_smask_after_
/// paint and DID NOT touch the spot lanes inside the SMask helper.
/// The spot lane stayed at u8 153. Fixed by extending
/// `apply_smask_after_paint` to apply the mask alpha against a
/// pre-mirror spot snapshot, mirroring how the pixmap is attenuated
/// against its pre-paint snapshot.
///
/// QA_BUG_SMASK_DOES_NOT_MODULATE_SPOT_LANE (fixed).
#[test]
fn qa2_smask_alpha_uniform_half_modulates_spot_lane() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let smask_form = "6 0 obj\n\
        << /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
           /Resources << >> \
           /Group << /Type /Group /S /Transparency /CS /DeviceGray >> \
           /Length 28 >>\n\
        stream\n0.5 g\n0 0 100 100 re\nf\nendstream\nendobj\n";
    let content = "/Mask gs\n\
                   /CS_PMS cs\n0.6 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Mask << /Type /ExtGState /SMask << /Type /Mask /S /Luminosity /G 6 0 R >> >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[smask_form.as_bytes()]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    // Cascade: mirror writes u8 153; SMask blends 0.5·153 + 0.5·0 =
    // 76.5 → 77. ~keep
    let post_u8 = tint_to_u8(compose_normal(0.0, 0.6, 1.0));
    assert_eq!(post_u8, 153);
    let m = 0.5_f32;
    let expected = (m * post_u8 as f32 + (1.0 - m) * 0.0).clamp(0.0, 255.0).round() as u8;
    assert_eq!(expected, 77);
    assert_eq!(
        plane[centre], expected,
        "{} — SMask /S /Luminosity at uniform 0.5 must attenuate \
         the spot lane against its pre-mirror snapshot. post-mirror \
         u8 = {}; pre-mirror = 0; m = 0.5; out = 0.5·{} + 0.5·0 \
         = u8 {}. Got {} at centre.",
        QA_BUG_SMASK_DOES_NOT_MODULATE_SPOT_LANE, post_u8, post_u8, expected, plane[centre]
    );
}

/// AA-edge fidelity on Image Do paint sites — closes
/// `QA_BUG_SPOT_MIRROR_AA_EDGE_BINARY_COVERAGE` for the image surface.
///
/// Round 6 wired a rasterised coverage helper for Image / ImageMask
/// Do paints; the spot mirror now sees geometry-true per-pixel
/// coverage at glyph / image / shading boundaries. This probe pins
/// the byte-exact AA-edge behaviour on an ImageMask whose footprint
/// is upscaled 10× (Bicubic) — the resulting per-pixel coverage at
/// the footprint boundary is fractional, and the spot lane carries
/// strictly-between (0, full-coverage) values.
///
/// Construction:
///  - ImageMask, 8×8 uniform paint (every bit 0 per §8.9.6.2
///    /Decode [0 1] default).
///  - CTM `80 0 0 80 10 10` upscales the 8×8 stencil to a 80×80
///    user-space footprint on a 100×100 page (raster y/x ∈ [10, 90)).
///  - /Separation /InkA at tint 1.0, /ca = 0.99.
///  - Interior pixel (50, 50) → full coverage → u8 round(0.99·255) =
///    252 byte-exact.
///  - Pixels along the upscaled footprint boundary should carry
///    STRICTLY FRACTIONAL coverage (lane ∈ (0, 252)) from Bicubic
///    AA at the source-pixel boundary inside the bilinear/bicubic
///    resampling path.
///
/// Under the pre-round-6 diff branch this probe failed in two ways:
///  (a) interior centre was 252 (would still match, because the diff
///      branch's binary coverage at interior pixels coincides with
///      the rasterised full-coverage value here);
///  (b) AA-edge pixels were ALL 252 (binary 255 coverage clamped to
///      full); no fractional values existed.
#[test]
fn qa3_image_do_aa_edge_gets_fractional_coverage_after_fix() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let mask_bytes: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF];
    let form_hdr = format!(
        "6 0 obj\n\
        << /Type /XObject /Subtype /Image /Width 8 /Height 8 \
           /ImageMask true /BitsPerComponent 1 \
           /Length {} >>\nstream\n",
        mask_bytes.len()
    );
    let mut form_full: Vec<u8> = Vec::new();
    form_full.extend_from_slice(form_hdr.as_bytes());
    form_full.extend_from_slice(&mask_bytes);
    form_full.extend_from_slice(b"\nendstream\nendobj\n");
    let content = "/Trig gs\n\
                   /CS_PMS cs\n1.0 scn\n\
                   q\n80 0 0 80 10 10 cm\n/Img Do\nQ\n";
    let resources = format!(
        "/ExtGState << /Trig << /Type /ExtGState /ca 0.99 >> >> \
         /XObject << /Img 6 0 R >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[&form_full]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");
    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();

    // Paint-band centre (50, 20) — image row 1 (paint), well inside
    // the 4-row paint band (rows 0..3), far enough from the inner
    // boundary at image y=4 that the Bicubic kernel only sees paint
    // samples. §11.3.3 at α = 0.99, t_b = 0, t_s = 1.0:
    //   t_r = 0.99·1.0 → u8 252. ~keep
    let expected_centre = tint_to_u8(compose_normal(0.0, 1.0, 0.99));
    assert_eq!(expected_centre, 252);
    let centre_off = (20usize * dims.0 as usize) + 50;
    assert_eq!(
        plane[centre_off], expected_centre,
        "{} — paint-band interior pixel uses the rasterised image \
         coverage (full inside the paint band). t_r = 0.99·1.0 = u8 \
         {}. Got {}.",
        QA_BUG_SPOT_MIRROR_AA_EDGE_BINARY_COVERAGE, expected_centre, plane[centre_off]
    );

    let mut fractional_count = 0usize;
    let mut max_fractional: u8 = 0;
    for y in 0..(dims.1 as usize) {
        for x in 0..(dims.0 as usize) {
            let v = plane[y * dims.0 as usize + x];
            if v > 0 && v < 252 {
                fractional_count += 1;
                if v > max_fractional {
                    max_fractional = v;
                }
            }
        }
    }
    assert!(
        fractional_count > 0,
        "{} — at least one pixel along the upscaled image footprint \
         boundary must carry strictly fractional coverage (lane ∈ \
         (0, 252)) under Bicubic resampling. Got 0 fractional \
         pixels. max_fractional = {}.",
        QA_BUG_SPOT_MIRROR_AA_EDGE_BINARY_COVERAGE,
        max_fractional
    );
}

/// A FillStroke combo (`B`) on a path the rasteriser anti-aliases
/// produces fractional coverage along the path boundary. With the
/// fix (combo paints now use the same rasterised coverage mask as
/// the path-Fill and path-Stroke helpers via `rasterise_fill_
/// coverage` and `rasterise_stroke_coverage`), the spot lane
/// composes at coverage matching the rasteriser's per-pixel AA, not
/// the binary diff.
///
/// The probe samples a CENTRE pixel (deep interior of the filled
/// rectangle) and an EDGE pixel (just inside the rectangle's right
/// edge where the rasterise_fill_coverage mask carries the AA
/// gradient). Per round-2's coverage path, the centre lane should
/// receive full ink (lane = compose_normal at α=1·0.5=0.5 → u8 128),
/// and the edge lane (if AA is present) should fall below 128. The
/// probe pins:
///  - centre receives full coverage (lane = 128 byte-exact),
///  - the spot lane is a STRICT FUNCTION of the rasterised coverage
///    — verified by computing the expected lane value from the
///    reference geometry's centre pixel only (where coverage = 255).
///
/// Pre-fix behaviour: the diff branch used a byte inequality on
/// snapshot vs post-paint pixmap; with /Half ca 0.5 the painted
/// region had pix_alpha = 128 (a "change"), so every painted pixel
/// — interior AND AA-edge — got coverage = 255. The lane at every
/// covered pixel was 128 (compose_normal at α=0.5·1=0.5 against 0).
/// The fix changes the AA-edge pixels (and any identical-RGB-collided
/// pixels) but leaves the centre byte-identical at 128.
///
/// The geometry is a tilted strip — a rectangle that the rasteriser
/// must AA along all four edges. Its centre pixel is guaranteed full
/// coverage; pixels near its tilted edges are fractional. We pin the
/// centre as the canonical test surface.
///
/// QA_BUG_SPOT_MIRROR_AA_EDGE_BINARY_COVERAGE (fixed for combo paints).
#[test]
fn qa3b_combo_fillstroke_aa_edge_gets_fractional_coverage_after_fix() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    // A full-page filled+stroked rectangle exercised by the `B`
    // combo. The interior centre pixel is full coverage; the edges
    // are AA but here we pin the centre invariant. The probe's
    // round-1 pre-fix would have produced an identical centre value
    // — the fix's contribution is byte-exact AA at the edges. We
    // assert the centre stays byte-exact so the regression guard
    // holds against accidental coverage scaling errors. ~keep
    let content = "/Half gs\n\
                   /CS_PMS cs\n1.0 scn\n/CS_PMS CS 1.0 SCN\n\
                   1 w\n10 10 80 80 re\nB\n";
    let resources = format!(
        "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;

    // Fill side: full coverage at the centre → t_r = (1-0.5)·0 +
    // 0.5·1.0 = 0.5 → u8 128. The stroke arm composes on top: the
    // stroke geometry is just the rectangle outline at the page
    // edges, which doesn't touch the centre pixel — so the centre
    // stays at the fill-side composed value 128. ~keep
    let expected_centre = tint_to_u8(compose_normal(0.0, 1.0, 0.5));
    assert_eq!(expected_centre, 128);
    assert_eq!(
        plane[centre], expected_centre,
        "{} — combo `B` centre pixel uses the rasterised fill \
         coverage. At full coverage and α=0.5: t_r = 0.5·1.0 = u8 \
         {}. Got {} at centre.",
        QA_BUG_SPOT_MIRROR_AA_EDGE_BINARY_COVERAGE, expected_centre, plane[centre]
    );

    // Now probe an explicit AA edge: a pixel just outside the
    // rectangle's nominal extent (x = 90, y = 50). With the fix,
    // the rasterised coverage at exactly the boundary may be 0 or
    // a small fraction depending on the rasteriser's pixel-centre
    // rule. We pin: the spot lane at a pixel just OUTSIDE the
    // rectangle (x = 91, y = 50) is ZERO (no fill coverage there,
    // no stroke contribution since stroke width=1 only covers x
    // ∈ {89, 90, 91} approximately). Under the pre-fix diff branch
    // the pixel at x=91 (interior to the stroke) would receive
    // coverage = 255 and lane u8 128; under the fix it receives
    // the rasteriser's actual coverage (possibly fractional).
    //
    // Rather than pin a specific fractional value (rasteriser-
    // dependent), pin a STRUCTURAL invariant: at least one pixel
    // along the rectangle's right edge has lane value STRICTLY
    // BETWEEN 0 and 128 — proving the coverage is fractional, not
    // binary. Under the pre-fix diff branch, every painted pixel
    // landed at exactly 128. ~keep
    let mut fractional_count = 0usize;
    for y in 8..92 {
        for x in 88..94 {
            let off = y * dims.0 as usize + x;
            let v = plane[off];
            if (1..128).contains(&v) {
                fractional_count += 1;
            }
        }
    }
    assert!(
        fractional_count > 0,
        "{} — the spot lane must show STRICTLY FRACTIONAL coverage \
         (lane ∈ (0, 128)) at AA-edge pixels along the rectangle's \
         right edge. Got 0 fractional pixels in the search range — \
         indicates the diff branch (binary coverage) is still \
         firing. Expected the rasterised fill / stroke coverage \
         path to write fractional lane values.",
        QA_BUG_SPOT_MIRROR_AA_EDGE_BINARY_COVERAGE
    );
}

/// `/CS_Sep_A scn 0.5 /CS_Sep_B f` — between the `scn` that sets
/// /Separation /InkA at tint 0.5 and the `f` that paints, a `cs
/// /CS_Sep_B` switches to a different /Separation space (/InkB)
/// without an intervening `scn`. Per ISO 32000-1 §8.6.8 + §8.6.6.4
/// the current colour reverts to its initial value: for /Separation
/// the initial tint is **1.0** for each colorant.
///
/// EXPECTED: paint writes to lane B at tint 1.0 composed via /Normal
/// at α=0.5 → t_r = (1-0.5)·0 + 0.5·1.0 = 0.5 → u8 = 128. Lane A is
/// unsourced under the preserve-backdrop policy → stays at zero.
///
/// CURRENT: the round-2 impl now resets `fill_spot_inks` on `cs` per
/// §8.6.8; this probe pins the spec-correct behaviour.
///
/// QA_BUG_CS_DOES_NOT_RESET_SPOT_IDENTITY (fixed).
#[test]
fn qa4_cs_without_scn_resets_spot_identity_to_initial_full_tint() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    // Paint sequence:
    // 1. /CS_A cs           → fill_spot_inks=[(InkA, 1.0)] per §8.6.8
    //                        (initial tint 1.0; CS_A is /Separation /InkA).
    // 2. /CS_A scn 0.5      → fill_spot_inks=[(InkA, 0.5)] per scn.
    // 3. /CS_B cs           → switches space to InkB; §8.6.8 resets
    //                        the colour to initial (tint 1.0 on /InkB).
    // 4. f                  → writes lane B at /Normal(0, 1.0, α=0.5)
    //                        = 0.5 → u8 128. Lane A is unsourced and
    //                        preserved at zero under HONEST_GAP_SPOT_
    //                        LANE_UNSOURCED_PRESERVE_BACKDROP.
    // Use /Half ca 0.5 to allocate the sidecar (transparency trigger). ~keep
    let content = "/Half gs\n\
                   /CS_A cs\n0.5 scn\n\
                   /CS_B cs\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
         /ColorSpace << /CS_A [/Separation /InkA /DeviceCMYK {} ] \
                        /CS_B [/Separation /InkB /DeviceCMYK {} ] >>",
        psfunc, psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let names = renderer.cmyk_sidecar_spot_names().expect("sidecar present");
    assert_eq!(names, &["InkA".to_string(), "InkB".to_string()]);

    let plane_a = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    assert!(
        plane_a.iter().all(|&b| b == 0),
        "{} — lane A is unsourced after `cs /CS_B`. Active space at f \
         is /CS_B → /InkB. Lane A stays at zero. First non-zero \
         offset: {:?}",
        QA_BUG_CS_DOES_NOT_RESET_SPOT_IDENTITY,
        plane_a.iter().position(|&b| b != 0)
    );

    // Lane B: sourced from /CS_B's reset-to-initial tint 1.0,
    // composed via /Normal at α=0.5 against backdrop zero → 0.5 →
    // u8 128. ~keep
    let plane_b = renderer.cmyk_sidecar_spot_plane(1).expect("InkB plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    let expected_b = tint_to_u8(compose_normal(0.0, 1.0, 0.5));
    assert_eq!(expected_b, 128);
    assert_eq!(
        plane_b[centre], expected_b,
        "{} — `cs /CS_B` resets the colour to /CS_B's initial value \
         per §8.6.8. For /Separation /InkB the initial tint is 1.0 \
         per §8.6.6.4. At α=0.5 the spot lane composes to 0.5 → u8 \
         {}. Got {} at centre.",
        QA_BUG_CS_DOES_NOT_RESET_SPOT_IDENTITY, expected_b, plane_b[centre]
    );
}

/// Follow-up to qa4: confirm that a `k` operator (DeviceCMYK
/// setter) correctly clears prior spot identity. This is the
/// inverse of qa4 and pins the agent's claim that the device-family
/// setters clear `fill_spot_inks`.
#[test]
fn qa4b_device_family_setter_clears_prior_spot_identity() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    // Set /CS_A /InkA at tint 0.5 (no paint), then DeviceCMYK k
    // setter, then f. After k, fill_spot_inks must be empty so the
    // f does NOT write to lane A. ~keep
    let content = "/Half gs\n\
                   /CS_A cs\n0.5 scn\n\
                   0 0 0 0.3 k\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
         /ColorSpace << /CS_A [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    assert!(
        plane.iter().all(|&b| b == 0),
        "ISO 32000-1 §8.6.8: the `k` DeviceCMYK setter clears prior \
         /Separation spot identity. lane A must stay at backdrop \
         zero. First non-zero offset: {:?}",
        plane.iter().position(|&b| b != 0)
    );
}

/// A /Separation paint whose alternate-CS RGB happens to be exactly
/// identical to the backdrop RGB at every pixel. The diff branch
/// (used by combo/text/Do/sh) computes "changed pixel" via byte
/// inequality on R, G, B, and A. When R, G, B, AND A are all
/// unchanged, the diff records coverage = 0 and the spot lane is
/// NOT written.
///
/// Setup: backdrop is a DeviceCMYK (0,0,0,0) paint at α=1 →
/// pixmap is (paper white, alpha 255). The /Separation /InkA paint
/// uses a tint transform whose C0/C1 are both (0,0,0,0) — so at any
/// tint the alternate-CS RGB is also paper white. The diff sees
/// no change at any pixel. Result: spot lane stays at zero, even
/// though /InkA was painted at a positive tint.
///
/// This is an edge case that real artwork hits when a designer
/// paints a spot over an identical-RGB region (e.g., a white-on-
/// white spot varnish that the alternate process approximation
/// renders as paper white).
///
/// QA_BUG_SPOT_MIRROR_IDENTICAL_RGB_COLLISION (fixed for combo paints).
#[test]
fn qa5_identical_rgb_paint_via_combo_writes_spot_lane_after_fix() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 0.0 0.0 0.0] /N 1 >>";
    let content = "/Half gs\n\
                   0 0 0 0 k\n0 0 100 100 re\nf\n\
                   /CS_PMS cs\n/CS_PMS CS 0.7 scn 0.7 SCN\n\
                   0 0 100 100 re\nB\n";
    let resources = format!(
        "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    // Spec-correct: α=0.5, t_s=0.7 → t_r = 0.5·0.7 = 0.35 → u8 = 89. ~keep
    let expected = tint_to_u8(compose_normal(0.0, 0.7, 0.5));
    assert_eq!(expected, 89);
    assert_eq!(
        plane[centre], expected,
        "QA_BUG_SPOT_MIRROR_IDENTICAL_RGB_COLLISION: a /Separation \
         /InkA paint whose alternate-CS RGB collides with backdrop \
         RGB must still write the spot lane. Spec value: {} (= \
         0.5·0.7·255). Got {} at centre. If the value is 0, the diff \
         branch missed the paint because no RGB/A bytes changed.",
        expected, plane[centre]
    );
}

/// `/Separation /InkA scn 0.5 f` with `/BM /Multiply` and NO prior
/// /DeviceCMYK setter: `gs.fill_color_cmyk` is `None` (Separation
/// sources do not populate it). The spot mirror must still fire.
///
/// This pins the agent's claim that the spot mirror is independent
/// of `fill_color_cmyk`.
#[test]
fn qa6_spot_mirror_fires_when_fill_color_cmyk_is_none() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    // First paint at tint 0.8 lays the backdrop. Second paint with
    // /BM /Multiply at tint 0.5 — separable+WP → spot lane runs
    // /Multiply per §11.7.4.2. fill_color_cmyk stays None throughout
    // because /Separation does not populate it. ~keep
    let content = "/CS_PMS cs\n0.8 scn\n0 0 100 100 re\nf\n\
                   /Mult gs\n0.5 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Mult << /Type /ExtGState /BM /Multiply >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    // B(0.8, 0.5) = 1 - 0.2·0.5 = 0.9 → t_r at α=1 = 0.9 → u8 = 230. ~keep
    let expected = 230u8;
    assert_eq!(
        plane[centre], expected,
        "ISO 32000-1 §11.7.4.2: a /Separation paint's spot mirror \
         fires independently of fill_color_cmyk (None for Separation \
         sources). /Multiply(0.8, 0.5) = 0.9 → u8 = {}. Got {}.",
        expected, plane[centre]
    );
}

/// `/DeviceN [/InkA /InkB]` with tints (0.5, 0.7) and `/BM /Difference`.
/// /Difference is separable but non-white-preserving → §11.7.4.2
/// substitutes /Normal on EVERY spot lane (both InkA and InkB).
///
/// The existing P4 covers single-spot Separation; this probe pins
/// the multi-spot DeviceN form to verify both lanes get the
/// substitution, not just the first.
#[test]
fn qa7_multi_spot_devicen_non_wp_bm_substitutes_normal_on_every_lane() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc4 = "<< /FunctionType 4 /Domain [0 1 0 1] \
                   /Range [0 1 0 1 0 1 0 1] /Length 28 >>\n\
                   stream\n{0 0 0 0}\nendstream\nendobj\n";
    // First paint at (0.6, 0.4) lays down backdrop via /Normal.
    // Second paint at (0.5, 0.7) with /BM /Difference — non-WP → spot
    // lanes substitute /Normal. Both lanes overwrite to source tints. ~keep
    let content = "/CS_DN cs\n0.6 0.4 scn\n0 0 100 100 re\nf\n\
                   /Diff gs\n0.5 0.7 scn\n0 0 100 100 re\nf\n";
    let resources = "/ExtGState << /Diff << /Type /ExtGState /BM /Difference >> >> \
                     /ColorSpace << /CS_DN [/DeviceN [/InkA /InkB] /DeviceCMYK 6 0 R] >>";
    let extra = format!("6 0 obj\n{}", psfunc4);
    let pdf = build_pdf_with_output_intent(content, resources, &icc, &[extra.as_bytes()]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let names = renderer.cmyk_sidecar_spot_names().expect("sidecar present");
    assert_eq!(names, &["InkA".to_string(), "InkB".to_string()]);

    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    let expected_a = tint_to_u8(0.5);
    let expected_b = tint_to_u8(0.7);
    let plane_a = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let plane_b = renderer.cmyk_sidecar_spot_plane(1).expect("InkB plane");
    assert_eq!(
        plane_a[centre], expected_a,
        "ISO 32000-1 §11.7.4.2: /Difference is non-WP → /Normal \
         substituted on EVERY spot lane. Lane A: source 0.5 at α=1 = \
         0.5 → u8 {}. Got {}. /Difference computed value would be \
         |0.6 - 0.5| = 0.1 → u8 26.",
        expected_a, plane_a[centre]
    );
    assert_eq!(
        plane_b[centre], expected_b,
        "Lane B: source 0.7 at α=1 = 0.7 → u8 {}. Got {}. /Difference \
         computed value would be |0.4 - 0.7| = 0.3 → u8 77.",
        expected_b, plane_b[centre]
    );
}

/// A spot named with `#XX` hex escape (e.g., `/PANTONE#20185#20C` →
/// "PANTONE 185 C") must surface in `fill_spot_inks` with the
/// DECODED name, and the sidecar lookup must match by decoded name.
///
/// Round-1 QA already verified the spot set surfaces the decoded
/// name; this probe pins that the round-2 paint mirror writes to
/// the correct lane.
#[test]
fn qa8_hex_escaped_spot_name_writes_decoded_lane() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 1.0 0.0] /N 1 >>";
    // `/PANTONE#20185#20C` → "PANTONE 185 C". ~keep
    let content = "/Half gs\n\
                   /CS_PMS cs\n0.7 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
         /ColorSpace << /CS_PMS [/Separation /PANTONE#20185#20C /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let names = renderer.cmyk_sidecar_spot_names().expect("sidecar present");
    assert_eq!(names, &["PANTONE 185 C".to_string()]);

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("PANTONE plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    // α = 0.5, t_s = 0.7 → 0.35 → u8 = 89. ~keep
    let expected = tint_to_u8(compose_normal(0.0, 0.7, 0.5));
    assert_eq!(expected, 89);
    assert_eq!(
        plane[centre], expected,
        "ISO 32000-1 §8.6.6.4 + §7.3.5 name decoding: the spot \
         /PANTONE#20185#20C decodes to 'PANTONE 185 C'. The paint \
         mirror's `fill_spot_inks` carry must use the decoded name, \
         and the sidecar lookup matches by decoded name. Expected u8 \
         = {}, got {}.",
        expected, plane[centre]
    );
}

// Split for file-too-long (#1567): the module continues below via `include!`,
// not a sibling tests/*_part2.rs file -- Cargo auto-discovers every *.rs file
// directly under tests/ as its own test binary, so a sibling would fail to
// compile on its own and, if it somehow did, run every #[test] in it twice.
// A subdirectory is not auto-discovered. ~keep
include!("test_46_round2_qa_pass/part2.rs");
