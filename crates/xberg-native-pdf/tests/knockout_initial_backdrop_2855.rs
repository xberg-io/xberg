use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{PageRenderer, RenderOptions, render_separation};

fn pdf(page_group: &str, content: &str, form: Option<(&str, &str)>, constant_alternate: bool) -> PdfDocument {
    pdf_with_mode(page_group, content, form, constant_alternate, "Normal", 1.0)
}

fn pdf_with_mode(
    page_group: &str,
    content: &str,
    form: Option<(&str, &str)>,
    constant_alternate: bool,
    mode: &str,
    alpha: f32,
) -> PdfDocument {
    let alternate = if constant_alternate { "0.5" } else { "1" };
    let end_alternate = if constant_alternate { "0.5" } else { "0" };
    let resources = format!(
        "/Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> /ColorSpace << /Other [/Separation /OtherInk /DeviceGray << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>] /Ink [/Separation /TestInk /DeviceGray << /FunctionType 2 /Domain [0 1] \
         /C0 [{alternate}] /C1 [{end_alternate}] /N 1 >>] >> \
         /ExtGState << /Opaque << /ca 1 /SMask /None >> /Trans << /ca 0.4 >> /Multiply << /BM /Multiply >> /PartialMask << /SMask << /S /Luminosity /G 7 0 R /TR << /FunctionType 2 /Domain [0 1] /C0 [0.4] /C1 [0.4] /N 1 >> >> >> /Masked << /SMask << /S /Luminosity /G 7 0 R >> >> /Zero << /ca 0 >> /Mode << /BM /{mode} /ca {alpha} >> >> \
         /Pattern << /P 6 0 R >> /XObject << /Fm 5 0 R /Img 8 0 R /SpotImg 10 0 R >>"
    );
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 32 16] /Resources << {resources} >> \
                 /Contents 4 0 R {page_group} >>"
        ),
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
    ];
    let (group, body) = form.unwrap_or(("", ""));
    objects.push(format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 32 16] /Resources << {resources} >> \
         {group} /Length {} >>\nstream\n{body}\nendstream",
        body.len()
    ));
    let pattern = "1 g 0 0 8 16 re f";
    objects.push(format!(
        "<< /Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
        /BBox [0 0 16 16] /XStep 16 /YStep 16 /Resources << >> /Length {} >>\nstream\n{pattern}\nendstream",
        pattern.len()
    ));
    let mask = "1 g 0 0 32 16 re f";
    objects.push(format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 32 16] /Resources << >> \
        /Group << /S /Transparency >> /Length {} >>\nstream\n{mask}\nendstream",
        mask.len()
    ));
    for (dictionary, samples) in [
        ("/ColorSpace /DeviceRGB /SMask 9 0 R", "ffffff>"),
        ("/ColorSpace /DeviceGray", "66>"),
    ] {
        objects.push(format!(
            "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /BitsPerComponent 8 \
            {dictionary} /Filter /ASCIIHexDecode /Length {} >>\nstream\n{samples}\nendstream",
            samples.len()
        ));
    }
    let spot_samples = "99>";
    objects.push(format!(
        "<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /BitsPerComponent 8 \
        /ColorSpace /Ink /SMask 9 0 R /Filter /ASCIIHexDecode /Length {} >>\nstream\n{spot_samples}\nendstream",
        spot_samples.len()
    ));
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0];
    for (i, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", i + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in &offsets[1..] {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    PdfDocument::from_bytes(bytes).expect("parse synthetic PDF")
}

fn sample(doc: &PdfDocument, x: usize) -> (u8, u8) {
    let image = PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
        .render_page(doc, 0)
        .expect("page");
    assert_eq!((image.width, image.height), (32, 16));
    let plate = render_separation(doc, 0, "TestInk", 72).expect("spot plate");
    assert_eq!((plate.width, plate.height), (32, 16));
    (image.data[(8 * 32 + x) * 4], plate.data[8 * 32 + x])
}

fn near(actual: u8, expected: f32) {
    let expected = (expected * 255.0).round() as u8;
    assert!(actual.abs_diff(expected) <= 1, "actual {actual}, expected {expected}");
}

#[test]
fn page_knockout_translucent_paint_uses_initial_paper() {
    let doc = pdf(
        "/Group << /S /Transparency /K true >>",
        "/Ink cs 0.8 scn 0 0 24 16 re f /Trans gs 0.6 scn 8 0 24 16 re f",
        None,
        false,
    );
    let (rgb, tint) = sample(&doc, 16);
    near(rgb, 1.0 - 0.4 * 0.6);
    near(tint, 0.4 * 0.6);
    let (rgb, tint) = sample(&doc, 4);
    near(rgb, 0.2);
    near(tint, 0.8);
}

#[test]
fn non_knockout_page_control_keeps_previous_paint() {
    let doc = pdf(
        "/Group << /S /Transparency /K false >>",
        "/Ink cs 0.8 scn 0 0 24 16 re f /Trans gs 0.6 scn 8 0 24 16 re f",
        None,
        false,
    );
    let (rgb, tint) = sample(&doc, 16);
    near(rgb, 1.0 - (0.8 * 0.6 + 0.6 * 0.4));
    near(tint, 0.8 * 0.6 + 0.6 * 0.4);
}

#[test]
fn form_knockout_restores_backdrop_even_when_the_paint_matches_it() {
    for isolated in [false, true] {
        for alpha in ["/Opaque", "/Trans"] {
            let group = format!("/Group << /S /Transparency /K true /I {isolated} >>");
            let body = format!("/Ink cs 0.8 scn 0 0 24 16 re f {alpha} gs 0.2 scn 8 0 24 16 re f");
            let doc = pdf(
                "",
                "/Ink cs 0.2 scn 0 0 32 16 re f /Fm Do",
                Some((&group, &body)),
                false,
            );
            let (rgb, tint) = sample(&doc, 16);
            near(rgb, 0.8);
            near(tint, 0.2);
        }
    }
}

#[test]
fn knockout_shape_does_not_depend_on_the_spot_alternate_rgb() {
    for page in [false, true] {
        let group = "/Group << /S /Transparency /K true >>";
        let body = "/Ink cs 0.8 scn 0 0 24 16 re f /Opaque gs 0 scn 8 0 24 16 re f";
        let doc = if page {
            pdf(group, body, None, true)
        } else {
            pdf("", "/Fm Do", Some((group, body)), true)
        };
        let (_, tint) = sample(&doc, 16);
        assert_eq!(tint, 0, "page={page}: explicit zero tint must replace prior ink");
        assert_eq!(sample(&doc, 4).1, 204, "first shape outside overlap must survive");
    }
}

#[test]
fn previous_fill_path_is_consumed_before_the_next_knockout_element() {
    let doc = pdf(
        "",
        "/Fm Do",
        Some((
            "/Group << /S /Transparency /K true >>",
            "/Ink cs 0.8 scn 0 0 8 16 re f 0.2 scn 24 0 8 16 re f",
        )),
        false,
    );
    near(sample(&doc, 4).1, 0.8);
    near(sample(&doc, 28).1, 0.2);
    assert_eq!(sample(&doc, 16).1, 0);
}

#[test]
fn fractional_shape_retains_only_the_uncovered_part_of_prior_paint() {
    let doc = pdf(
        "/Group << /S /Transparency /K true >>",
        "/Ink cs 0.8 scn 0 0 32 16 re f /Trans gs 0.6 scn 8.5 0 16 16 re f",
        None,
        false,
    );
    let (rgb, tint) = sample(&doc, 8);
    near(rgb, 1.0 - (0.5 * 0.8 + 0.5 * 0.4 * 0.6));
    near(tint, 0.5 * 0.8 + 0.5 * 0.4 * 0.6);
}

#[test]
fn zero_opacity_still_knocks_out_the_previous_element_shape() {
    let doc = pdf(
        "/Group << /S /Transparency /K true >>",
        "/Ink cs 0.8 scn 0 0 32 16 re f /Zero gs 0 scn 8 0 16 16 re f",
        None,
        true,
    );
    let (_, tint) = sample(&doc, 16);
    assert_eq!(tint, 0);
}

#[test]
fn knockout_blends_match_a_single_element_against_the_same_initial_backdrop() {
    for isolated in [false, true] {
        for mode in [
            "Normal",
            "Multiply",
            "Darken",
            "Screen",
            "Lighten",
            "SoftLight",
            "ColorDodge",
            "ColorBurn",
        ] {
            for alpha in [1.0, 0.4] {
                let group = format!("/Group << /S /Transparency /K true /I {isolated} >>");
                let last = "/Mode gs /Ink cs 0.6 scn 8 0 16 16 re f";
                let both = format!("/Ink cs 0.8 scn 0 0 24 16 re f {last}");
                let make = |body: &str| {
                    pdf_with_mode(
                        "",
                        "/Ink cs 0.2 scn 0 0 32 16 re f /Fm Do",
                        Some((&group, body)),
                        false,
                        mode,
                        alpha,
                    )
                };
                assert_eq!(
                    sample(&make(&both), 16),
                    sample(&make(last), 16),
                    "isolated={isolated}, mode={mode}, alpha={alpha}"
                );
            }
        }
    }
}

#[test]
fn inline_image_is_a_knockout_element() {
    let doc = pdf(
        "/Group << /S /Transparency /K true >>",
        "/Ink cs 0.8 scn 0 0 32 16 re f q 16 0 0 16 8 0 cm BI /W 1 /H 1 /CS /G /BPC 8 ID \x7f\nEI Q",
        None,
        false,
    );
    assert_eq!(sample(&doc, 16).0, 127);
}

#[test]
fn invisible_text_has_no_knockout_shape() {
    let group = "/Group << /S /Transparency /K true >>";
    let base = "/Ink cs 0.8 scn 0 0 32 16 re f";
    let render = |body: &str| {
        PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
            .render_page(&pdf(group, body, None, false), 0)
            .expect("page")
            .data
    };
    let reference = render(base);
    let visible = render(&format!("{base} BT /F1 12 Tf 8 2 Td 1 g (M) Tj ET"));
    assert_ne!(visible, reference, "visible text control must paint real glyphs");
    for mode in [3, 7] {
        let body = format!("{base} BT /F1 12 Tf 8 2 Td 1 g {mode} Tr (M) Tj ET");
        assert_eq!(render(&body), reference, "render mode {mode}");
    }
}

#[test]
fn fractional_clip_and_path_coverages_multiply() {
    let doc = pdf(
        "/Group << /S /Transparency /K true >>",
        "/Ink cs 0.8 scn 0 0 32 16 re f q 8.5 0 16 16 re W n 0 scn 8.5 0 16 16 re f Q",
        None,
        false,
    );
    near(sample(&doc, 8).0, 1.0 - 0.75 * 0.8);
}

#[test]
fn tiling_pattern_shape_is_mapped_to_every_tile_and_keeps_holes() {
    let doc = pdf(
        "/Group << /S /Transparency /K true >>",
        "/Ink cs 0.8 scn 0 0 32 16 re f /Pattern cs /P scn 0 0 32 16 re f",
        None,
        false,
    );
    for x in [4, 20] {
        assert_eq!(sample(&doc, x).0, 255, "painted tile x={x}");
    }
    for x in [12, 28] {
        assert_eq!(sample(&doc, x).0, 51, "hole x={x}");
    }
}

#[test]
fn mask_geometry_does_not_expand_the_host_element_shape() {
    let doc = pdf(
        "/Group << /S /Transparency /K true >>",
        "/Ink cs 0.8 scn 0 0 32 16 re f /Masked gs 0 scn 8 0 16 16 re f",
        None,
        false,
    );
    assert_eq!(sample(&doc, 4), (51, 204));
    assert_eq!(sample(&doc, 16), (255, 0));
}

#[test]
fn isolated_group_preserves_parent_inks_it_does_not_name() {
    for knockout in [false, true] {
        let group = format!("/Group << /S /Transparency /I true /K {knockout} >>");
        let doc = pdf(
            "",
            "/Ink cs 0.8 scn 0 0 32 16 re f /Fm Do",
            Some((&group, "1 g 0 0 32 16 re f")),
            false,
        );
        assert_eq!(sample(&doc, 16), (255, 204), "knockout={knockout}");
    }
}

#[test]
fn isolated_spot_opacity_is_independent_of_other_inks_and_rgb_alpha() {
    for knockout in [false, true] {
        for state in ["/Trans", "/PartialMask"] {
            let group = format!("/Group << /S /Transparency /I true /K {knockout} >>");
            let body = format!(
                "{state} gs /Ink cs 0.6 scn 0 0 32 16 re f \
                /Opaque gs /Other cs 1 scn 0 0 32 16 re f"
            );
            let doc = pdf(
                "",
                "/Ink cs 0.8 scn 0 0 32 16 re f /Fm Do",
                Some((&group, &body)),
                false,
            );
            near(sample(&doc, 16).1, 0.4 * 0.6 + 0.6 * 0.8);
        }
    }
}

#[test]
fn image_soft_mask_is_opacity_rather_than_geometric_shape() {
    let body = "/Ink cs 0.8 scn 0 0 32 16 re f q 16 0 0 16 8 0 cm /Img Do Q";
    let doc = pdf("/Group << /S /Transparency /K true >>", body, None, false);
    assert_eq!(sample(&doc, 16).0, 255);
    assert_eq!(sample(&doc, 4).0, 51);
    let control = pdf("/Group << /S /Transparency /K false >>", body, None, false);
    near(sample(&control, 16).0, 0.4 + 0.6 * 0.2);
}

#[test]
fn masked_fill_and_stroke_union_their_opacities_before_group_composition() {
    let group = "/Group << /S /Transparency /I true /K false >>";
    let body = "/PartialMask gs /Ink cs /Ink CS 0.6 scn 0.6 SCN 12 w 8 4 16 8 re B";
    let doc = pdf("", "/Ink cs 0.8 scn 0 0 32 16 re f /Fm Do", Some((group, body)), false);
    near(sample(&doc, 16).1, 0.6 * 0.64 + 0.8 * 0.36);
}

#[test]
fn page_knockout_blend_uses_the_initial_unpainted_spot_backdrop() {
    for mode in ["Multiply", "Darken"] {
        for alpha in [1.0, 0.4] {
            let doc = pdf_with_mode(
                "/Group << /S /Transparency /K true >>",
                "/Ink cs 0.8 scn 0 0 24 16 re f /Mode gs 0.6 scn 8 0 24 16 re f",
                None,
                false,
                mode,
                alpha,
            );
            let (rgb, tint) = sample(&doc, 16);
            near(rgb, 1.0 - alpha * 0.6);
            near(tint, alpha * 0.6);
        }
    }
}

#[test]
fn form_knockout_blend_that_matches_initial_backdrop_still_replaces_prior_paint() {
    for isolated in [false, true] {
        for (mode, source) in [("Darken", 0.12), ("SoftLight", 0.5)] {
            for alpha in [1.0, 0.4] {
                let group = format!("/Group << /S /Transparency /K true /I {isolated} >>");
                let body = format!("/Ink cs 0.8 scn 0 0 24 16 re f /Mode gs {source} scn 8 0 24 16 re f");
                let doc = pdf_with_mode(
                    "",
                    "/Ink cs 0.2 scn 0 0 32 16 re f /Fm Do",
                    Some((&group, &body)),
                    false,
                    mode,
                    alpha,
                );
                let expected = if isolated {
                    alpha * source + (1.0 - alpha) * 0.2
                } else {
                    0.2
                };
                let (rgb, tint) = sample(&doc, 16);
                near(rgb, 1.0 - expected);
                near(tint, expected);
            }
        }
    }
}

#[test]
fn stroke_only_text_uses_the_stroke_silhouette_for_knockout() {
    let parent = "/Ink cs 0.8 scn 0 0 32 16 re f";
    let body = format!("{parent} BT /F1 12 Tf 8 2 Td 1 g 1 G 2 w 1 Tr (M) Tj ET");
    let render = |knockout: bool, content: &str| {
        let group = format!("/Group << /S /Transparency /K {knockout} >>");
        PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
            .render_page(&pdf(&group, content, None, false), 0)
            .expect("page")
            .data
    };
    let reference = render(false, &body);
    assert_ne!(
        reference,
        render(false, parent),
        "the stroke must paint real glyph outlines"
    );
    let actual = render(true, &body);
    assert_eq!(actual.len(), reference.len());
    assert!(
        actual
            .iter()
            .zip(reference)
            .all(|(actual, expected)| actual.abs_diff(expected) <= 1),
        "stroke coverage must match within one antialiasing quantization unit"
    );
}

#[test]
fn stroke_only_text_does_not_erase_an_unpainted_fill_ink() {
    for knockout in [false, true] {
        for mode in [1, 5] {
            let first = "/Ink cs 0.6 scn 0 0 32 16 re f";
            let body = format!("{first} 0.4 scn BT /F1 12 Tf 8 2 Td 1 G 2 w {mode} Tr (M) Tj ET");
            let group = format!("/Group << /S /Transparency /I true /K {knockout} >>");
            let parent = "/Ink cs 0.8 scn 0 0 32 16 re f /Fm Do";
            let doc = pdf("", parent, Some((&group, &body)), false);
            let reference = pdf("", parent, Some((&group, first)), false);
            let render = |doc: &PdfDocument| {
                PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
                    .render_page(doc, 0)
                    .expect("page")
                    .data
            };
            assert_ne!(render(&doc), render(&reference), "real glyphs must paint");
            let plate = render_separation(&doc, 0, "TestInk", 72).expect("spot plate");
            assert_eq!(plate.data, vec![153; 32 * 16], "mode {mode}, knockout={knockout}");
        }
    }
}

#[test]
fn spot_image_opacity_is_separate_from_knockout_shape() {
    let body = "/Ink cs 0.8 scn 0 0 32 16 re f 0.6 scn q 16 0 0 16 8 0 cm /SpotImg Do Q";
    let control = pdf("/Group << /S /Transparency /K false >>", body, None, false);
    near(sample(&control, 16).1, 0.4 * 0.6 + 0.6 * 0.8);
    let doc = pdf("/Group << /S /Transparency /K true >>", body, None, false);
    near(sample(&doc, 16).1, 0.4 * 0.6);
}
