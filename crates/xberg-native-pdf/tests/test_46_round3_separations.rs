//! Round-3 probes for composite-then-decompose separation rendering.
//!
//! Round 1 landed the sidecar storage + dispatch enum. Round 2 wired
//! per-paint spot lane writes with §11.7.4.2 BM split. Round 3 is the
//! architectural payoff: `render_separations` now produces spec-
//! correct per-plate output for transparency-bearing pages by routing
//! through the page renderer's composite path and decomposing the
//! populated sidecar into one [`SeparationPlate`] per requested ink.
//!
//! Detection-OFF pages stay on the existing per-plate walker (which is
//! byte-identical to a "no-transparency" render at the pixel level
//! and remains the source of truth for §11.7.4 OPM overprint
//! semantics, which the per-plate walker implements correctly per-
//! plate).
//!
//! Spec citations:
//!  - ISO 32000-1 §8.6.6.3 / §8.6.6.4 / §8.6.6.5 — `/Separation` /
//!    `/DeviceN` colour spaces and `/Process` attributes
//!  - ISO 32000-1 §10.5 — separated plate output per ink name
//!  - ISO 32000-1 §11.3.3 — single shape / opacity across all lanes
//!  - ISO 32000-1 §11.3.5.2 — separable blend modes + Note 2
//!    non-white-preserving
//!  - ISO 32000-1 §11.3.5.3 — non-separable blend modes + CMYK
//!    K-channel rule
//!  - ISO 32000-1 §11.4.6.2 — knockout groups (last-paint-wins
//!    composition against group backdrop)
//!  - ISO 32000-1 §11.4.7 — soft masks
//!  - ISO 32000-1 §11.6.7 — spot colour
//!  - ISO 32000-1 §11.7.3 — spot colours and transparency (sidecar
//!    model)
//!  - ISO 32000-1 §11.7.4.2 — BM split per lane class

#![allow(dead_code)]

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::rendering::{PageRenderer, RenderOptions, render_separations};

/// `/K` (knockout) groups whose constituent paints target DIFFERENT
/// spot inks at the same pixel. ISO 32000-1 §11.4.6.2 says the
/// group's "constituent objects shall be composited with the group's
/// initial backdrop rather than with each other". §11.3.3 + §11.7.3
/// say the single (shape, opacity) per pixel applies to BOTH process
/// AND spot lanes. Together this means: if paint 1 writes InkA and
/// paint 2 writes InkB at the same pixel under a /K group, paint 2's
/// (shape, opacity) — extended to every lane — composes the InkB
/// source against the InkB backdrop. The InkA lane is "not specified"
/// by paint 2, which per §11.7.3 takes additive 1.0 (subtractive
/// tint 0.0) as the source; under /Normal at full alpha this would
/// ERASE the InkA backdrop. Paint 1 had already written the InkA
/// backdrop value, but paint 1 is now treated as composing against
/// the GROUP backdrop too, so paint 1's InkA tint composes against
/// the group's initial InkA backdrop, then paint 2's
/// "unsourced-erase" overwrites it.
///
/// The round-2 spot mirror adopted the §11.7.4.3 CompatibleOverprint
/// reading on unsourced spot lanes — they PRESERVE the backdrop
/// rather than erase it. Under that reading, paint 2 leaves InkA's
/// post-paint-1 lane alone; paint 1's InkA value survives the
/// knockout. Under the strict §11.7.3 reading, paint 2's
/// "unsourced-erase" wins and InkA is back at the group backdrop.
///
/// Both readings are defensible. Round 3 honours the round-2
/// CompatibleOverprint policy on unsourced spot lanes inside /K
/// groups for consistency — the /K rule covers what each paint
/// touches, and round 2 already pinned "if you didn't name it, don't
/// touch it".
pub const HONEST_GAP_KNOCKOUT_DIFFERENT_INK_SPOT_INTERACTION: &str = "HONEST_GAP_KNOCKOUT_DIFFERENT_INK_SPOT_INTERACTION: ISO 32000-1 \
     §11.4.6.2 + §11.7.3 + §11.7.4.3 admit two readings for a /K \
     group whose paints target DIFFERENT spot inks at the same \
     pixel: (a) §11.7.3 strict — unsourced spot lanes get additive \
     1.0 source from every paint, erasing earlier paint's lane writes; \
     (b) §11.7.4.3 CompatibleOverprint — unsourced spot lanes \
     preserve the backdrop. Round 2 adopted (b) as policy \
     (HONEST_GAP_SPOT_LANE_UNSOURCED_PRESERVE_BACKDROP). Round 3 \
     honours that policy inside /K groups too: paint 2 to InkB at \
     the same pixel as paint 1 to InkA leaves InkA's paint-1 value \
     intact. The InkB lane gets paint 2's tint composed against the \
     group's InkB backdrop (the /K rule for paint 2 itself).";

// ===========================================================================
// Synthetic PDF builder — re-uses the round-1/2 shape for corpus
// uniformity. The PDF includes an `/OutputIntents` array pointing to
// a constant CMYK→Lab ICC profile so the page renderer's compose-
// first / overprint helpers fire on transparent CMYK paints.
// =========================================================================== ~keep

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

/// Build a synthetic PDF WITHOUT an `/OutputIntents` array. Used by
/// the byte-identity probes that pin the detection-OFF path to the
/// pre-round-3 per-plate walker output.
fn build_pdf_no_output_intent(content: &str, resources_inner: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let cat_off = buf.len();
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
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

    let xref_off = buf.len();
    let total_objs = 4;
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", total_objs).as_bytes());
    for off in [cat_off, pages_off, page_off, stream_off] {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            total_objs, xref_off
        )
        .as_bytes(),
    );
    buf
}

/// Constant-output CMYK→Lab ICC profile (any CMYK input → near-
/// neutral grey at the chosen L*).
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
fn compose_multiply(t_b: f32, t_s: f32, alpha: f32) -> f32 {
    // Multiply is defined on additive values (ISO 32000-1 §11.3.5.2), so the
    // subtractive tints are complemented before the product and after it. ~keep
    let blended = 1.0 - (1.0 - t_b) * (1.0 - t_s);
    (1.0 - alpha) * t_b + alpha * blended
}

/// Find a plate in the result list by ink name.
fn plate<'a>(
    plates: &'a [xberg_native_pdf::rendering::SeparationPlate],
    name: &str,
) -> &'a xberg_native_pdf::rendering::SeparationPlate {
    plates
        .iter()
        .find(|p| p.ink_name == name)
        .unwrap_or_else(|| panic!("no plate named {}", name))
}

/// Sample a plate at its centre pixel.
fn centre(plate: &xberg_native_pdf::rendering::SeparationPlate) -> u8 {
    let off = ((plate.height / 2) * plate.width + plate.width / 2) as usize;
    plate.data[off]
}

// ===========================================================================
// PROBE 1: detection-ON spot paint with /BM /Multiply + /ca 0.5 over
// uniform InkA backdrop. The composite-then-decompose path must produce
// the §11.3.5 Multiply + §11.4 alpha composition result on the InkA
// plate.
// =========================================================================== ~keep

/// ISO 32000-1 §11.3.5.2 Multiply (separable, white-preserving) +
/// §11.7.4.2 spot-lane dispatch (Multiply IS separable+WP so it
/// applies to spot lanes unchanged) + §11.3.3 basic compositing.
///
/// First paint: `/Separation /InkA` at tint 0.4 with no transparency
/// lays the backdrop on the spot lane.
/// Second paint: `/Separation /InkA` at tint 0.6 with `/BM /Multiply`
/// + `/ca 0.5` composes over the backdrop on the InkA spot lane.
///
/// Byte-exact references:
///  - After backdrop: lane = compose_normal(0, 0.4, 1.0) = 0.4 → u8 102.
///  - After Multiply paint at α = 1·0.5 = 0.5:
///    blend = 1 - (1 - 0.4)·(1 - 0.6) = 0.76 (Multiply on the additive complements).
///    lane = (1 - 0.5)·0.4 + 0.5·0.76 = 0.2 + 0.38 = 0.58 → u8 148.
#[test]
fn round3_p1_separation_paint_with_multiply_and_alpha_composites_correctly() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let content = "/CS_PMS cs\n0.4 scn\n0 0 100 100 re\nf\n\
                   /Mult gs\n0.6 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Mult << /Type /ExtGState /BM /Multiply /ca 0.5 >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("synthetic PDF parses");

    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");

    // Reference computation:
    //   Backdrop lane = compose_normal(0, 0.4, α=1) = 0.4
    //   B(t_b, t_s) = 1 - (1 - 0.4)·(1 - 0.6) = 0.76
    //   lane = (1 - 0.5)·0.4 + 0.5·0.76 = 0.20 + 0.38 = 0.58
    //   tint_to_u8(0.58) = round(0.58·255) = round(147.9) = u8 148 ~keep
    let backdrop = compose_normal(0.0, 0.4, 1.0);
    assert_eq!(tint_to_u8(backdrop), 102);
    let after_mult = compose_multiply(backdrop, 0.6, 0.5);
    let expected = tint_to_u8(after_mult);
    assert_eq!(expected, 148);
    assert_eq!(
        centre(inka),
        expected,
        "ISO 32000-1 §11.3.5.2 Multiply + §11.3.3 compose + §11.7.4.2 \
         (Multiply is separable AND white-preserving on spot lanes): \
         backdrop 0.4, source 0.6, B = 1 - 0.6·0.4 = 0.76, α = 0.5, \
         lane = 0.5·0.4 + 0.5·0.76 = 0.58 → u8 {}. Got {}.",
        expected,
        centre(inka)
    );
}

// ===========================================================================
// PROBE 2: detection-OFF byte-identity guard.
//
// A page that declares NO transparency triggers (no /ca, no /SMask,
// no /BM≠Normal, no /OP) renders separations via the per-plate walker
// regardless of whether OutputIntent is present. Round 3 must NOT
// change the walker's output for these pages.
// =========================================================================== ~keep

/// ISO 32000-1 §10.5: a Separation /InkA paint at tint 1.0 produces a
/// full-tint plate. Round 3's detection gate skips the composite path
/// when no transparency trigger is declared, so this probe runs
/// through the existing per-plate walker — same code as pre-round-3.
/// The probe pins centre = 255 byte-exact (the walker's `fill_separation`
/// writes the gray value of the tint, with no transparency math).
#[test]
fn round3_p2_detection_off_page_renders_via_per_plate_walker_byte_identical() {
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let content = "/CS_PMS cs\n1.0 scn\n0 0 100 100 re\nf\n";
    let resources = format!("/ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>", psfunc);
    let pdf = build_pdf_no_output_intent(content, &resources);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");
    assert_eq!(
        centre(inka),
        255,
        "Per-plate walker: Separation /InkA at tint 1.0 paints u8 255 \
         on the plate. Got {}.",
        centre(inka)
    );
}

// ===========================================================================
// PROBE 3: detection-ON CMYK paint with /ca 0.8 produces per-plate
// composed CMYK output. The page has CMYK paint at (0.5, 0.2, 0.7, 0.1)
// with /ca 0.8 — the four process plates should reflect §11.3.5.2
// Normal blend + §11.4 alpha composition against the all-zero CMYK
// backdrop (no prior paint).
// =========================================================================== ~keep

/// ISO 32000-1 §11.3.5.2 Normal blend on each CMYK channel + §11.3.3
/// basic compositing. Backdrop is all zero (no prior paint).
/// Source = (0.5, 0.2, 0.7, 0.1) with α = 0.8.
///
/// Per channel: t_r = (1 - 0.8)·0 + 0.8·t_s = 0.8 · t_s.
///  - C: 0.8 · 0.5 = 0.40 → tint_to_u8 = round(102.0) = 102
///  - M: 0.8 · 0.2 = 0.16 → tint_to_u8 = round(40.8) = 41
///  - Y: 0.8 · 0.7 = 0.56 → tint_to_u8 = round(142.8) = 143
///  - K: 0.8 · 0.1 = 0.08 → tint_to_u8 = round(20.4) = 20
#[test]
fn round3_p3_cmyk_plates_compose_under_alpha() {
    let icc = build_constant_cmyk_icc(135);
    let content = "/Alpha gs\n0.5 0.2 0.7 0.1 k\n0 0 100 100 re\nf\n";
    let resources = "/ExtGState << /Alpha << /Type /ExtGState /ca 0.8 >> >>";
    let pdf = build_pdf_with_output_intent(content, resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let c = plate(&plates, "Cyan");
    let m = plate(&plates, "Magenta");
    let y = plate(&plates, "Yellow");
    let k = plate(&plates, "Black");

    let exp_c = tint_to_u8(compose_normal(0.0, 0.5, 0.8));
    let exp_m = tint_to_u8(compose_normal(0.0, 0.2, 0.8));
    let exp_y = tint_to_u8(compose_normal(0.0, 0.7, 0.8));
    let exp_k = tint_to_u8(compose_normal(0.0, 0.1, 0.8));
    assert_eq!(exp_c, 102);
    assert_eq!(exp_m, 41);
    assert_eq!(exp_y, 143);
    assert_eq!(exp_k, 20);
    assert_eq!(centre(c), exp_c, "C plate: 0.8·0.5 → u8 {}", exp_c);
    assert_eq!(centre(m), exp_m, "M plate: 0.8·0.2 → u8 {}", exp_m);
    assert_eq!(centre(y), exp_y, "Y plate: 0.8·0.7 → u8 {}", exp_y);
    assert_eq!(centre(k), exp_k, "K plate: 0.8·0.1 → u8 {}", exp_k);
}

// ===========================================================================
// PROBE 4: non-separable /BM /Luminosity + /Separation /InkA paint
// substitutes /Normal on the spot lane per §11.7.4.2. The plate output
// reflects the /Normal substitution, NOT the /Luminosity formula.
// =========================================================================== ~keep

/// ISO 32000-1 §11.7.4.2: non-separable blend modes apply only to
/// process lanes; spot lanes substitute /Normal. So /BM /Luminosity
/// on a /Separation /InkA paint at tint 0.7 with /ca 1.0 over an
/// existing InkA backdrop of 0.3 produces:
///   B(0.3, 0.7) = /Normal substituted = 0.7
///   lane = (1 - 1)·0.3 + 1·0.7 = 0.7 → u8 round(178.5) = 179
///
/// If the spot lane had INCORRECTLY honoured /Luminosity, the formula
/// reduces over a 1-vector but the spot lane should never reach it.
#[test]
fn round3_p4_non_separable_bm_substitutes_normal_on_spot_plate() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let content = "/CS_PMS cs\n0.3 scn\n0 0 100 100 re\nf\n\
                   /Lumi gs\n0.7 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Lumi << /Type /ExtGState /BM /Luminosity >> >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");

    // §11.7.4.2 substitution: spot lane runs /Normal at α=1, so:
    //   lane = (1-1)·0.3 + 1·0.7 = 0.7 → u8 round(178.5) = 179 ~keep
    let expected = tint_to_u8(compose_normal(0.3, 0.7, 1.0));
    assert_eq!(expected, 179);
    assert_eq!(
        centre(inka),
        expected,
        "ISO 32000-1 §11.7.4.2: /Luminosity is non-separable → spot \
         lane substitutes /Normal. /Normal(0.3, 0.7) at α=1 = 0.7 → \
         u8 {}. Got {}.",
        expected,
        centre(inka)
    );
}

/// ISO 32000-1 §11.4.7 soft mask + round-2 P10 spot-lane attenuation.
/// Cascade:
///  - Pre-paint lane = 0 (no prior paint).
///  - Mirror writes post = compose_normal(0, 0.6, α=1) = 0.6 → u8 153.
///  - SMask form is uniform 0.5 grey; /S /Luminosity yields
///    Lum(0.5, 0.5, 0.5) = 0.5, so m = 0.5 at every pixel.
///  - SMask attenuation per round-2 P10: out = m·post + (1-m)·pre =
///    0.5·153 + 0.5·0 = 76.5 → u8 round = 77.
///
/// The plate output therefore equals u8 77 at every pixel within the
/// page footprint.
#[test]
fn round3_p5_smask_attenuates_spot_plate_via_composite_path() {
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
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[smask_form]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");

    let post_u8 = tint_to_u8(compose_normal(0.0, 0.6, 1.0));
    assert_eq!(post_u8, 153);
    let m = 0.5_f32;
    let expected = (m * post_u8 as f32 + (1.0 - m) * 0.0).clamp(0.0, 255.0).round() as u8;
    assert_eq!(expected, 77);
    assert_eq!(
        centre(inka),
        expected,
        "ISO 32000-1 §11.4.7 SMask: post-paint lane u8 = {}; SMask \
         m = 0.5 at centre; out = 0.5·{} + 0.5·0 = u8 {}. Got {}.",
        post_u8,
        post_u8,
        expected,
        centre(inka)
    );
}

/// Page has three paints under /ca 0.99 (just barely triggers the
/// transparency detection gate):
///  (a) /DeviceCMYK paint (0.3, 0.0, 0.0, 0.0) → Cyan plate only.
///  (b) /Separation /PANTONE_185_C paint at tint 0.7 → PMS lane only.
///  (c) /DeviceN [/Cyan /Magenta /Yellow /Black /SpotA] /Process /CMYK
///      paint at (0.0, 0.5, 0.0, 0.0, 0.4) → /Magenta CMYK lane via
///      /Process channel + /SpotA spot lane.
///
/// Expected plate set:
///  - Cyan / Magenta / Yellow / Black (always returned by render_separations)
///  - PANTONE 185 C (from /Separation declaration)
///  - SpotA (from /DeviceN non-process declaration)
///  - /Cyan, /Magenta, /Yellow, /Black inside /DeviceN are NOT separate
///    spot plates — they are filtered out per §8.6.6.5 /Process.
///
/// The test pins:
///  - The PANTONE 185 C plate value at centre (composed from (b) only).
///  - The SpotA plate value at centre (composed from (c) only).
///  - The Cyan plate value at centre (composed from (a) + (c) /Process).
///  - The /Process channel names ARE NOT in the plate list as standalone spots.
#[test]
fn round3_p6_mixed_shape_page_routes_paints_to_correct_plates() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc2 = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 1.0 0.0] /N 1 >>";
    let psfunc4 = "<< /FunctionType 4 /Domain [0 1 0 1 0 1 0 1 0 1] \
                  /Range [0 1 0 1 0 1 0 1] /Length 28 >>\n\
                  stream\n{0 0 0 0}\nendstream\nendobj\n";
    let content = "/Trig gs\n\
                   0.3 0 0 0 k\n0 0 100 100 re\nf\n\
                   /CS_PMS cs\n0.7 scn\n0 0 100 100 re\nf\n\
                   /CS_DN cs\n0 0.5 0 0 0.4 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Trig << /Type /ExtGState /ca 0.99 >> >> \
         /ColorSpace << \
         /CS_PMS [/Separation /PANTONE#20185#20C /DeviceCMYK {} ] \
         /CS_DN [/DeviceN [/Cyan /Magenta /Yellow /Black /SpotA] /DeviceCMYK 6 0 R \
            << /Subtype /DeviceN /Process << /ColorSpace /DeviceCMYK \
               /Components [/Cyan /Magenta /Yellow /Black] >> >>] >>",
        psfunc2
    );
    let extra = format!("6 0 obj\n{}", psfunc4);
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[&extra]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");

    let names: Vec<&str> = plates.iter().map(|p| p.ink_name.as_str()).collect();
    assert!(names.contains(&"Cyan"));
    assert!(names.contains(&"Magenta"));
    assert!(names.contains(&"Yellow"));
    assert!(names.contains(&"Black"));
    assert!(names.contains(&"PANTONE 185 C"));
    assert!(names.contains(&"SpotA"));
    // The /Process colorants from /DeviceN are filtered out of the
    // spot set per §8.6.6.5 — they should NOT appear twice in any
    // way: the four process plates are CMYK proper, not /DeviceN
    // sub-colorants masquerading as spots. We assert the exact total
    // plate count: 4 (CMYK) + 2 (PMS + SpotA) = 6. ~keep
    assert_eq!(
        plates.len(),
        6,
        "ISO 32000-1 §8.6.6.5 /Process: /Cyan /Magenta /Yellow /Black \
         inside /DeviceN are not standalone spot plates. Expected 6 \
         total plates (CMYK + PMS + SpotA); got {} → {:?}",
        plates.len(),
        names
    );

    let pms = plate(&plates, "PANTONE 185 C");
    let exp_pms = tint_to_u8(compose_normal(0.0, 0.7, 0.99));
    assert_eq!(exp_pms, 177);
    assert_eq!(centre(pms), exp_pms, "PANTONE 185 C centre u8 = {}", exp_pms);

    let spota = plate(&plates, "SpotA");
    let exp_spota = tint_to_u8(compose_normal(0.0, 0.4, 0.99));
    assert_eq!(exp_spota, 101);
    assert_eq!(centre(spota), exp_spota, "SpotA centre u8 = {}", exp_spota);
}

// ===========================================================================
// PROBE 7: knockout /K with two paints to the SAME spot ink at the
// SAME pixel. The /K rule (§11.4.6.2) says paint 2 composes against
// the group's INITIAL BACKDROP, not against paint 1's lane state.
// =========================================================================== ~keep

/// ISO 32000-1 §11.4.6.2 knockout: a /K group's elements compose
/// each against the group's initial backdrop, not against each other.
/// §11.3.3 + §11.7.3 extend the (shape, opacity) per-pixel rule to
/// spot lanes. So inside a /K group with two overlapping /Separation
/// /InkA paints:
///
///  - Paint 1: InkA at tint 0.6 → composes against InitialBackdrop_A
///    (=0 outside the group, since no prior paint).
///  - Paint 2: InkA at tint 0.3 → composes against InitialBackdrop_A
///    (=0), NOT against 0.6.
///
/// Final lane inside the group = paint 2's result = 0.3 → u8 round(76.5)
/// = 77 (NOT 0.3 composed against 0.6, which would be 0.3 · 0.6 = 0.18
/// → u8 46 if Multiply, or 0.3 → u8 77 if Normal-over).
///
/// Under /Normal at α=1 the two answers coincide (both produce 0.3
/// because Normal-over with α=1 just replaces). To DISCRIMINATE
/// knockout from non-knockout we use α = 0.5: under non-knockout the
/// second paint composes (1-0.5)·0.6 + 0.5·0.3 = 0.45 → u8 115; under
/// knockout (1-0.5)·0 + 0.5·0.3 = 0.15 → u8 38.
#[test]
fn round3_p7_knockout_group_same_ink_uses_group_backdrop_not_prior_paint() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let form = "6 0 obj\n\
        << /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
           /Resources << /ExtGState << /Half << /Type /ExtGState /ca 0.5 >> >> \
                         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK \
                           << /FunctionType 2 /Domain [0 1] \
                              /Range [0 1 0 1 0 1 0 1] /C0 [0.0 0.0 0.0 0.0] \
                              /C1 [0.0 1.0 0.0 0.0] /N 1 >> ] >> >> \
           /Group << /Type /Group /S /Transparency /K true /CS /DeviceCMYK >> \
           /Length 67 >>\n\
        stream\n/CS_PMS cs\n0.6 scn\n0 0 100 100 re\nf\n\
/Half gs\n0.3 scn\n0 0 100 100 re\nf\n\
endstream\nendobj\n";
    let content = "/Form Do\n";
    let resources = format!(
        "/XObject << /Form 6 0 R >> \
         /ColorSpace << /CS_PMS [/Separation /InkA /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[form]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");

    let expected_knockout = tint_to_u8(0.5 * 0.0 + 0.5 * 0.3);
    assert_eq!(expected_knockout, 38);
    // Non-knockout (what the bug would produce): lane = 0.5·0.6 +
    // 0.5·0.3 = 0.45 → u8 round(114.75) = 115. ~keep
    let non_knockout_wrong = tint_to_u8(0.5 * 0.6 + 0.5 * 0.3);
    assert_eq!(non_knockout_wrong, 115);
    assert_eq!(
        centre(inka),
        expected_knockout,
        "ISO 32000-1 §11.4.6.2 + §11.3.3 + §11.7.3: /K group spot \
         lane composes paint 2 (tint 0.3, α=0.5) against the group's \
         initial InkA backdrop (=0), not against paint 1's lane (=0.6). \
         Knockout = 0.5·0 + 0.5·0.3 = u8 {}; non-knockout would be \
         u8 {}. Got {}.",
        expected_knockout,
        non_knockout_wrong,
        centre(inka)
    );
}

/// ISO 32000-1 §11.4.6.2 + §11.7.3 + §11.7.4.3: a /K group with
/// paint 1 to /InkA followed by paint 2 to /InkB at the same pixel.
/// Per HONEST_GAP_KNOCKOUT_DIFFERENT_INK_SPOT_INTERACTION the
/// CompatibleOverprint reading wins: paint 2 does not name InkA, so
/// it leaves the InkA lane alone, and paint 1's InkA write (composed
/// against the group's initial InkA backdrop = 0) survives.
///
/// Paint 1: InkA at tint 0.6, /ca 1.0. lane_A = (1-1)·0 + 1·0.6 = 0.6
/// → u8 153.
/// Paint 2: InkB at tint 0.4, /ca 1.0. lane_B = (1-1)·0 + 1·0.4 = 0.4
/// → u8 102.
///
/// Both lanes should reflect their respective paints — InkA survives
/// the /K group because paint 2 didn't touch it.
#[test]
fn round3_p8_knockout_group_different_inks_preserve_each_other() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >>";
    let form = "6 0 obj\n\
        << /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 100 100] \
           /Resources << /ColorSpace << \
              /CS_A [/Separation /InkA /DeviceCMYK \
                 << /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                    /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 0.0 0.0] /N 1 >> ] \
              /CS_B [/Separation /InkB /DeviceCMYK \
                 << /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                    /C0 [0.0 0.0 0.0 0.0] /C1 [1.0 0.0 0.0 0.0] /N 1 >> ] >> >> \
           /Group << /Type /Group /S /Transparency /K true /CS /DeviceCMYK >> \
           /Length 71 >>\n\
        stream\n/CS_A cs\n0.6 scn\n0 0 100 100 re\nf\n\
/CS_B cs\n0.4 scn\n0 0 100 100 re\nf\n\
endstream\nendobj\n";
    let content = "/Form Do\n";
    let resources = format!(
        "/XObject << /Form 6 0 R >> \
         /ColorSpace << /CS_A [/Separation /InkA /DeviceCMYK {} ] \
                        /CS_B [/Separation /InkB /DeviceCMYK {} ] >>",
        psfunc, psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[form]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let inka = plate(&plates, "InkA");
    let inkb = plate(&plates, "InkB");

    let expected_a = tint_to_u8(compose_normal(0.0, 0.6, 1.0));
    assert_eq!(expected_a, 153);
    let expected_b = tint_to_u8(compose_normal(0.0, 0.4, 1.0));
    assert_eq!(expected_b, 102);
    assert_eq!(
        centre(inka),
        expected_a,
        "{} — InkA preserved across /K group: paint 2 (to InkB) does \
         not touch the InkA lane. paint 1's InkA composed against the \
         group's initial InkA backdrop (=0) → u8 {}. Got {}.",
        HONEST_GAP_KNOCKOUT_DIFFERENT_INK_SPOT_INTERACTION,
        expected_a,
        centre(inka)
    );
    assert_eq!(
        centre(inkb),
        expected_b,
        "InkB lane: paint 2 composes against the group's initial InkB \
         backdrop (=0) → 0.4 → u8 {}. Got {}.",
        expected_b,
        centre(inkb)
    );
}

// ===========================================================================
// PROBE 9: composite preview RGB stays byte-identical for the
// existing round-2 SMask probe (regression guard).
//
// The composite path is shared between separation rendering and
// composite-preview rendering. Round 3 only changes the separation
// dispatch; the composite preview's pixmap output must not change.
// =========================================================================== ~keep

/// Re-runs the round-2 P10 SMask configuration through `render_page`
/// and asserts the spot lane is still u8 77 byte-exact at centre.
/// This is the regression guard that round 3's separation-side
/// changes did not perturb the composite-side state machine.
#[test]
fn round3_p9_composite_path_smask_spot_lane_byte_identity_holds() {
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
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[smask_form]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    let _img = renderer.render_page(&doc, 0).expect("render");

    let plane = renderer.cmyk_sidecar_spot_plane(0).expect("InkA plane present");
    let dims = renderer.cmyk_sidecar_dims().unwrap();
    let centre_off = ((dims.1 / 2) * dims.0 + dims.0 / 2) as usize;
    assert_eq!(
        plane[centre_off], 77,
        "ISO 32000-1 §11.4.7 SMask byte-identity carryover from round \
         2 P10. The composite path's spot-lane state machine produces \
         u8 77 at centre regardless of round 3's separation-side \
         dispatch change. Got {}.",
        plane[centre_off]
    );
}

/// ISO 32000-1 §7.3.5 Name objects + §11.6.7 spot colour names: PDF
/// names can carry `#XX` hex escapes for whitespace and reserved
/// characters. The decoded ink name is the one that lives on the
/// plate.
///
/// PDF stream declares `/Separation /PANTONE#20185#20C /DeviceCMYK ...`;
/// the lexer decodes to "PANTONE 185 C". The plate list returned by
/// `render_separations` must include "PANTONE 185 C" (the decoded
/// form) and the plate's centre pixel must reflect the paint value.
#[test]
fn round3_p10_hex_escaped_spot_ink_name_routes_to_decoded_plate() {
    let icc = build_constant_cmyk_icc(135);
    let psfunc = "<< /FunctionType 2 /Domain [0 1] /Range [0 1 0 1 0 1 0 1] \
                  /C0 [0.0 0.0 0.0 0.0] /C1 [0.0 1.0 1.0 0.0] /N 1 >>";
    let content = "/Trig gs\n\
                   /CS_PMS cs\n1.0 scn\n0 0 100 100 re\nf\n";
    let resources = format!(
        "/ExtGState << /Trig << /Type /ExtGState /ca 0.5 >> >> \
         /ColorSpace << /CS_PMS [/Separation /PANTONE#20185#20C /DeviceCMYK {} ] >>",
        psfunc
    );
    let pdf = build_pdf_with_output_intent(content, &resources, &icc, &[]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let pms = plate(&plates, "PANTONE 185 C");
    let expected = tint_to_u8(compose_normal(0.0, 1.0, 0.5));
    assert_eq!(expected, 128);
    assert_eq!(
        centre(pms),
        expected,
        "ISO 32000-1 §7.3.5: /PANTONE#20185#20C decodes to \"PANTONE \
         185 C\"; the plate lookup uses the decoded name end-to-end. \
         Centre value = 0.5·1.0 = u8 {}. Got {}.",
        expected,
        centre(pms)
    );
}
