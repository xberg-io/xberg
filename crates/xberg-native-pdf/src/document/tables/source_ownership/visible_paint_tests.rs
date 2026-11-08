use super::*;

fn page_with_fill(fill: &str) -> Vec<u8> {
    let rules = "0 0 m 10 0 l S\n".repeat(MIN_RULES);
    format!("{rules}BT {fill} /F1 10 Tf (x) Tj ET\n").into_bytes()
}

fn audited(fill: &str) -> bool {
    audit_operators(&page_with_fill(fill), |font| font == "F1")
}

#[test]
fn dark_text_fill_is_painted() {
    assert!(audited("0 g"));
    assert!(audited("0.1 0.2 0.3 rg"));
}

#[test]
fn light_text_fills_are_not_painted() {
    for fill in ["1 g", "0.9 g", "1 1 0 rg", "0 1 0 rg"] {
        assert!(!audited(fill), "{fill} fill unexpectedly audited as visible");
    }
}

fn rules(count: usize) -> String {
    "0 0 m 10 0 l S\n".repeat(count)
}

const TEXT: &str = "BT /F1 10 Tf (x) Tj ET\n";

fn audit(content: &str) -> bool {
    audit_operators(content.as_bytes(), |font| font == "F1")
}

#[test]
fn well_formed_page_is_audited() {
    assert!(audit(&format!("{}{TEXT}", rules(MIN_RULES))));
}

#[test]
fn restore_without_save_is_declined() {
    assert!(!audit(&format!("Q\n{}{TEXT}", rules(MIN_RULES))));
}

#[test]
fn unclosed_save_is_declined() {
    assert!(!audit(&format!("q\n{}{TEXT}", rules(MIN_RULES))));
}

#[test]
fn unclosed_text_object_is_declined() {
    assert!(!audit(&format!("{}BT /F1 10 Tf\n", rules(MIN_RULES))));
}

#[test]
fn unclosed_marked_content_is_declined() {
    assert!(!audit(&format!("{}/Span BMC\n{TEXT}", rules(MIN_RULES))));
}

#[test]
fn path_left_open_at_the_end_is_declined() {
    assert!(!audit(&format!("{}{TEXT}0 0 m\n", rules(MIN_RULES))));
}

#[test]
fn too_few_rules_is_declined() {
    assert!(!audit(&format!("{}{TEXT}", rules(MIN_RULES - 1))));
}

#[test]
fn invisible_text_render_mode_is_declined() {
    assert!(audit(&format!("{}BT 0 Tr /F1 10 Tf (x) Tj ET\n", rules(MIN_RULES))));
    assert!(!audit(&format!("{}BT 3 Tr /F1 10 Tf (x) Tj ET\n", rules(MIN_RULES))));
}

#[test]
fn unknown_font_is_declined() {
    assert!(!audit(&format!("{}BT /F2 10 Tf (x) Tj ET\n", rules(MIN_RULES))));
}

#[test]
fn non_finite_operands_are_declined() {
    let known = |font: &str| font == "F1";
    let mut state = PaintState::default();
    assert!(!state.path_operator(&Operator::MoveTo { x: f32::NAN, y: 0.0 }));
    assert!(state.path_operator(&Operator::MoveTo { x: 0.0, y: 0.0 }));
    assert!(!state.path_operator(&Operator::LineTo {
        x: f32::INFINITY,
        y: 0.0
    }));
    let mut state = PaintState::default();
    assert!(!state.path_operator(&Operator::SetLineWidth { width: f32::NAN }));
    assert!(!state.text_operator(
        Operator::Tf {
            font: "F1".into(),
            size: f32::INFINITY
        },
        known
    ));
}

mod page {
    use super::super::plain_page_bounds;
    use crate::PdfDocument;
    use crate::geometry::Rect;
    use std::io::Write;

    /// Comment lines of pseudo-random hex, so a Flate stream cannot shrink them past a ratio limit.
    fn padding(bytes: usize) -> Vec<u8> {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        let mut padding = Vec::with_capacity(bytes + 64);
        while padding.len() < bytes {
            padding.extend_from_slice(b"% ");
            for _ in 0..60 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                padding.push(b"0123456789abcdef"[(state >> 60) as usize]);
            }
            padding.push(b'\n');
        }
        padding
    }

    struct Page {
        page_extra: &'static str,
        resources_extra: &'static str,
        contents: &'static str,
        stream_extra: &'static str,
        font_subtype: &'static str,
        content: Vec<u8>,
        flate: bool,
    }

    impl Page {
        fn new() -> Self {
            Self {
                page_extra: "",
                resources_extra: "",
                contents: "4 0 R",
                stream_extra: "",
                font_subtype: "Type1",
                content: format!("{}BT /F1 10 Tf (x) Tj ET\n", super::rules(super::MIN_RULES)).into_bytes(),
                flate: false,
            }
        }

        fn bounds(self) -> Option<Rect> {
            let data = if self.flate {
                let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                encoder.write_all(&self.content).unwrap();
                encoder.finish().unwrap()
            } else {
                self.content.clone()
            };
            let filter = if self.flate { "/Filter /FlateDecode " } else { "" };
            let objects = [
                "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] {} /Contents {} \
                     /Resources << /Font << /F1 5 0 R >> {} >> >>",
                    self.page_extra, self.contents, self.resources_extra
                ),
                String::new(),
                format!("<< /Type /Font /Subtype /{} /BaseFont /Helvetica >>", self.font_subtype),
            ];
            let mut pdf = b"%PDF-1.4\n".to_vec();
            let mut offsets = Vec::new();
            for (index, body) in objects.iter().enumerate() {
                offsets.push(pdf.len());
                if index == 3 {
                    pdf.extend_from_slice(
                        format!(
                            "4 0 obj\n<< /Length {} {filter}{} >>\nstream\n",
                            data.len(),
                            self.stream_extra
                        )
                        .as_bytes(),
                    );
                    pdf.extend_from_slice(&data);
                    pdf.extend_from_slice(b"\nendstream\nendobj\n");
                } else {
                    pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
                }
            }
            let xref = pdf.len();
            pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
            for offset in offsets {
                pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
            }
            pdf.extend_from_slice(
                format!(
                    "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
                    objects.len() + 1
                )
                .as_bytes(),
            );
            let doc = PdfDocument::from_bytes(pdf).unwrap();
            plain_page_bounds(&doc, 0)
        }
    }

    #[test]
    fn plain_page_reports_its_media_box() {
        assert_eq!(Page::new().bounds(), Some(Rect::new(0.0, 0.0, 612.0, 792.0)));
    }

    #[test]
    fn flate_encoded_plain_page_is_accepted() {
        let mut page = Page::new();
        page.flate = true;
        assert!(page.bounds().is_some());
    }

    #[test]
    fn flate_content_under_the_limit_is_accepted() {
        let mut page = Page::new();
        page.content.extend(padding(super::super::MAX_CONTENT_BYTES / 2));
        page.flate = true;
        assert!(page.bounds().is_some());
    }

    #[test]
    fn matching_crop_box_is_accepted() {
        let mut page = Page::new();
        page.page_extra = "/CropBox [0 0 612 792]";
        assert!(page.bounds().is_some());
    }

    #[test]
    fn differing_crop_box_is_declined() {
        let mut page = Page::new();
        page.page_extra = "/CropBox [0 0 300 300]";
        assert!(page.bounds().is_none());
    }

    #[test]
    fn annotations_group_and_user_unit_are_declined() {
        for extra in ["/Annots []", "/Group << /S /Transparency >>", "/UserUnit 2"] {
            let mut page = Page::new();
            page.page_extra = extra;
            assert!(page.bounds().is_none(), "{extra} page unexpectedly accepted");
        }
    }

    #[test]
    fn rotated_page_is_declined() {
        let mut page = Page::new();
        page.page_extra = "/Rotate 90";
        assert!(page.bounds().is_none());
    }

    #[test]
    fn graphics_state_colour_space_and_property_resources_are_declined() {
        for extra in [
            "/ExtGState << /G << /CA 0 >> >>",
            "/ColorSpace << /C /DeviceGray >>",
            "/Properties << >>",
        ] {
            let mut page = Page::new();
            page.resources_extra = extra;
            assert!(page.bounds().is_none(), "{extra} resources unexpectedly accepted");
        }
    }

    #[test]
    fn composite_font_is_declined() {
        let mut page = Page::new();
        page.font_subtype = "Type0";
        assert!(page.bounds().is_none());
    }

    #[test]
    fn content_array_is_declined() {
        let mut page = Page::new();
        page.contents = "[4 0 R]";
        assert!(page.bounds().is_none());
    }

    #[test]
    fn decode_parameters_are_declined() {
        let mut page = Page::new();
        page.stream_extra = "/DecodeParms << >>";
        assert!(page.bounds().is_none());
    }

    #[test]
    fn oversized_stored_content_is_declined() {
        let mut page = Page::new();
        page.content.extend(padding(super::super::MAX_CONTENT_BYTES));
        assert!(page.bounds().is_none());
    }

    #[test]
    fn oversized_decoded_content_is_declined() {
        let mut page = Page::new();
        page.content.extend(padding(super::super::MAX_CONTENT_BYTES));
        page.flate = true;
        assert!(page.bounds().is_none());
    }
}

mod source_stream {
    use super::super::matches_source_stream;
    use crate::object::Object;
    use std::io::Write;

    fn deflate(content: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(content).unwrap();
        encoder.finish().unwrap()
    }

    fn flate() -> Object {
        Object::Name("FlateDecode".to_string())
    }

    #[test]
    fn unfiltered_stream_must_equal_its_content() {
        assert!(matches_source_stream(None, b"0 0 m", b"0 0 m"));
        assert!(!matches_source_stream(None, b"0 0 m", b"0 0 l"));
    }

    #[test]
    fn flate_stream_must_decode_to_exactly_its_content() {
        let raw = deflate(b"0 0 m 1 1 l S");
        assert!(matches_source_stream(Some(&flate()), &raw, b"0 0 m 1 1 l S"));
        assert!(!matches_source_stream(Some(&flate()), &raw, b"0 0 m 1 1 l"));
        assert!(!matches_source_stream(Some(&flate()), &raw, b"0 0 m 1 1 l S "));
        assert!(!matches_source_stream(Some(&flate()), &raw, b"0 0 m 1 1 l T"));
    }

    #[test]
    fn flate_stream_with_trailing_bytes_is_declined() {
        let mut raw = deflate(b"0 0 m 1 1 l S");
        raw.extend_from_slice(b"garbage");
        assert!(!matches_source_stream(Some(&flate()), &raw, b"0 0 m 1 1 l S"));
    }

    #[test]
    fn truncated_flate_stream_is_declined() {
        let raw = deflate(b"0 0 m 1 1 l S");
        assert!(!matches_source_stream(
            Some(&flate()),
            &raw[..raw.len() - 2],
            b"0 0 m 1 1 l S"
        ));
    }

    #[test]
    fn other_filters_are_declined() {
        let filter = Object::Name("ASCIIHexDecode".to_string());
        assert!(!matches_source_stream(Some(&filter), b"30>", b"0"));
    }
}
