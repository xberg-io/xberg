//! `/CalRGB` components are CIE values, not sRGB ones.
//!
//! ISO 32000-1:2008 §8.6.5.3 (`docs/spec/pdf.md`:10313-10320):
//!
//! > The transformation defined by the **Gamma** and **Matrix** entries in the
//! > **CalRGB** colour space dictionary shall be
//! > `X = X_A × A^G_R + X_B × B^G_G + X_C × C^G_B`
//!
//! and likewise for Y and Z, after which XYZ is projected to the device.
//!
//! `CalRGB` used to share the `DeviceRGB` arm, which passes the components
//! straight through as sRGB. With the common `/Gamma [1 1 1]` the components
//! are **linear**, and linear values read as sRGB render too dark: on the
//! corpus file for this case we sat 31.5 grey levels below two engines that
//! agreed with each other, while coverage matched to 0.0003 — the signature
//! of a colour-conversion error rather than a geometric one.

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

/// A page filled with one `/CalRGB` colour, with the given `/Gamma`.
fn calrgb_page(gamma: &str, comps: &str) -> Vec<u8> {
    let content = format!("/Cs1 cs {comps} sc 0 0 100 100 re f\n").into_bytes();

    let mut pdf = Vec::new();
    let mut off = [0usize; 7];
    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }
    push!("%PDF-1.7\n");
    off[1] = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    off[2] = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    off[3] = pdf.len();
    push!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
         /Contents 4 0 R /Resources << /ColorSpace << /Cs1 5 0 R >> >> >>\nendobj\n"
    );
    off[4] = pdf.len();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()));
    pdf.extend_from_slice(&content);
    push!("endstream\nendobj\n");
    off[5] = pdf.len();
    push!(format!(
        "5 0 obj\n[ /CalRGB << /WhitePoint [0.9505 1.0 1.089] /Gamma {gamma} \
         /Matrix [1 0 0 0 1 0 0 0 1] >> ]\nendobj\n"
    ));

    let xref = pdf.len();
    push!("xref\n0 6\n0000000000 65535 f \r\n");
    for offset in &off[1..=5] {
        push!(format!("{:010} 00000 n \r\n", offset));
    }
    push!(format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    ));
    pdf
}

fn mean_channel(gamma: &str, comps: &str) -> f64 {
    let doc = PdfDocument::from_bytes(calrgb_page(gamma, comps)).expect("synthetic PDF parses");
    let img = render_page(&doc, 0, &RenderOptions::default()).expect("page renders");
    let px = image::load_from_memory(&img.data).expect("PNG decodes").to_rgba8();
    let n = px.pixels().len() as f64;
    px.pixels().map(|p| f64::from(p[0])).sum::<f64>() / n
}

#[test]
fn linear_components_are_encoded_to_srgb_not_passed_through() {
    // Gamma [1 1 1] and an identity matrix mean the components are linear
    // CIE values. Linear 0.5 is roughly sRGB 0.735 — about 188 — not 128.
    let v = mean_channel("[1 1 1]", "0.5 0.5 0.5");
    assert!(
        v > 165.0,
        "linear 0.5 rendered as {v:.1}; passing the component straight through \
         as sRGB would give ~128, which is the defect this pins"
    );
    assert!(v < 215.0, "linear 0.5 rendered implausibly light: {v:.1}");
}

#[test]
fn test_gamma_entry_is_honoured() {
    // A gamma of 2.2 darkens the component before the matrix (0.5^2.2 ≈ 0.22),
    // so the result must be materially darker than the gamma-1 case.
    let g1 = mean_channel("[1 1 1]", "0.5 0.5 0.5");
    let g22 = mean_channel("[2.2 2.2 2.2]", "0.5 0.5 0.5");
    assert!(
        g22 < g1 - 30.0,
        "/Gamma appears to be ignored: gamma 1 gave {g1:.1}, gamma 2.2 gave {g22:.1}"
    );
}

#[test]
fn black_and_white_are_preserved() {
    assert!(mean_channel("[1 1 1]", "0 0 0") < 12.0, "black should stay black");
    assert!(mean_channel("[1 1 1]", "1 1 1") > 243.0, "white should stay white");
}
