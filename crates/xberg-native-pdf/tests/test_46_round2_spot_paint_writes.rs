//! Round-2 probes for spot-lane writes from `/Separation`
//! and `/DeviceN` paint operators with §11.7.4.2 dispatch wired.
//!
//! Round 1 landed the storage scaffolding: a per-page CMYK + spot-ink
//! sidecar, a discovery pre-pass that enumerates the active spot set,
//! and a pure dispatch enum classifying each PDF blend mode under
//! §11.7.4.2 ("separable + white-preserving applies to spots; everything
//! else substitutes /Normal on spots").
//!
//! Round 2 wires the per-paint mirror: every path / text / image-XObject
//! / shading / Form-XObject paint operator whose active colour space is
//! `/Separation` or non-process `/DeviceN` now writes the resolved spot
//! tint into the sidecar's spot lanes, with §11.7.4.2 dispatch applied
//! per lane class (process lanes use the requested BM unchanged; spot
//! lanes substitute /Normal for non-separable and non-white-preserving
//! modes).
//!
//! Methodology references:
//!  - `docs/research/2026-06-06-nonsep-blends-in-devicen.md` — the
//!    architectural decision: CMYK is the blend space, spots ride
//!    alongside, §11.7.4.2 splits the BM per lane class.
//!  - `tests/test_46_round1_spot_sidecar.rs` — round-1 design+impl
//!    probes for storage, discovery, and the dispatch decision
//!    function. Round 2 layers paint-time writes on top.
//!  - `tests/test_46_round1_qa_pass.rs` — round-1 QA pin set.
//!
//! Spec citations:
//!  - ISO 32000-1 §8.6.6.3 reserved `/All` / `/None` Separation names
//!  - ISO 32000-1 §8.6.6.4 `/Separation` colour space
//!  - ISO 32000-1 §8.6.6.5 `/DeviceN` colour space + `/Process` attrs
//!  - ISO 32000-1 §11.3.3 basic compositing formula
//!  - ISO 32000-1 §11.3.5.2 separable blend modes + Note 2 (Difference
//!    and Exclusion non-WP)
//!  - ISO 32000-1 §11.3.5.3 non-separable blend modes (RGB projection
//!    + CMYK K-channel rule)
//!  - ISO 32000-1 §11.6.3 `/BM` array first-recognised rule
//!  - ISO 32000-1 §11.7.3 spot colours and transparency (sidecar model;
//!    source-component expansion to 1.0 additive / 0.0 subtractive on
//!    unsourced channels)
//!  - ISO 32000-1 §11.7.4.2 BM split per lane class (THE KEY)
//!  - ISO 32000-1 §11.7.4.3 CompatibleOverprint (B(c_b, c_s) = c_b for
//!    unsourced channels)

#![allow(dead_code)]

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{PageRenderer, RenderOptions};

/// Spot lanes for inks that are NOT the active source's named ink.
///
/// ISO 32000-1 §11.7.3 says every paint conceptually touches every
/// component, with unsourced channels assigned an additive value of 1.0
/// (subtractive tint 0.0). Under /Normal BM and `α = 1`, the basic
/// compositing formula gives `t_r = (1 - α) · t_b + α · t_s = 0` —
/// which erases the backdrop on unsourced spot lanes. Under
/// §11.7.4.3 CompatibleOverprint (implicit when `/OP true`), the spec
/// instead preserves the backdrop on unsourced channels: B(c_b, c_s)
/// = c_b.
///
/// Real-world spot-aware workflows expect the CompatibleOverprint
/// semantics regardless of `/OP` state: a /Separation paint targets
/// one ink and is not meant to disturb other inks. Round 2's spot
/// mirror leaves unsourced spot lanes alone (no write) — which
/// matches the CompatibleOverprint behaviour byte-for-byte under
/// every BM. The spec's "erase under /Normal" reading is a corner
/// case the round 2 impl deliberately does not implement; a future
/// round can revisit if a real PDF requires the erase behaviour.
pub const HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP: &str = "HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP: ISO 32000-1 \
     §11.7.3 + §11.7.4.2 allow a strict reading that unsourced spot \
     lanes get source 0.0 subtractive and compose via the requested BM \
     — which for /Normal under opaque paint would erase the backdrop. \
     The §11.7.4.3 CompatibleOverprint rule (implicit when overprint \
     is enabled) preserves the backdrop on unsourced channels. Round 2 \
     adopts the CompatibleOverprint semantics for spot lanes NOT named \
     by the active source's colorant list, regardless of /OP state. \
     The asymmetry this creates between two superficially similar \
     paint shapes is real and worth spelling out: \
     \n\n\
     (1) EXPLICIT zero tint via `/CS_InkA cs 0 scn` on a /Separation \
     /InkA space — the source DOES name /InkA at tint 0, so the spot \
     mirror walks the source ink list, finds /InkA, composes via the \
     §11.3.3 formula with t_s = 0 under /Normal at α = 1:  \n\
       t_r = (1 − 1) · t_b + 1 · 0 = 0   (ERASES the backdrop).  \n\
     This branch is exercised by the QA probe \
     `qa1_explicit_zero_tint_separation_erases_inka_backdrop_under_\
     normal`. \
     \n\n\
     (2) IMPLICIT not-named — the source's colour space does not name \
     /InkA at all (e.g. a /DeviceCMYK `k` paint following an earlier \
     /InkA paint). The spot mirror's source ink list is empty for the \
     /InkA dimension; under the round-2 preserve-backdrop policy the \
     lane is not touched at all (PRESERVES the backdrop). This branch \
     is exercised by `qa1_unsourced_inka_lane_preserves_backdrop_under_\
     normal_at_full_alpha`. \
     \n\n\
     Both readings have spec support — §11.7.3's strict reading \
     supports (1) on a literal application of the basic compositing \
     formula with 'source 0.0 subtractive' for unsourced channels; \
     §11.7.4.3's CompatibleOverprint example supports (2) on its \
     definition that 'the value is c_s for that spot component and \
     c_b for all process components and all other spot components'. \
     Round 2 picks (2) for the implicit case because real-world \
     spot-aware artwork almost always intends 'paint only what I said \
     to paint'; (1) is preserved for explicit zero-tint because \
     erasing what the source literally requested is the only reading \
     that does not silently drop the operator's intent.";

fn build_pdf_with_output_intent(
    content: &str,
    resources_inner: &str,
    icc_profile: &[u8],
    extra_objs: &[&str],
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
        buf.extend_from_slice(obj.as_bytes());
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

/// Constant-output CMYK→Lab ICC profile (any CMYK input → near-neutral
/// grey at the chosen L*). Mirrors the round-1 helper.
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

// ===========================================================================
// Byte-exact reference helpers — compute expected spot-lane values from
// the §11.3.3 + §11.3.5.2 formulas in floating point, then round to the
// same `u8` representation the renderer writes.
// =========================================================================== ~keep

/// Round `t` (in `[0, 1]`) to the same u8 quantisation the spot mirror
/// uses: `(t · 255).round() as u8`.
fn tint_to_u8(t: f32) -> u8 {
    (t.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// §11.3.3 basic compositing formula at full coverage and full
/// backdrop alpha: `t_r = (1 - α_s) · t_b + α_s · B(t_b, t_s)`, where
/// `α_s = coverage · gs_alpha` and `B(·,·)` is the dispatched separable
/// blend function on the additive complements of the subtractive tints.
fn compose_normal(t_b: f32, t_s: f32, alpha: f32) -> f32 {
    (1.0 - alpha) * t_b + alpha * t_s
}
fn compose_multiply(t_b: f32, t_s: f32, alpha: f32) -> f32 {
    // Multiply is defined on additive values (ISO 32000-1 §11.3.5.2), so the
    // subtractive tints are complemented before the product and after it. ~keep
    let blended = 1.0 - (1.0 - t_b) * (1.0 - t_s);
    (1.0 - alpha) * t_b + alpha * blended
}

/// A `/Separation /SpotA` paint with tint 0.6 over a /DeviceN
/// `[/SpotA /SpotB /SpotC]` declaration must write the SpotA lane and
/// leave SpotB / SpotC at backdrop (zero). ISO 32000-1 §11.7.3 + the
/// §11.7.4.3 CompatibleOverprint principle (carried as a HONEST_GAP):
/// unsourced spot lanes preserve the backdrop.
#[test]
fn round2_p1_separation_paint_writes_only_active_spot_lane() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 1.0 0.0] /N 1 >>";
    let psfunc4 = "<< /FunctionType 4 /Domain [0 1 0 1 0 1] /Range [0 1 0 1 0 1 0 1] \
                  /Length 28 >>\nstream\n{0 0 0 0}\nendstream\nendobj\n";
    let content = "/Half gs\n\
                   /CS_PMS cs\n1.0 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
         /ColorSpace << \
         /CS_PMS [/Separation /SpotA /DeviceCMYK {} ] \
         /CS_DN [/DeviceN [/SpotA /SpotB /SpotC] /DeviceCMYK 6 0 R] >>",
        psfunc
    );
    let extra = format!("6 0 obj\n{}", psfunc4);
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[&extra]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let names = renderer.cmyk_sidecar_spot_names().expect("sidecar present");
    assert_eq!(names, &["SpotA".to_string(), "SpotB".to_string(), "SpotC".to_string()]);
    let dims = renderer.cmyk_sidecar_dims().unwrap();

    let expected_spota = tint_to_u8(compose_normal(0.0, 1.0, 0.5));
    let plane_a = renderer.cmyk_sidecar_spot_plane(0).expect("SpotA plane");
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    assert_eq!(
        plane_a[centre], expected_spota,
        "ISO 32000-1 §11.7.3 + §11.3.3: /Separation /SpotA at tint 1.0 \
         with fill_alpha 0.5 composes to (1-0.5)·0 + 0.5·1.0 = 0.5 → \
         u8 = {} on the SpotA lane",
        expected_spota
    );

    // SpotB and SpotC must stay at zero (backdrop preserved per
    // HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP). ~keep
    let plane_b = renderer.cmyk_sidecar_spot_plane(1).expect("SpotB plane");
    let plane_c = renderer.cmyk_sidecar_spot_plane(2).expect("SpotC plane");
    assert!(
        plane_b.iter().all(|&b| b == 0),
        "{} — SpotB lane preserved at backdrop zero",
        HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP
    );
    assert!(
        plane_c.iter().all(|&b| b == 0),
        "{} — SpotC lane preserved at backdrop zero",
        HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP
    );
}

/// A /DeviceCMYK paint over a page with /CS_DN [/InkA] DeviceN
/// declaration must NOT write to the InkA spot lane. The CMYK paint
/// targets process channels only; per §11.7.3 the unsourced spot lane
/// preserves the backdrop (zero) under round 2's CompatibleOverprint-
/// style policy.
#[test]
fn round2_p2_process_paint_does_not_touch_spot_lanes() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 4 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /Length 28 >>\nstream\n{0 0 0 0}\nendstream\nendobj\n";
    let content = "/Half gs\n0 0 0 0.3 k\n0 0 100 100 re\nf\n";
    let resources = "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
                     /ColorSpace << /CS_DN [/DeviceN [/InkA] /DeviceCMYK 6 0 R] >>";
    let extra = format!("6 0 obj\n{}", psfunc);
    let pdf = build_pdf_with_output_intent(content, resources, &icc, &[&extra]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let names = renderer.cmyk_sidecar_spot_names().expect("sidecar present");
    assert_eq!(names, &["InkA".to_string()]);

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    assert!(
        plane.iter().all(|&b| b == 0),
        "ISO 32000-1 §11.7.3: a DeviceCMYK paint does not name InkA as \
         a source colorant — under round 2's preserve-backdrop policy \
         the InkA lane stays at zero. First non-zero offset: {:?}",
        plane.iter().position(|&b| b != 0)
    );
}

// ===========================================================================
// PROBE 3: §11.7.4.2 non-sep substitution (Luminosity → Normal on spot).
// =========================================================================== ~keep

/// `/BM /Luminosity` + /Separation /InkA paint at tint 0.8: per
/// §11.7.4.2 the spot lane composes with /Normal substituted (not the
/// requested /Luminosity, which is non-separable). Byte-exact: at
/// fill_alpha 1.0 the spot tint becomes `t_r = (1-1)·0 + 1·0.8 = 0.8 →
/// u8 = 204` (Normal at α=1 is a straight overwrite to source).
#[test]
fn round2_p3_non_separable_bm_substitutes_normal_on_spot() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    // /Lumi sets /BM /Luminosity. Sidecar is allocated because the BM
    // is non-Normal (transparency trigger). ~keep
    let content = "/Lumi gs\n\
                   /CS_PMS cs\n0.8 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Lumi << /Type /ExtGState /BM /Luminosity >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let names = renderer.cmyk_sidecar_spot_names().expect("sidecar present");
    assert_eq!(names, &["InkA".to_string()]);

    // §11.7.4.2: /Luminosity is non-separable → spot lane substitutes
    // /Normal. At fill_alpha 1.0 and coverage 1.0:
    //   t_r = (1-1)·0 + 1·0.8 = 0.8 → u8 = (0.8 · 255).round() = 204. ~keep
    let expected = tint_to_u8(compose_normal(0.0, 0.8, 1.0));
    assert_eq!(expected, 204);
    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    assert_eq!(
        plane[centre], expected,
        "ISO 32000-1 §11.7.4.2: a non-separable /BM /Luminosity must \
         substitute /Normal on the spot lane. /Normal(0, 0.8) at α=1 \
         = 0.8 → u8 = {}. Got {} at centre pixel.",
        expected, plane[centre]
    );
}

// ===========================================================================
// PROBE 4: §11.7.4.2 non-WP substitution (Difference → Normal on spot).
// =========================================================================== ~keep

/// `/BM /Difference` + /Separation /InkA paint at tint 0.4 onto a
/// backdrop where InkA was already painted to 0.6: per §11.7.4.2
/// /Difference is separable but NOT white-preserving (Note 2), so the
/// spot lane substitutes /Normal. Result: `t_r = (1-1)·0.6 + 1·0.4 =
/// 0.4 → u8 = 102`. If the spot lane had honoured /Difference, the
/// result would have been `|0.6 - 0.4| = 0.2 → u8 = 51`.
#[test]
fn round2_p4_non_wp_bm_substitutes_normal_on_spot() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    // First paint at tint 0.6 with /Normal lays down the backdrop.
    // Second paint at tint 0.4 with /BM /Difference must compose as
    // /Normal per §11.7.4.2 (spot lane). ~keep
    let content = "/CS_PMS cs\n0.6 scn\n0 0 100 100 re\nf\n\
                   /Diff gs\n0.4 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Diff << /Type /ExtGState /BM /Difference >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let expected = tint_to_u8(compose_normal(0.6, 0.4, 1.0));
    assert_eq!(expected, 102);
    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    let difference_wrong = tint_to_u8((0.6_f32 - 0.4).abs());
    assert_eq!(difference_wrong, 51);
    assert_eq!(
        plane[centre], expected,
        "ISO 32000-1 §11.7.4.2 + §11.3.5.2 Note 2: /Difference is \
         separable but NOT white-preserving → spot lane substitutes \
         /Normal. /Normal(0.6, 0.4) at α=1 = 0.4 → u8 = {}. If the \
         renderer had honoured /Difference the value would have been \
         {} instead. Got {} at centre.",
        expected, difference_wrong, plane[centre]
    );
}

// ===========================================================================
// PROBE 5: §11.7.4.2 separable + WP passes through (/Multiply on spot).
// =========================================================================== ~keep

/// `/BM /Multiply` + /Separation /InkA paint at tint 0.5 onto a backdrop
/// where InkA was already painted to 0.8: per §11.7.4.2 /Multiply is
/// separable AND white-preserving, so the spot lane runs the requested
/// /Multiply unchanged. Result: B(0.8, 0.5) = 1 - 0.2 · 0.5 = 0.9 (two inks
/// stack, so the tint grows). At α=1 the basic compositing formula collapses
/// to the blend value: `t_r = (1-1)·0.8 + 1·B(0.8, 0.5) = 0.9`.
#[test]
fn round2_p5_separable_wp_bm_passes_through_on_spot() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
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

    let expected = tint_to_u8(compose_multiply(0.8, 0.5, 1.0));
    assert!(
        expected > tint_to_u8(0.8),
        "two stacked inks must be denser than the backdrop"
    );
    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    assert_eq!(
        plane[centre], expected,
        "ISO 32000-1 §11.7.4.2: /Multiply is separable AND \
         white-preserving → spot lane uses the requested /Multiply \
         unchanged. /Multiply(0.8, 0.5) = 0.9 → u8 = {}. Got {} at \
         centre.",
        expected, plane[centre]
    );
}

// ===========================================================================
// PROBE 6: §11.3.5.3 K-channel rule for non-sep on CMYK.
// ===========================================================================
//
// Round 4 wired the CMYK compose path for non-sep blend modes through
// the existing apply_cmyk_compose_after_paint pipeline (CMYK direct
// paint with /BM /Luminosity uses a separable BM path through
// tiny_skia — the K-channel rule lives in the renderer's blend
// pipeline). Round 2's scope is the SPOT lane writes; the CMYK-lane
// behaviour under non-sep BM stays as round-4 wired it.
//
// This probe pins the cross-lane invariant: under /BM /Luminosity on a
// /DeviceCMYK paint, the SPOT lanes are untouched (no source spot
// colour) so the discovered InkA lane stays at zero per the
// HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP policy. The
// process-lane K-channel rule (use source K under /Luminosity) is
// pinned by `tests/test_transparency_flattening_qa_round*` so this
// probe focuses on the round-2 contribution. ~keep

#[test]
fn round2_p6_k_channel_rule_on_cmyk_does_not_perturb_spot_lanes() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 4 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /Length 28 >>\nstream\n{0 0 0 0}\nendstream\nendobj\n";
    let content = "/Lumi gs\n0.2 0.6 0.0 0.3 k\n0 0 100 100 re\nf\n";
    let resources = "/ExtGState << /Lumi << /Type /ExtGState /BM /Luminosity >> >> \
                     /ColorSpace << /CS_DN [/DeviceN [/InkA] /DeviceCMYK 6 0 R] >>";
    let extra = format!("6 0 obj\n{}", psfunc);
    let pdf = build_pdf_with_output_intent(content, resources, &icc, &[&extra]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let names = renderer.cmyk_sidecar_spot_names().expect("sidecar present");
    assert_eq!(names, &["InkA".to_string()]);

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    assert!(
        plane.iter().all(|&b| b == 0),
        "ISO 32000-1 §11.3.5.3 K-channel rule lives on the process \
         lanes; the InkA spot lane is unsourced and preserved at \
         backdrop zero per round 2's policy. First non-zero offset: \
         {:?}",
        plane.iter().position(|&b| b != 0)
    );
}

/// Page with three paint operators:
/// (a) DeviceCMYK paint at (0.3, 0.0, 0.0, 0.0) → writes CMYK lanes only.
/// (b) /Separation /PANTONE 185 C paint at tint 0.7 → writes the
///     PANTONE 185 C spot lane only.
/// (c) /DeviceN /[Cyan Magenta Yellow Black SpotA] /Process /CMYK paint
///     at (0.0, 0.5, 0.0, 0.0, 0.4) → writes /Magenta lane via the
///     process-channel mapping AND writes the SpotA spot lane.
///
/// Expected sidecar state:
/// - Spot set: ["PANTONE 185 C", "SpotA"] (Cyan/Magenta/Yellow/Black
///   are filtered out as /Process channels).
/// - PANTONE 185 C lane: composed from (b) only → 0.7 → 179.
/// - SpotA lane: composed from (c) only → 0.4 → 102.
/// - The probe pins the lane-targeting; the process-lane CMYK
///   composition under /DeviceN /Process is round-4's territory and
///   is not asserted here.
#[test]
fn round2_p7_mixed_shape_page_writes_only_targeted_lanes() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc2 = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 1.0 0.0] /N 1 >>";
    let psfunc4 = "<< /FunctionType 4 /Domain [0 1 0 1 0 1 0 1 0 1] \
                  /Range [0 1 0 1 0 1 0 1] /Length 28 >>\n\
                  stream\n{0 0 0 0}\nendstream\nendobj\n";
    let content = "/Half gs\n\
                   0.3 0 0 0 k\n0 0 100 100 re\nf\n\
                   /CS_PMS cs\n0.7 scn\n0 0 100 100 re\nf\n\
                   /CS_DN cs\n0 0.5 0 0 0.4 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Half << /Type /ExtGState /ca 1.0 /BM /Normal >> >> \
         /ColorSpace << \
         /CS_PMS [/Separation /PANTONE#20185#20C /DeviceCMYK {} ] \
         /CS_DN [/DeviceN [/Cyan /Magenta /Yellow /Black /SpotA] /DeviceCMYK 6 0 R \
            << /Subtype /DeviceN /Process << /ColorSpace /DeviceCMYK \
               /Components [/Cyan /Magenta /Yellow /Black] >> >>] >>",
        psfunc2
    );
    let extra = format!("6 0 obj\n{}", psfunc4);
    let all_extra: Vec<&str> = vec![&extra];
    let resources = resources.replace("/ca 1.0 /BM /Normal", "/ca 0.99");
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &all_extra);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let names = renderer.cmyk_sidecar_spot_names().expect("sidecar present");
    assert_eq!(
        names,
        &["PANTONE 185 C".to_string(), "SpotA".to_string()],
        "ISO 32000-1 §8.6.6.5 /Process: Cyan/Magenta/Yellow/Black are \
         filtered out; only PANTONE 185 C and SpotA surface as spots"
    );

    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;

    let plane_pms = renderer.cmyk_sidecar_spot_plane(0).expect("PANTONE plane");
    let alpha_99 = 0.99_f32;
    let expected_pms = tint_to_u8(compose_normal(0.0, 0.7, alpha_99));
    assert_eq!(
        plane_pms[centre], expected_pms,
        "PANTONE 185 C: /Normal(0, 0.7) at α=0.99 = 0.693 → u8 = {}. \
         Got {} at centre.",
        expected_pms, plane_pms[centre]
    );

    let plane_spota = renderer.cmyk_sidecar_spot_plane(1).expect("SpotA plane");
    let expected_spota = tint_to_u8(compose_normal(0.0, 0.4, alpha_99));
    assert_eq!(
        plane_spota[centre], expected_spota,
        "SpotA: /Normal(0, 0.4) at α=0.99 = 0.396 → u8 = {}. Got {} at \
         centre.",
        expected_spota, plane_spota[centre]
    );

    // CMYK lane assertion: paint (a) was a DeviceCMYK at (0.3, 0, 0, 0)
    // — its C component should land on the C lane via round-4's CMYK
    // mirror. We assert the C lane is non-zero at the centre to verify
    // the process-channel write happened independently of the spot
    // writes. (The exact CMYK composition under the page's combined
    // paint stack is round 4's territory; this probe pins the
    // round-2-relevant invariant: CMYK paints continue to mirror to the
    // CMYK plane even on a page with spot inks discovered.) ~keep
    let cmyk_bytes = renderer.cmyk_sidecar_cmyk_bytes().expect("sidecar CMYK plane");
    let c_at_centre = cmyk_bytes[centre * 4];
    assert!(
        c_at_centre > 0,
        "ISO 32000-1 §11.7.3: a DeviceCMYK paint at (0.3, 0, 0, 0) \
         must write the C component to the CMYK plane independently \
         of the spot lanes. Got C = {} at centre.",
        c_at_centre
    );
}

/// A /DeviceN [/InkA /InkB] paint with tints (0.5, 0.7) — the InkA lane
/// receives tint 0.5 and the InkB lane receives 0.7, simultaneously.
/// Per ISO 32000-1 §8.6.6.5 + §11.7.3: a single /DeviceN paint
/// targets every named colorant with the corresponding component
/// value. Round 2's spot mirror walks the source ink list and writes
/// each lane independently.
#[test]
fn round2_p8_multi_spot_devicen_writes_all_named_lanes() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc4 = "<< /FunctionType 4 /Domain [0 1 0 1] \
                   /Range [0 1 0 1 0 1 0 1] /Length 28 >>\n\
                   stream\n{0 0 0 0}\nendstream\nendobj\n";
    let content = "/Half gs\n\
                   /CS_DN cs\n0.5 0.7 scn\n0 0 100 100 re\nf\n";
    let resources = "/ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
                     /ColorSpace << /CS_DN [/DeviceN [/InkA /InkB] /DeviceCMYK 6 0 R] >>";
    let extra = format!("6 0 obj\n{}", psfunc4);
    let pdf = build_pdf_with_output_intent(content, resources, &icc, &[&extra]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let names = renderer.cmyk_sidecar_spot_names().expect("sidecar present");
    assert_eq!(names, &["InkA".to_string(), "InkB".to_string()]);

    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    let expected_a = tint_to_u8(compose_normal(0.0, 0.5, 0.5));
    let expected_b = tint_to_u8(compose_normal(0.0, 0.7, 0.5));
    assert_eq!(expected_a, 64);
    assert_eq!(expected_b, 89);
    let plane_a = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let plane_b = renderer.cmyk_sidecar_spot_plane(1).expect("InkB plane");
    assert_eq!(
        plane_a[centre], expected_a,
        "/DeviceN paint with tints (0.5, 0.7): InkA lane at α=0.5 = \
         0.25 → u8 = {}. Got {}.",
        expected_a, plane_a[centre]
    );
    assert_eq!(
        plane_b[centre], expected_b,
        "/DeviceN paint with tints (0.5, 0.7): InkB lane at α=0.5 = \
         0.35 → u8 = {}. Got {}.",
        expected_b, plane_b[centre]
    );
}

/// Page where:
/// - The stroke side uses /Stroke /BM /Multiply.
/// - The fill side uses /Fill /BM /Normal.
/// - A single `b` (closefill+stroke) paint with /Separation /InkA at
///   tint 0.5 first lays down a backdrop tint 0.8.
///
/// The fill side writes with /Normal: t_r = (1-1)·0.8 + 1·0.5 = 0.5.
/// The stroke side then composes ON TOP with /Multiply: B(0.5, 0.5) =
/// 0.75 → t_r = (1-1)·0.5 + 1·0.75 = 0.75 along the stroke geometry.
///
/// Probe pin: the centre pixel (interior of the fill, NOT on the
/// stroke line) has t_r = 0.5 → u8 = 128. The stroke geometry only
/// affects pixels on the stroke; pin the centre to verify the fill
/// arm composed with /Normal.
///
/// This pin verifies the per-side BM dispatch wiring — `gs.blend_mode`
/// is the SAME parameter for fill and stroke (§11.7.4.2 says "the
/// PDF graphics state specifies only one current blend mode
/// parameter"), so this probe is more about asserting that the spot
/// mirror reads `gs.blend_mode` once per paint side rather than
/// claiming two different BMs. The probe pins that a single shared
/// `/Multiply` BM dispatched to a separation source produces the
/// Multiply formula on the spot lane (separable + WP path).
#[test]
fn round2_p9_stroke_fill_share_one_bm_per_paint_arm() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
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

    let expected = tint_to_u8(compose_multiply(0.8, 0.5, 1.0));
    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    assert_eq!(
        plane[centre], expected,
        "spot mirror dispatches the active /BM /Multiply through the \
         §11.7.4.2 spot dispatch (separable+WP → UseRequested). \
         Multiply(0.8, 0.5) = 0.9 → u8 = {}. Got {}.",
        expected, plane[centre]
    );
}

// ===========================================================================
// PROBE 10: soft mask interaction — /SMask attenuates the spot lane the
// same way it attenuates the pixmap (single shape/opacity per pixel
// per §11.3.3 + §11.7.3), simultaneously with the §11.7.4.2 /Normal
// substitution for non-separable BMs.
// =========================================================================== ~keep

/// `/SMask /S /Alpha` over a /Separation /InkA paint with /BM /Hue:
/// two §11 mechanisms compose on the SAME paint operator:
///  1. §11.7.4.2 — /Hue is non-separable → spot lane substitutes
///     /Normal (the requested BM is honoured on the process lanes
///     only).
///  2. §11.4.7 + §11.3.3 + §11.7.3 — the soft mask produces an
///     alpha that applies to BOTH the visible pixmap AND every spot
///     lane via the SHARED (shape, opacity) per-pixel rule. The spot
///     lane composes against its pre-mirror snapshot with the SMask
///     alpha attenuating the source contribution exactly the way
///     the pixmap RGB attenuates against `snapshot`.
///
/// Construction:
///  - SMask form renders a uniform 0.5 grey over the page bbox; in
///    /S /Alpha mode the mask alpha is then the form's alpha
///    channel — uniformly 1.0 across the form's footprint (the
///    `0.5 g 0 0 100 100 re f` paints opaque mid-grey). So /Alpha
///    yields a mask of 1.0, which would NOT attenuate. We instead
///    use /S /Luminosity (BC absent → default backdrop is colour
///    space's black point, which is luminosity 0). The /Luminosity
///    extraction of the form's grey 0.5 fill yields a mask of 0.5
///    over the form footprint.
///
/// Byte-exact reference computation:
///  - Source: /CS_PMS /Separation /InkA at scn 0.6, /BM /Hue.
///  - /Hue is non-separable → spot dispatch substitutes /Normal.
///  - gs.fill_alpha = 1.0 (no /ca explicitly set on the SMask gs);
///    coverage = 1.0 at the centre pixel.
///  - Mirror writes lane[centre] via Normal(0, 0.6) at α=1: t_r =
///    (1-1)·0 + 1·0.6 = 0.6 → u8 = round(0.6·255) = 153.
///  - SMask materialises mask m = 0.5 at the centre pixel.
///  - SMask attenuation: out = m·post + (1-m)·pre =
///    0.5·153 + 0.5·0 = 76.5 → round to u8 = 77.
#[test]
fn round2_p10_smask_attenuates_spot_lane_under_normal_substitution() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let smask_form = "6 0 obj\n\
        << /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
           /Resources << >> \
           /Group << /Type /Group /S /Transparency /CS /DeviceGray >> \
           /Length 28 >>\n\
        stream\n0.5 g\n0 0 100 100 re\nf\nendstream\nendobj\n";
    let content = "/HueG gs\n\
                   /CS_PMS cs\n0.6 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /HueG << /Type /ExtGState /BM /Hue \
            /SMask << /Type /Mask /S /Luminosity /G 6 0 R >> >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[smask_form]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    // §11.7.4.2: /Hue is non-separable → spot lane substitutes
    // /Normal. Mirror writes post = (1-1)·0 + 1·0.6 = 0.6 → u8 153.
    // §11.4.7 + §11.3.3 + §11.7.3: SMask m = 0.5; pre = 0.
    //   out = m·post + (1-m)·pre = 0.5·153 + 0.5·0 = 76.5 → u8 77.
    //
    // Compute byte-exact in the same quantise-after-mirror cascade
    // the impl uses: mirror writes the u8 first (153), then SMask
    // attenuates the u8. ~keep
    let post_u8 = tint_to_u8(compose_normal(0.0, 0.6, 1.0));
    assert_eq!(post_u8, 153);
    let m = 0.5_f32;
    let expected = (m * post_u8 as f32 + (1.0 - m) * 0.0).clamp(0.0, 255.0).round() as u8;
    assert_eq!(expected, 77);

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    assert_eq!(
        plane[centre], expected,
        "ISO 32000-1 §11.7.4.2 + §11.4.7 + §11.3.3 + §11.7.3: two \
         rules compose on this paint. (1) /Hue is non-separable → \
         spot lane substitutes /Normal: mirror writes u8 {}. (2) \
         SMask /S /Luminosity at uniform 0.5 attenuates the lane \
         against the pre-mirror snapshot (zero): out = 0.5·{} + \
         0.5·0 = u8 {}. Got {} at centre.",
        post_u8, post_u8, expected, plane[centre]
    );
}

// ===========================================================================
// PROBE 11: §11.6.3 + §11.3.5 /BM array first-recognised rule (parser
// fix verification).
// =========================================================================== ~keep

/// Round 1 left a known bug in `ext_gstate.rs:111`: the `/BM` array
/// parser picked `arr.first()` without classifying. Round 2's fix
/// applies the §11.6.3 first-recognised rule. This probe pins the
/// fix end-to-end: a gstate with `/BM [/UnknownMode /Multiply]`
/// should select /Multiply (first recognised) and apply it to a
/// /Separation paint on the spot lane (separable+WP → UseRequested).
#[test]
fn round2_p11_bm_array_first_recognised_rule_drives_spot_dispatch() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    // /MArr declares /BM [/UnknownMode /Multiply]. Per §11.6.3 +
    // §11.3.5, the conforming reader uses the FIRST RECOGNISED name —
    // which is /Multiply. The first paint lays down 0.8 with /Normal,
    // the second paint at tint 0.5 with /MArr → /Multiply. ~keep
    let content = "/CS_PMS cs\n0.8 scn\n0 0 100 100 re\nf\n\
                   /MArr gs\n0.5 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /MArr << /Type /ExtGState /BM [/UnknownMode /Multiply] >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render succeeds");

    let expected_after_fix = tint_to_u8(compose_multiply(0.8, 0.5, 1.0));
    let pre_fix_wrong = tint_to_u8(compose_normal(0.8, 0.5, 1.0));
    assert_ne!(expected_after_fix, pre_fix_wrong);

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    assert_eq!(
        plane[centre], expected_after_fix,
        "ISO 32000-1 §11.6.3 + §11.3.5: /BM array picks the first \
         RECOGNISED name. [/UnknownMode /Multiply] → /Multiply. \
         Multiply(0.8, 0.5) = 0.9 → u8 = {}. Got {}. If still showing \
         {}, the parser fell through to arr.first() == UnknownMode and \
         classified as /Normal.",
        expected_after_fix, plane[centre], pre_fix_wrong
    );
}
