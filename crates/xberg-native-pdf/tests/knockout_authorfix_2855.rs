use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::object::{Object, ObjectRef};
use xberg_native_pdf::rendering::{PageRenderer, RenderOptions, render_separation};

mod common;

const RECT: &str = "0 0 32 16 re f";

fn stream(dictionary: &str, body: &str) -> String {
    format!("<< {dictionary} /Length {} >>\nstream\n{body}\nendstream", body.len())
}

fn document(page_group: &str, page: &str, group: &str, body: &str) -> PdfDocument {
    document_with_mask_generation(page_group, page, group, body, 0)
}

fn document_with_mask_generation(
    page_group: &str,
    page: &str,
    group: &str,
    body: &str,
    generation: u16,
) -> PdfDocument {
    document_with_profile(page_group, page, group, body, generation, None)
}

fn document_with_profile(
    page_group: &str,
    page: &str,
    group: &str,
    body: &str,
    generation: u16,
    profile: Option<&[u8]>,
) -> PdfDocument {
    // Objects 1..8 are the fixed catalog/pages/page/contents/form/group/
    // state/pattern set below; a profile takes object 9, then the fixture
    // font's FontFile2 stream and its FontDescriptor (which the font loader
    // only reads through an indirect reference) follow.
    let font_object_id = 9 + usize::from(profile.is_some());
    let descriptor_object_id = font_object_id + 1;
    let resources = format!(
        "/ColorSpace << /Ink [/Separation /TestInk /DeviceGray \
        << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>] \
        /Flat [/Separation /TestInk /DeviceGray << /FunctionType 2 /Domain [0 1] /C0 [0.5] /C1 [0.5] /N 1 >>] \
        /Other [/Separation /OtherInk /DeviceGray << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >>] >> \
        /ExtGState << /Zero << /ca 0 >> /Full << /ca 1 >> /Half << /ca 0.5 /CA 0.5 >> /Quarter << /ca 0.25 >> /Multiply << /BM /Multiply >> \
        /StrokeHalf << /CA 0.5 >> /StrokeOverprint << /OP true /op false /OPM 1 >> \
        /Darken << /BM /Darken >> /Lighten << /BM /Lighten >> /SoftLight << /BM /SoftLight >> \
        /Difference << /BM /Difference >> /Exclusion << /BM /Exclusion >> \
        /Hue << /BM /Hue >> /Saturation << /BM /Saturation >> /Color << /BM /Color >> /Luminosity << /BM /Luminosity >> \
        /Mask << /SMask << /S /Luminosity /G 6 0 R >> >> >> \
        /Font << /F1 << /Type /Font /Subtype /TrueType /BaseFont /FixtureBox \
        /FirstChar 77 /LastChar 77 /Widths [500] \
        /FontDescriptor {descriptor_object_id} 0 R >> >> \
        /XObject << /Fm 5 0 R >> /Pattern << /P 8 0 R >>"
    );
    let resources = resources.replace("/G 6 0 R", &format!("/G 6 {generation} R"));
    let catalog = if profile.is_some() {
        "/OutputIntents [<< /S /GTS_PDFX /DestOutputProfile 9 0 R >>]"
    } else {
        ""
    };
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R {catalog} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 32 16] \
             /Resources << {resources} >> /Contents 4 0 R {page_group} >>"
        ),
        stream("", page),
        stream(
            &format!(
                "/Type /XObject /Subtype /Form /BBox [0 0 32 16] \
                 /Resources << {resources} >> {group}"
            ),
            body,
        ),
        stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 32 16] /Resources << >> \
             /Group << /S /Transparency /CS /DeviceGray /I true >>",
            "0.5 g 0 0 32 16 re f",
        ),
        "<< /S /Transparency /K false >>".to_string(),
        stream(
            "/Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 \
             /BBox [0 0 2 2] /XStep 0.25 /YStep 0.25 /Resources << >>",
            "1 g 0 0 2 2 re f",
        ),
    ];
    let mut objects: Vec<_> = objects.into_iter().map(String::into_bytes).collect();
    if let Some(profile) = profile {
        let mut stream = format!("<< /N 4 /Length {} >>\nstream\n", profile.len()).into_bytes();
        stream.extend_from_slice(profile);
        stream.extend_from_slice(b"\nendstream");
        objects.push(stream);
    }
    debug_assert_eq!(objects.len(), font_object_id - 1);
    let font = fixture_font();
    let mut stream = format!("<< /Length {} /Length1 {} >>\nstream\n", font.len(), font.len()).into_bytes();
    stream.extend_from_slice(&font);
    stream.extend_from_slice(b"\nendstream");
    objects.push(stream);
    objects.push(
        format!(
            "<< /Type /FontDescriptor /FontName /FixtureBox /Flags 4 \
             /FontBBox [250 0 750 800] /ItalicAngle 0 /Ascent 800 /Descent -200 \
             /CapHeight 800 /StemV 80 /FontFile2 {font_object_id} 0 R >>"
        )
        .into_bytes(),
    );
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        bytes.extend_from_slice(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    PdfDocument::from_bytes(bytes).expect("parse finite PDF with byte-accurate xref and stream lengths")
}

#[derive(Clone, Copy, Debug)]
enum Ink {
    Spot,
    Black,
    FlatSpot,
}

impl Ink {
    fn paint(self, tint: &str) -> String {
        match self {
            Self::Spot => format!("/Ink cs {tint} scn {RECT}"),
            Self::FlatSpot => format!("/Flat cs {tint} scn {RECT}"),
            Self::Black => format!("0 0 0 {tint} k {RECT}"),
        }
    }

    fn plate(self) -> &'static str {
        match self {
            Self::Spot | Self::FlatSpot => "TestInk",
            Self::Black => "Black",
        }
    }
}

fn sample(doc: &PdfDocument, ink: Ink, x: usize) -> ([u8; 4], u8) {
    let image = PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
        .render_page(doc, 0)
        .expect("recoverable page renders");
    let plate = render_separation(doc, 0, ink.plate(), 72).expect("render separation");
    assert_eq!((image.width, image.height), (32, 16));
    assert_eq!((plate.width, plate.height), (32, 16));
    let pixel = 8 * 32 + x;
    (
        image.data[pixel * 4..pixel * 4 + 4].try_into().unwrap(),
        plate.data[pixel],
    )
}

fn expect_sample(doc: &PdfDocument, ink: Ink, x: usize, gray: u8, tint: u8) {
    expect_rgb_sample(doc, ink, x, [gray; 3], tint);
}

fn expect_rgb_sample(doc: &PdfDocument, ink: Ink, x: usize, rgb: [u8; 3], tint: u8) {
    let (rgba, plate) = sample(doc, ink, x);
    assert_eq!(rgba[3], 255, "opaque paper: {ink:?}");
    for (channel, expected) in rgba[..3].iter().zip(rgb) {
        assert!(
            channel.abs_diff(expected) <= 1,
            "{ink:?}: RGBA {rgba:?}, expected RGB {rgb:?}"
        );
    }
    assert!(plate.abs_diff(tint) <= 1, "{ink:?}: plate {plate}, expected {tint}");
}

// Root painting bypasses Form compositing while retaining the renderer's color conversion. ~keep
fn root_rgb(ink: Ink, tint: &str) -> [u8; 3] {
    let doc = document("", &ink.paint(tint), "", "");
    let (rgba, _) = sample(&doc, ink, 16);
    assert_eq!(rgba[3], 255, "opaque root reference: {ink:?}");
    let rgb = [rgba[0], rgba[1], rgba[2]];
    if matches!(ink, Ink::Spot) {
        let expected = ((1.0 - tint.parse::<f64>().unwrap()) * 255.0).round() as u8;
        assert_eq!(rgb, [expected; 3], "linear Spot root control");
    }
    rgb
}

fn mix_rgb(parent: [u8; 3], source: [u8; 3], alpha: f64) -> [u8; 3] {
    std::array::from_fn(|i| ((1.0 - alpha) * f64::from(parent[i]) + alpha * f64::from(source[i])).round() as u8)
}

// ISO 32000-1 §11.3.5 separable blend functions, evaluated in RGB per channel. ~keep
fn blend_rgb(mode: &str, parent: [u8; 3], source: [u8; 3]) -> [u8; 3] {
    std::array::from_fn(|i| {
        let b = f64::from(parent[i]) / 255.0;
        let s = f64::from(source[i]) / 255.0;
        let result = match mode {
            "Multiply" => b * s,
            "Darken" => b.min(s),
            "SoftLight" if s <= 0.5 => b - (1.0 - 2.0 * s) * b * (1.0 - b),
            "SoftLight" => {
                let d = if b <= 0.25 {
                    ((16.0 * b - 12.0) * b + 4.0) * b
                } else {
                    b.sqrt()
                };
                b + (2.0 * s - 1.0) * (d - b)
            }
            _ => panic!("unsupported reference blend {mode}"),
        };
        assert!(result.is_finite());
        (result * 255.0).round() as u8
    })
}

fn form(ink: Ink, isolated: bool, knockout: bool, outer: &str, body: &str) -> PdfDocument {
    document(
        "",
        &format!("{} q {outer} /Fm Do Q", ink.paint("0.2")),
        &format!("/Group << /S /Transparency /I {isolated} /K {knockout} >>"),
        body,
    )
}

#[test]
fn form_without_outer_modulator_controls() {
    for ink in [Ink::Spot, Ink::Black] {
        for isolated in [false, true] {
            for knockout in [false, true] {
                let doc = form(ink, isolated, knockout, "", &ink.paint("0.6"));
                expect_rgb_sample(&doc, ink, 16, root_rgb(ink, "0.6"), 153);
            }
        }
    }
}

#[test]
fn outer_half_opacity_modulates_form_pixels_and_plates() {
    // ISO 32000-1 §11.6.6: the group's ca is applied once at Do, not per child. ~keep
    for ink in [Ink::Spot, Ink::Black] {
        for isolated in [false, true] {
            for knockout in [false, true] {
                let body = format!("{} {}", ink.paint("0.8"), ink.paint("0.6"));
                let doc = form(ink, isolated, knockout, "/Half gs", &body);
                // 0.5 * 0.6 + 0.5 * 0.2 = 0.4 ink. ~keep
                expect_rgb_sample(
                    &doc,
                    ink,
                    16,
                    mix_rgb(root_rgb(ink, "0.2"), root_rgb(ink, "0.6"), 0.5),
                    102,
                );
            }
        }
    }
}

fn outer_blend(mode: &str, source: &str, gray: u8, tint: u8) {
    for ink in [Ink::Spot, Ink::Black] {
        for knockout in [false, true] {
            let doc = form(ink, true, knockout, &format!("/{mode} gs"), &ink.paint(source));
            let expected = blend_rgb(mode, root_rgb(ink, "0.2"), root_rgb(ink, source));
            if matches!(ink, Ink::Spot) {
                assert_eq!(expected, [gray; 3], "independent Spot blend oracle");
            }
            expect_rgb_sample(&doc, ink, 16, expected, tint);
        }
    }
}

#[test]
fn isolated_form_outer_multiply_reaches_pixels_and_plates() {
    // Complement-space Multiply: 0.8 * 0.4 = 0.32 gray, 0.68 ink. ~keep
    outer_blend("Multiply", "0.6", 82, 173);
}

#[test]
fn isolated_form_outer_darken_reaches_pixels_and_plates() {
    outer_blend("Darken", "0.1", 204, 51);
}

#[test]
fn isolated_form_outer_soft_light_reaches_pixels_and_plates() {
    // SoftLight(0.8, 0.4) = 0.8 - (1 - 2 * 0.4) * 0.8 * 0.2 = 0.768. ~keep
    outer_blend("SoftLight", "0.6", 196, 59);
}

#[test]
fn outer_soft_mask_characterization_for_form_pixels_and_plates() {
    for ink in [Ink::Spot, Ink::Black] {
        for isolated in [false, true] {
            for knockout in [false, true] {
                let doc = form(ink, isolated, knockout, "/Mask gs", &ink.paint("0.6"));
                expect_rgb_sample(
                    &doc,
                    ink,
                    16,
                    mix_rgb(root_rgb(ink, "0.2"), root_rgb(ink, "0.6"), 0.5),
                    102,
                );
            }
        }
    }
}

#[test]
fn malformed_indirect_page_group_recovers_pixels_and_plates() {
    for ink in [Ink::Spot, Ink::Black] {
        let body = ink.paint("0.6");
        let control = document("/Group 7 0 R", &body, "", "");
        expect_rgb_sample(&control, ink, 16, root_rgb(ink, "0.6"), 153);
        // Object 7 exists at generation 0; asking for generation 1 must error, not yield Null. ~keep
        let doc = document("/Group 7 1 R", &body, "", "");
        let page = doc.get_page(0).expect("page tree remains valid");
        let group = page.as_dict().unwrap().get("Group").unwrap();
        assert_eq!(group.as_reference(), Some(ObjectRef::new(7, 1)));
        assert!(
            doc.resolve_object(group).is_err(),
            "fixture must exercise a real resolver error"
        );
        assert!(
            matches!(
                doc.resolve_object(&Object::Reference(ObjectRef::new(7, 0))),
                Ok(Object::Dictionary(_))
            ),
            "valid generation control"
        );
        expect_rgb_sample(&doc, ink, 16, root_rgb(ink, "0.6"), 153);
    }
}

#[test]
fn non_knockout_fractional_clip_keeps_minimum_coverage() {
    for knockout in [false, true] {
        let doc = document(
            &format!("/Group << /S /Transparency /K {knockout} >>"),
            "/Ink cs 0.8 scn 0 0 32 16 re f q 8.5 0 16 16 re W n 0 scn 8.5 0 16 16 re f Q",
            "",
            "",
        );
        // TinySkia multiplies path and clip coverage; the sidecar uses legacy minimum coverage. ~keep
        let (gray, tint) = if knockout { (103, 153) } else { (103, 102) };
        expect_sample(&doc, Ink::Spot, 8, gray, tint);
        expect_sample(&doc, Ink::Spot, 4, 51, 204);
        expect_sample(&doc, Ink::Spot, 16, 255, 0);
    }
}

#[test]
fn bounded_dense_pattern_flood_has_opaque_shape_and_respects_path() {
    // Subpixel steps select the flood guard before tile iteration; page and cell stay tiny. ~keep
    for knockout in [false, true] {
        let doc = document(
            &format!("/Group << /S /Transparency /K {knockout} >>"),
            "0.2 g 0 0 32 16 re f /Pattern cs /P scn 8 0 16 16 re f",
            "",
            "",
        );
        let render = PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
            .render_page(&doc, 0)
            .expect("bounded pattern flood");
        for (x, expected) in [(4, 51), (16, 255), (28, 51)] {
            assert_eq!(
                &render.data[(8 * 32 + x) * 4..(8 * 32 + x + 1) * 4],
                &[expected, expected, expected, 255]
            );
        }
    }
}

#[test]
fn outer_opacity_endpoints_preserve_parent_or_source() {
    for ink in [Ink::Spot, Ink::Black] {
        for isolated in [false, true] {
            for knockout in [false, true] {
                for (state, source, tint) in [("Zero", "0.2", 51), ("Full", "0.6", 153)] {
                    let doc = form(ink, isolated, knockout, &format!("/{state} gs"), &ink.paint("0.6"));
                    expect_rgb_sample(&doc, ink, 16, root_rgb(ink, source), tint);
                }
            }
        }
    }
}

#[test]
fn isolated_form_partial_inner_and_outer_opacity_compose_once() {
    for ink in [Ink::Spot, Ink::Black] {
        for knockout in [false, true] {
            let body = format!("/Half gs {}", ink.paint("0.6"));
            let doc = form(ink, true, knockout, "/Half gs", &body);
            // A single half-opaque child and half-opaque group give effective alpha 0.25. ~keep
            let expected = mix_rgb(root_rgb(ink, "0.2"), root_rgb(ink, "0.6"), 0.25);
            expect_rgb_sample(&doc, ink, 16, expected, 77);
        }
    }
}

#[test]
fn malformed_form_soft_mask_restores_pixels_and_every_plate() {
    for ink in [Ink::Spot, Ink::Black] {
        for knockout in [false, true] {
            let doc = document_with_mask_generation(
                "",
                &format!("{} /Mask gs /Fm Do", ink.paint("0.2")),
                &format!("/Group << /S /Transparency /I true /K {knockout} >>"),
                &ink.paint("0.6"),
                1,
            );
            assert!(
                doc.load_object(ObjectRef::new(6, 1)).is_err(),
                "real mask resolver error"
            );
            expect_rgb_sample(&doc, ink, 16, root_rgb(ink, "0.2"), 51);
        }
    }
}

#[test]
fn form_invocation_clip_limits_parent_knockout_shape() {
    for isolated in [false, true] {
        let doc = document(
            "/Group << /S /Transparency /K true >>",
            "/Ink cs 0.8 scn 0 0 32 16 re f q 0 0 16 16 re W n /Fm Do Q",
            &format!("/Group << /S /Transparency /I {isolated} /K true >>"),
            "/Ink cs 0.2 scn 0 0 32 16 re f",
        );
        expect_sample(&doc, Ink::Spot, 8, 204, 51);
        expect_sample(&doc, Ink::Spot, 24, 51, 204);
    }
}

#[test]
fn nonisolated_form_blends_its_source_instead_of_reblending_the_backdrop() {
    for ink in [Ink::Spot, Ink::Black] {
        for knockout in [false, true] {
            for (mode, source, blended_tint) in [
                ("Multiply", "0.6", 0.68),
                ("Darken", "0.1", 0.2),
                ("SoftLight", "0.6", 0.232),
            ] {
                for outer_alpha in [1.0, 0.5] {
                    for inner_alpha in [1.0, 0.5] {
                        let outer = format!("{} /{mode} gs", if outer_alpha == 0.5 { "/Half gs" } else { "" });
                        let body = format!(
                            "{} {}",
                            if inner_alpha == 0.5 { "/Half gs" } else { "" },
                            ink.paint(source)
                        );
                        let doc = form(ink, false, knockout, &outer, &body);
                        let effective = inner_alpha * outer_alpha;
                        let expected = mix_rgb(
                            root_rgb(ink, "0.2"),
                            blend_rgb(mode, root_rgb(ink, "0.2"), root_rgb(ink, source)),
                            effective,
                        );
                        let tint = ((0.2 * (1.0 - effective) + blended_tint * effective) * 255.0).round() as u8;
                        expect_rgb_sample(&doc, ink, 16, expected, tint);
                    }
                }
            }
        }
    }
}

#[test]
fn nonisolated_source_recovery_keeps_equal_rgb_and_unnamed_ink_participation() {
    for knockout in [false, true] {
        let body = format!("/Half gs {}", Ink::FlatSpot.paint("0.6"));
        let doc = form(Ink::FlatSpot, false, knockout, "/Multiply gs", &body);
        let expected = mix_rgb(
            root_rgb(Ink::FlatSpot, "0.2"),
            blend_rgb(
                "Multiply",
                root_rgb(Ink::FlatSpot, "0.2"),
                root_rgb(Ink::FlatSpot, "0.6"),
            ),
            0.5,
        );
        expect_rgb_sample(&doc, Ink::FlatSpot, 16, expected, 112);
        let doc = document(
            "",
            "/Other cs 0.8 scn 0 0 32 16 re f /Flat cs 0.2 scn 0 0 32 16 re f /Lighten gs /Fm Do",
            &format!("/Group << /S /Transparency /I false /K {knockout} >>"),
            "/Half gs /Flat cs 0 scn 0 0 32 16 re f",
        );
        expect_rgb_sample(&doc, Ink::FlatSpot, 16, root_rgb(Ink::FlatSpot, "0.2"), 26);
        let untouched = render_separation(&doc, 0, "OtherInk", 72).expect("untouched plate");
        assert_eq!(untouched.data[8 * 32 + 16], 204);
    }
}

#[test]
fn group_nonseparable_and_nonwhite_modes_keep_process_and_spot_dispatch_distinct() {
    for ink in [Ink::Spot, Ink::Black] {
        for isolated in [false, true] {
            for knockout in [false, true] {
                for (mode, black_tint) in [
                    ("Hue", 51),
                    ("Saturation", 51),
                    ("Color", 51),
                    ("Luminosity", 77),
                    ("Difference", 77),
                    ("Exclusion", 66),
                ] {
                    let body = format!("/Half gs {}", ink.paint("0.6"));
                    let doc = form(ink, isolated, knockout, &format!("/Half gs /{mode} gs"), &body);
                    let direct = document(
                        "",
                        &format!("{} /Quarter gs /{mode} gs {}", ink.paint("0.2"), ink.paint("0.6")),
                        "",
                        "",
                    );
                    let reference = sample(&direct, ink, 16).0;
                    if matches!(ink, Ink::Spot) {
                        let gray = match mode {
                            "Hue" | "Saturation" | "Color" => 204,
                            "Exclusion" => 189,
                            _ => 179,
                        };
                        assert!(reference[0].abs_diff(gray) <= 1, "independent {mode} RGB oracle");
                    }
                    let tint = if matches!(ink, Ink::Spot) { 77 } else { black_tint };
                    expect_rgb_sample(&doc, ink, 16, [reference[0], reference[1], reference[2]], tint);
                }
            }
        }
    }
}

#[test]
fn ordinary_form_caller_mask_still_modulates_pixels_and_plates() {
    for ink in [Ink::Spot, Ink::Black] {
        let doc = document(
            "",
            &format!("{} /Mask gs /Fm Do", ink.paint("0.2")),
            "",
            &ink.paint("0.6"),
        );
        let expected = mix_rgb(root_rgb(ink, "0.2"), root_rgb(ink, "0.6"), 0.5);
        expect_rgb_sample(&doc, ink, 16, expected, 102);
    }
}

#[test]
fn discarded_form_masks_do_not_contribute_parent_knockout_shape() {
    for group in ["/Group << /S /Transparency /I true /K true >>", ""] {
        let doc = document_with_mask_generation(
            "/Group << /S /Transparency /K true >>",
            "/Ink cs 0.8 scn 0 0 32 16 re f /Mask gs /Fm Do",
            group,
            "/Ink cs 0.6 scn 0 0 32 16 re f",
            1,
        );
        assert!(doc.load_object(ObjectRef::new(6, 1)).is_err(), "real discarded mask");
        expect_sample(&doc, Ink::Spot, 16, 51, 204);
    }
}

#[test]
fn group_process_mirror_uses_evenodd_and_combined_path_coverage() {
    for knockout in [false, true] {
        for op in ["f", "f*", "B", "B*"] {
            let body = format!("/Half gs 0 0 0 0.6 k 0 0 32 16 re {op}");
            let doc = form(Ink::Black, true, knockout, "/Half gs", &body);
            let expected = mix_rgb(root_rgb(Ink::Black, "0.2"), root_rgb(Ink::Black, "0.6"), 0.25);
            expect_rgb_sample(&doc, Ink::Black, 16, expected, 77);
        }
    }
}

/// A synthetic embedded TrueType font, so the glyph-pixel tests do not depend
/// on the host's font database. A non-embedded Helvetica resolves through
/// `fontdb::Database::load_system_fonts()`, and the glyph that stands in for
/// it — therefore the sampled pixel row — differs between machines (issue
/// #2954). Embedded font programs bypass the system database entirely.
///
/// The fixture font maps only `M` (byte 0x4D, through a byte-indexed (1, 0)
/// cmap, the same shape `gh1632_whitespace_inferred_glyph` uses) to glyph 1: a
/// filled rectangle spanning x 250..750, y 0..800 of a 1000-unit em. At 14 pt
/// from `Td 4 2` its edges land on half-pixel boundaries (7.5 and 14.5), so a
/// 4 pt stroke centred on the outline covers whole pixels on both sides of
/// every edge — the row preconditions the tests below assert on.
fn fixture_font() -> Vec<u8> {
    common::build_box_glyph_font(common::BoxGlyphFont {
        outline: [250, 0, 750, 800],
        head_bbox: [250, 0, 750, 800],
        lsb: 250,
        cmap_byte: b'M',
    })
}

fn opaque_glyph_row(mode: u8) -> Vec<u8> {
    let content = format!("0 g 0 G 4 w {mode} Tr BT /F1 14 Tf 4 2 Td (M) Tj ET");
    let doc = document("", &content, "", "");
    let image = PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
        .render_page(&doc, 0)
        .expect("glyph control");
    image.data[8 * 32 * 4..9 * 32 * 4].to_vec()
}

/// The exact glyph row the fixture font paints at 14 pt from `4 2 Td` on
/// row 8 of the 32×16 page, indexed by render mode (0 fill, 1 stroke, 2
/// both), pixel by pixel. The glyph's rectangle spans 7.5..14.5 pt, so its
/// edges land on half-pixel boundaries and the 127 entries are half
/// coverage. Pinning the row — instead of searching it for a covered pixel
/// — is what keeps a silent fallback to system fonts (when the embedded
/// program fails to parse, `text_rasterizer.rs`) from satisfying the
/// coverage selections below: a Helvetica glyph matches a `find`, never
/// this row.
const GLYPH_ROWS: [[u8; 32]; 3] = [
    [
        255, 255, 255, 255, 255, 255, 255, 127, 0, 0, 0, 0, 0, 0, 127, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255,
    ],
    [
        255, 255, 255, 255, 255, 127, 0, 0, 0, 127, 255, 255, 127, 0, 0, 0, 127, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255, 255, 255,
    ],
    [
        255, 255, 255, 255, 255, 127, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 127, 255, 255, 255, 255, 255, 255, 255, 255, 255,
        255, 255, 255, 255, 255, 255,
    ],
];

/// Assert every pixel of the fixture glyph row matches [`GLYPH_ROWS`] for
/// `mode`.
fn expect_pinned_glyph_row(mode: usize) {
    let row = opaque_glyph_row(mode as u8);
    for x in 0..32 {
        let pixel = &row[x * 4..x * 4 + 4];
        assert_eq!(&pixel[..3], &[GLYPH_ROWS[mode][x]; 3], "render mode {mode} pixel {x}");
        assert_eq!(pixel[3], 255, "render mode {mode} pixel {x} alpha");
    }
}

#[test]
fn embedded_glyph_row_is_pinned_exactly() {
    for mode in 0..3 {
        expect_pinned_glyph_row(mode);
    }
}

/// Sample positions the pinned rows guarantee: x 8 is covered by both the
/// fill (x 8..13) and the stroke (x 6..8 and 13..15), x 6 is stroke-only
/// black over the fill row's white.
const FILL_AND_STROKE_X: usize = 8;
const STROKE_ONLY_X: usize = 6;

#[test]
fn group_process_text_mirror_respects_fill_and_stroke_order() {
    let parent = root_rgb(Ink::Black, "0.2");
    let strong = root_rgb(Ink::Black, "0.6");
    let weak = root_rgb(Ink::Black, "0.1");
    for knockout in [false, true] {
        for mode in [0, 1, 2] {
            let x = if mode == 1 { STROKE_ONLY_X } else { FILL_AND_STROKE_X };
            let (fill_tint, stroke_tint) = if mode == 1 { ("0.1", "0.6") } else { ("0.6", "0.1") };
            let body = format!(
                "/Half gs 0 0 0 {fill_tint} k 0 0 0 {stroke_tint} K 4 w {mode} Tr BT /F1 14 Tf 4 2 Td (M) Tj ET"
            );
            let doc = form(Ink::Black, true, knockout, "/Half gs", &body);
            let expected = if mode == 2 {
                std::array::from_fn(|i| {
                    (0.625 * f64::from(parent[i]) + 0.125 * f64::from(strong[i]) + 0.25 * f64::from(weak[i])).round()
                        as u8
                })
            } else {
                mix_rgb(parent, strong, 0.25)
            };
            expect_rgb_sample(&doc, Ink::Black, x, expected, if mode == 2 { 57 } else { 77 });
        }
    }
}

#[test]
fn rgb_stroke_erases_black_process_under_combined_path() {
    for knockout in [false, true] {
        for op in ["B", "B*"] {
            let body = format!("0 0 0 0.6 k 1 1 1 RG 4 w 4 0 24 16 re {op}");
            let doc = form(Ink::Black, true, knockout, "", &body);
            expect_rgb_sample(&doc, Ink::Black, 4, [255; 3], 0);
            expect_rgb_sample(&doc, Ink::Black, 16, root_rgb(Ink::Black, "0.6"), 153);
        }
    }
}

#[test]
fn glyph_fill_and_stroke_write_only_their_own_spot_ink() {
    let x = STROKE_ONLY_X;
    let pixel = 8 * 32 + x;
    for knockout in [false, true] {
        for mode in [1, 2] {
            let body =
                format!("/Ink cs 0.8 scn /Other CS 0.6 SCN /Half gs 4 w {mode} Tr BT /F1 14 Tf 4 2 Td (M) Tj ET");
            let doc = document(
                "",
                "/Fm Do",
                &format!("/Group << /S /Transparency /I true /K {knockout} >>"),
                &body,
            );
            let fill = render_separation(&doc, 0, "TestInk", 72).expect("fill spot plate");
            let stroke = render_separation(&doc, 0, "OtherInk", 72).expect("stroke spot plate");
            assert_eq!(
                fill.data[pixel], 0,
                "stroke-only geometry cannot write fill ink; Tr={mode}"
            );
            assert!(
                stroke.data[pixel].abs_diff(77) <= 1,
                "stroke must write its own half-opaque ink; Tr={mode}: {}",
                stroke.data[pixel]
            );
        }
    }
}

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

#[test]
fn icc_text_stroke_preserves_opaque_fill_on_process_plate() {
    let profile = build_constant_cmyk_icc(155);
    let content = "/StrokeHalf gs 0 0 0 1 k 1 0 0 0 K 4 w 2 Tr BT /F1 14 Tf 4 2 Td (M) Tj ET";
    let doc = document_with_profile("", content, "", "", 0, Some(&profile));
    let x = FILL_AND_STROKE_X;
    let mut renderer = PageRenderer::new(RenderOptions::with_dpi(72).as_raw());
    renderer.render_page(&doc, 0).expect("ICC glyph page");
    let inks = renderer
        .cmyk_sidecar_cmyk_bytes()
        .expect("ICC process sidecar allocated");
    let px = (8 * 32 + x) * 4;
    assert!(inks[px].abs_diff(128) <= 1, "half Cyan stroke: {}", inks[px]);
    assert!(
        inks[px + 3].abs_diff(128) <= 1,
        "Black under half Cyan stroke: {}",
        inks[px + 3]
    );
}

#[test]
fn overlapping_text_keeps_glyph_paint_order_under_icc() {
    let profile = build_constant_cmyk_icc(155);
    let prologue = "/Half gs 1 0 0 0 k 0 0 0 1 K 3 w 2 Tr BT /F1 14 Tf 4 2 Td";
    let compare = |adjustment: i32| {
        let both = format!("{prologue} [(M) {adjustment} (M)] TJ ET");
        let split = format!("{prologue} [(M) {adjustment}] TJ (M) Tj ET");
        let render = |text: &str| {
            let doc = document_with_profile("", text, "", "", 0, Some(&profile));
            render_separation(&doc, 0, "Black", 72).expect("Black glyph plate").data
        };
        (render(&both), render(&split))
    };
    let (both, split) = compare(-1_000);
    assert_eq!(both, split, "disjoint glyph control");
    let (both, split) = compare(750);
    assert!(split.contains(&128), "real half-opacity stroke");
    assert!(both == split, "overlapping glyphs must retain their paint order");
}

#[test]
fn cropped_glyph_plate_uses_the_same_fractional_clip_as_visible_ink() {
    let mut fractional = 0;
    for x in [5.25, 6.25, 7.25, 8.25, 9.25, 10.25, 11.25, 12.25, 13.25, 14.25] {
        let body = format!("{x} 0 16 16 re W n /Ink cs 1 scn BT /F1 14 Tf 4 2 Td (M) Tj ET");
        let doc = document("", "/Fm Do", "/Group << /S /Transparency /I true >>", &body);
        let image = PageRenderer::new(RenderOptions::with_dpi(72).as_raw())
            .render_page(&doc, 0)
            .expect("clipped glyph pixels");
        let plate = render_separation(&doc, 0, "TestInk", 72).expect("clipped glyph plate");
        for (rgba, &ink) in image.data.as_chunks::<4>().0.iter().zip(&plate.data) {
            fractional += usize::from(rgba[0] > 0 && rgba[0] < 255);
            assert!(
                ink.abs_diff(255 - rgba[0]) <= 1,
                "clipped coverage at {x}: plate={ink}, gray={}",
                rgba[0]
            );
        }
    }
    assert!(fractional > 0, "actual fractional pixels");
}

#[test]
fn singular_spot_stroke_text_has_no_ink_and_render_continues() {
    let show = "/Other CS 1 SCN 2 w 1 Tr BT /F1 14 Tf 4 2 Td (M) Tj (M) Tj ET";
    for transform in ["", "0 0 0 1 10 0 cm"] {
        let body = format!("q {transform} {show} Q /Ink cs 0.6 scn 28 0 4 16 re f");
        let doc = document("", "/Fm Do", "/Group << /S /Transparency /I true >>", &body);
        let stroke = render_separation(&doc, 0, "OtherInk", 72).expect("spot stroke render completes");
        assert_eq!(stroke.data.iter().any(|&v| v > 0), transform.is_empty());
        expect_sample(&doc, Ink::Spot, 30, 102, 153);
    }
}
