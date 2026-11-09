//! The sidecar transparency gate reads ExtGState through the shared parser. These tests
//! pin the rendered output of the pages whose gate decision that move changed.

use super::*;
use crate::rendering::page_renderer::{PageRenderer, RenderOptions};

const DPI: u32 = 72;

/// Two overlapping fills in the spot ink `InkA`; the second runs under `/T gs`.
const CONTENT: &str = "/CS0 cs 0.9 scn 10 10 60 60 re f /T gs /CS0 cs 0.5 scn 40 40 50 50 re f";

fn build_page(state_body: &str, extras: &[&str]) -> PdfDocument {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offs: Vec<usize> = Vec::new();
    let mut push = |buf: &mut Vec<u8>, body: &str| {
        offs.push(buf.len());
        buf.extend_from_slice(body.as_bytes());
    };
    push(&mut buf, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    push(&mut buf, "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    push(
        &mut buf,
        &format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << /ColorSpace << /CS0 [/Separation /InkA /DeviceCMYK \
             << /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 1 1 0] /N 1 >>] >> \
             /ExtGState << /T << /Type /ExtGState {state_body} >> >> >> >>\nendobj\n"
        ),
    );
    push(
        &mut buf,
        &format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{CONTENT}\nendstream\nendobj\n",
            CONTENT.len()
        ),
    );
    for extra in extras {
        push(&mut buf, extra);
    }
    let xref_off = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offs.len() + 1).as_bytes());
    for off in &offs {
        buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n",
            offs.len() + 1
        )
        .as_bytes(),
    );
    PdfDocument::from_bytes(buf).expect("synthetic PDF parses")
}

fn plate(state_body: &str, extras: &[&str]) -> Vec<u8> {
    let inks = vec!["InkA".to_string()];
    let doc = build_page(state_body, extras);
    let plates = render_plates_for_inks(&doc, 0, DPI, &inks, &inks).expect("render plates");
    assert_eq!(plates.len(), 1);
    plates.into_iter().next().expect("one plate").data
}

fn assert_page_independent_of_sidecar(state_body: &str) {
    let doc = build_page(state_body, &[]);
    let render = |force: bool| {
        let mut renderer = PageRenderer::new(RenderOptions::with_dpi(DPI).as_raw());
        renderer.force_cmyk_sidecar = force;
        renderer.render_page(&doc, 0).expect("render page").data
    };
    let without_sidecar = render(false);
    assert!(without_sidecar.iter().any(|&b| b != 255), "{state_body}: page is blank");
    assert!(
        without_sidecar == render(true),
        "{state_body}: page pixels depend on the sidecar"
    );
}

#[test]
fn indirect_blend_array_element_blends_like_a_direct_name() {
    let direct = plate("/BM /Multiply", &[]);
    let indirect = plate("/BM [5 0 R]", &["5 0 obj\n/Multiply\nendobj\n"]);
    let unblended = plate("/BM /Normal", &[]);
    assert!(direct != unblended, "Multiply must change the plate");
    assert!(indirect == direct, "indirect /BM array element must blend as /Multiply");
}

#[test]
fn malformed_soft_mask_and_unrecognised_blend_name_render_the_same_page_with_or_without_sidecar() {
    for state in [
        "/SMask << /S /Alpha >>",
        "/SMask << /S /Alpha /G 7 >>",
        "/SMask 3",
        "/SMask 99 0 R",
        "/BM /NoSuchMode",
    ] {
        assert_page_independent_of_sidecar(state);
    }
}
