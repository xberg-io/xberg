use super::*;
use crate::types::internal::ElementKind;

#[test]
fn should_ignore_only_known_hocr_run_bookkeeping_in_a_plain_page() {
    let mut doc = super::super::document::flat_ocr_page_document("real source words");
    doc.metadata
        .additional
        .insert("source_format".into(), serde_json::json!("hocr"));
    doc.metadata
        .additional
        .insert("table_count".into(), serde_json::json!(0));
    assert!(has_only_certified_tesseract_text(&doc));
    assert!(!super::super::document::ocr_page_has_only_plain_payload(&doc));

    let mut unknown = doc.clone();
    unknown
        .metadata
        .additional
        .insert("unknown".into(), serde_json::json!(0));
    assert!(!has_only_certified_tesseract_text(&unknown));

    let mut invalid = doc.clone();
    invalid
        .metadata
        .additional
        .insert("table_count".into(), serde_json::json!(1));
    assert!(!has_only_certified_tesseract_text(&invalid));

    doc.elements[0].kind = ElementKind::Heading { level: 1 };
    assert!(!has_only_certified_tesseract_text(&doc));
}

#[test]
fn should_compare_a_table_read_to_its_words_without_the_table_markup() {
    let content = "| Value | Type |\n| --- | --- |\n| 00H | General |";
    let words = ["Value", "Type", "00H", "General", "|", "|"];
    assert!(content_holds_exactly_the_read_words(content, words.iter().copied()));
    assert!(!content_holds_exactly_the_read_words(
        content,
        ["Value", "Type", "00H"].iter().copied()
    ));
    assert!(!content_holds_exactly_the_read_words(
        content,
        ["Value", "Type", "00H", "Special"].iter().copied()
    ));
    assert!(
        !content_holds_exactly_the_read_words(
            "Value Type 00H General",
            ["General", "00H", "Type", "Value"].iter().copied()
        ),
        "a page without table markup keeps the exact word order"
    );
}

#[test]
fn should_undo_the_pipe_escape_of_a_table_cell_when_comparing_a_read() {
    let content = "| Name | Value |\n| --- | --- |\n| \\|A32_X2APIC_PPR | 0 |";
    let words = ["Name", "Value", "|A32_X2APIC_PPR", "0"];
    assert!(content_holds_exactly_the_read_words(content, words.iter().copied()));
    assert!(
        !content_holds_exactly_the_read_words(content, ["Name", "Value", "A32_X2APIC_PPR", "0"].iter().copied()),
        "the leading bar is part of the read word"
    );
}

#[test]
fn should_reject_a_table_read_whose_words_differ_from_the_iterator_words() {
    let content = "| Value | Type |\n| --- | --- |";
    assert!(content_holds_exactly_the_read_words(
        content,
        ["Value", "Type"].iter().copied()
    ));
    assert!(!content_holds_exactly_the_read_words(
        content,
        ["Value", "Kind"].iter().copied()
    ));
}

#[test]
fn should_certify_a_page_of_text_and_tables_only_when_it_holds_exactly_the_read_words() {
    let read = "| Value | Type |\n| --- | --- |\n| 00H | General |";
    let mut doc = super::super::document::flat_ocr_page_document("Table 3-12 Encoding");
    let index = doc.push_table(crate::types::Table {
        cells: vec![
            vec!["Value".into(), "Type".into()],
            vec!["00H".into(), "General".into()],
        ],
        markdown: read.into(),
        page_number: 1,
        ..Default::default()
    });
    doc.elements.push(crate::types::internal::InternalElement::text(
        ElementKind::Table { table_index: index },
        "",
        0,
    ));
    let page_read = format!("Table 3-12 Encoding {read}");
    assert!(only_segments_certified_tesseract_text_and_tables(&doc, &page_read));
    assert!(
        !only_segments_certified_tesseract_text_and_tables(&doc, &format!("{page_read} Extra")),
        "a read word the structure lacks is content the native page would drop"
    );
    assert!(!only_segments_certified_tesseract_text(&doc, &page_read));

    let mut extra_cell = doc.clone();
    extra_cell.tables[0].cells[1].push("Extra".into());
    assert!(
        !only_segments_certified_tesseract_text_and_tables(&extra_cell, &page_read),
        "a cell word the certified markdown lacks would be emitted uncertified"
    );

    let mut with_image = doc.clone();
    with_image.images.push(Default::default());
    assert!(!only_segments_certified_tesseract_text_and_tables(
        &with_image,
        &page_read
    ));
}

mod glued_cells {
    use super::*;

    const HEIGHT: f32 = 10.0;

    fn at(left: f32, right: f32) -> LineBox {
        LineBox {
            left,
            top: 0.0,
            right,
            bottom: HEIGHT,
        }
    }

    fn glues(native: LineBox, read: &[(&str, LineBox)]) -> bool {
        glues_token("abcd", native, read)
    }

    fn glues_token(token: &str, native: LineBox, read: &[(&str, LineBox)]) -> bool {
        let read: Vec<(String, LineBox)> = read.iter().map(|(text, place)| ((*text).to_owned(), *place)).collect();
        native_word_glues_separate_cells(&[(token.to_owned(), native)], &read)
    }

    #[test]
    fn should_not_veto_a_gap_just_below_the_box_height() {
        assert!(!glues(
            at(0.0, 100.0),
            &[("ab", at(0.0, 40.0)), ("cd", at(49.9, 100.0))]
        ));
    }

    #[test]
    fn should_veto_a_gap_just_above_the_box_height() {
        assert!(glues(at(0.0, 100.0), &[("ab", at(0.0, 40.0)), ("cd", at(50.1, 100.0))]));
    }

    #[test]
    fn should_not_veto_a_split_inside_a_word() {
        assert!(!glues(
            at(0.0, 100.0),
            &[("ab", at(0.0, 48.0)), ("cd", at(51.0, 100.0))]
        ));
    }

    #[test]
    fn should_not_veto_a_gap_that_punctuation_fills() {
        let dots = ("....", at(30.0, 70.0));
        assert!(!glues(
            at(0.0, 100.0),
            &[("ab", at(0.0, 30.0)), dots, ("cd", at(70.0, 100.0))]
        ));
        assert!(glues(at(0.0, 100.0), &[("ab", at(0.0, 30.0)), ("cd", at(70.0, 100.0))]));
    }

    #[test]
    fn should_still_veto_a_gap_with_a_table_rule_read_in_it() {
        let rule = ("|", at(47.0, 48.0));
        assert!(glues(
            at(0.0, 100.0),
            &[("ab", at(0.0, 40.0)), rule, ("cd", at(55.0, 100.0))]
        ));
    }

    #[test]
    fn should_still_veto_a_gap_with_a_lone_dot_read_in_it() {
        let dot = (".", at(47.0, 48.0));
        assert!(glues(
            at(0.0, 100.0),
            &[("ab", at(0.0, 40.0)), dot, ("cd", at(55.0, 100.0))]
        ));
    }

    #[test]
    fn should_close_a_gap_with_a_leader_run_of_dots_or_an_ellipsis() {
        for leader in ["...", "......", "\u{2026}", "_____", "-----"] {
            assert!(
                !glues(
                    at(0.0, 100.0),
                    &[("ab", at(0.0, 40.0)), (leader, at(42.0, 53.0)), ("cd", at(55.0, 100.0))]
                ),
                "{leader}"
            );
        }
    }

    #[test]
    fn should_not_veto_a_label_joined_to_its_value_by_a_leader_the_read_dropped() {
        let read = [("Introduction", at(0.0, 60.0)), ("12", at(90.0, 100.0))];
        assert!(!glues_token("Introduction.........12", at(0.0, 100.0), &read));
        assert!(!glues_token("Introduction\u{2026}12", at(0.0, 100.0), &read));
        assert!(glues_token("Introduction12", at(0.0, 100.0), &read));
    }

    #[test]
    fn should_measure_a_gap_from_the_furthest_edge_of_the_words_before_it() {
        let read = [("wide", at(0.0, 100.0)), ("x", at(10.0, 20.0)), ("y", at(105.0, 115.0))];
        assert!(!glues(at(0.0, 130.0), &read));
        let far = [("wide", at(0.0, 100.0)), ("x", at(10.0, 20.0)), ("y", at(115.0, 125.0))];
        assert!(glues(at(0.0, 130.0), &far));
    }

    #[test]
    fn should_not_veto_a_box_without_height() {
        let flat = LineBox {
            top: 5.0,
            bottom: 5.0,
            ..at(0.0, 100.0)
        };
        assert!(!glues(flat, &[("ab", at(0.0, 30.0)), ("cd", at(70.0, 100.0))]));
    }

    #[test]
    fn should_keep_the_prior_outcome_for_a_word_set_sideways() {
        let tall = LineBox {
            left: 0.0,
            top: 0.0,
            right: HEIGHT,
            bottom: 100.0,
        };
        let part = |top: f32, bottom: f32| LineBox {
            left: 0.0,
            top,
            right: HEIGHT,
            bottom,
        };
        assert!(!glues(tall, &[("ab", part(0.0, 30.0)), ("cd", part(70.0, 100.0))]));
    }
}

mod ink_ownership {
    use super::*;
    use crate::types::Metadata;

    const PAGE: (f32, f32) = (612.0, 792.0);
    const SCALE: f32 = 2.0;
    const WORKING_BYTES: usize = 64 << 20;
    const PROSE: [&str; 6] = [
        "The committee met on Tuesday to review the plans for the library.",
        "Members asked about the schedule, budget, and planned parking.",
        "The architect said work would start after the winter season ends.",
        "A vote is planned for the next regular meeting of the council.",
        "Residents may send written comments before the end of the month.",
        "The treasurer reported that the reserve fund remained steady this year.",
    ];
    const LAYER: &str = concat!(
        "mov eax, ebx\n",
        "call puts\n",
        "The committee met on Tuesday to review the plans for the library.\n",
        "Members asked about the schedule, budget, and planned parking.\n",
        "The architect said work would start after the winter season ends.\n",
        "A vote is planned for the next regular meeting of the council.\n",
        "Residents may send written comments before the end of the month.\n",
        "The treasurer reported that the reserve fund remained steady this year.",
    );

    fn pdf_with(extra_content: &str) -> PdfDocument {
        pdf_between("", extra_content)
    }

    fn pdf_between(under_text: &str, over_text: &str) -> PdfDocument {
        pdf_with_lost_glyphs(under_text, over_text, &[])
    }

    /// A page whose text also holds one glyph without a Unicode value at each of `lost`.
    fn pdf_with_lost_glyphs(under_text: &str, over_text: &str, lost: &[(i32, i32, i32)]) -> PdfDocument {
        use lopdf::{Document, Object, Stream, dictionary};

        let prose: String = PROSE
            .iter()
            .enumerate()
            .map(|(at, line)| format!("BT /F1 12 Tf 72 {} Td ({line}) Tj ET\n", 640 - 20 * at))
            .collect();
        let lost: String = lost
            .iter()
            .map(|(x, y, size)| format!("BT /F2 {size} Tf {x} {y} Td <0044> Tj ET\n"))
            .collect();
        let content = format!(
            "{under_text}\nBT /F1 12 Tf 72 700 Td (mov eax, ebx) Tj ET\n\
             BT /F1 12 Tf 72 680 Td (call puts) Tj ET\n{prose}\n{lost}{over_text}"
        );
        let mut document = Document::with_version("1.5");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Courier",
        });
        let cmap = b"begincmap /CMapName /Probe def /CMapType 2 def \
            1 begincodespacerange <0000> <FFFF> endcodespacerange \
            5 beginbfchar <0041> <0041> <0044> <0000> <0063> <0063> <0061> <0061> <0066> <0066> endbfchar endcmap";
        let unicode_id = document.add_object(Stream::new(dictionary! {}, cmap.to_vec()));
        let program = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../xberg-native-pdf/tests/fixtures/fonts/DejaVuSansMono.ttf"
        ));
        let program_id = document.add_object(Stream::new(
            dictionary! { "Length1" => program.len() as i64 },
            program.to_vec(),
        ));
        let descriptor_id = document.add_object(dictionary! {
            "Type" => "FontDescriptor", "FontName" => "ABCDEF+Probe", "Flags" => 4,
            "FontFile2" => program_id,
            "FontBBox" => vec![0.into(), (-200).into(), 1000.into(), 800.into()],
            "ItalicAngle" => 0, "Ascent" => 800, "Descent" => -200, "CapHeight" => 700, "StemV" => 80,
        });
        let descendant_id = document.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "ABCDEF+Probe",
            "CIDSystemInfo" => dictionary! {"Registry" => Object::string_literal("Adobe"),
                "Ordering" => Object::string_literal("Identity"), "Supplement" => 0},
            "FontDescriptor" => descriptor_id, "DW" => 600, "CIDToGIDMap" => "Identity",
        });
        let lost_font_id = document.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "ABCDEF+Probe",
            "Encoding" => "Identity-H", "DescendantFonts" => vec![descendant_id.into()],
            "ToUnicode" => unicode_id,
        });
        let content_id = document.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id, "F2" => lost_font_id } },
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1,
            }),
        );
        let catalog_id = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        document.trailer.set("Root", catalog_id);
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        PdfDocument::from_bytes(bytes).unwrap()
    }

    fn iterator_word(text: &str, placed: Rect) -> OcrElement {
        let raster_height = PAGE.1 * SCALE;
        OcrElement {
            text: text.to_owned(),
            level: OcrElementLevel::Word,
            geometry: OcrBoundingGeometry::Rectangle {
                left: (placed.x * SCALE) as u32,
                top: (raster_height - (placed.y + placed.height) * SCALE) as u32,
                width: (placed.width * SCALE) as u32,
                height: (placed.height * SCALE) as u32,
            },
            backend_metadata: [("backend".to_owned(), serde_json::json!("tesseract-iterator"))].into(),
            ..Default::default()
        }
    }

    fn read_of(words: Vec<OcrElement>) -> ExtractedDocument {
        let mut metadata = Metadata::default();
        metadata.additional.insert(
            OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
            serde_json::json!((PAGE.0 * SCALE) as u32),
        );
        metadata.additional.insert(
            OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
            serde_json::json!((PAGE.1 * SCALE) as u32),
        );
        ExtractedDocument {
            content: words
                .iter()
                .map(|word| word.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            ocr_elements: Some(words),
            metadata,
            ..Default::default()
        }
    }

    fn ink_owned(pdf: &PdfDocument, layer: &str, read: &ExtractedDocument) -> bool {
        native_text_paints_all_read_ink(
            pdf,
            0,
            layer,
            read,
            InkProofLimits {
                thresholds: &OcrQualityThresholds::default(),
                max_working_bytes: WORKING_BYTES,
                cancelled: &|| false,
            },
        )
    }

    fn native_word_boxes(pdf: &PdfDocument) -> Vec<(String, Rect)> {
        pdf.extract_text_lines(0)
            .unwrap()
            .iter()
            .flat_map(|line| line.words.iter().map(|word| (word.text.clone(), word.bbox)))
            .collect()
    }

    fn owns(pdf: &PdfDocument, read_texts: Option<&[&str]>) -> bool {
        let boxes = native_word_boxes(pdf);
        assert_eq!(boxes.len(), LAYER.split_whitespace().count(), "{boxes:?}");
        let words = boxes
            .iter()
            .enumerate()
            .map(|(at, (text, placed))| {
                iterator_word(
                    read_texts
                        .and_then(|texts| texts.get(at).copied())
                        .unwrap_or(text.as_str()),
                    *placed,
                )
            })
            .collect();
        ink_owned(pdf, LAYER, &read_of(words))
    }

    #[test]
    fn should_own_a_read_whose_words_differ_from_the_native_words_it_sits_on() {
        let pdf = pdf_with("");
        assert!(owns(&pdf, None));
        assert!(owns(&pdf, Some(&["rnov", "eax.", "ebx", "ca11", "puts"])));
    }

    /// The read of `pdf` with its widest native word read as two words at its two ends, `apart`
    /// or touching in the middle of the word.
    fn read_splitting_widest_word(pdf: &PdfDocument, apart: bool) -> ExtractedDocument {
        let boxes = native_word_boxes(pdf);
        let widest = (0..boxes.len())
            .max_by(|a, b| boxes[*a].1.width.total_cmp(&boxes[*b].1.width))
            .unwrap();
        let words = boxes
            .iter()
            .enumerate()
            .flat_map(|(at, (text, placed))| {
                if at != widest {
                    return vec![iterator_word(text, *placed)];
                }
                let piece = if apart { placed.width * 0.2 } else { placed.width / 2.0 };
                let (head, tail) = text.split_at(text.len() / 2);
                vec![
                    iterator_word(
                        head,
                        Rect {
                            width: piece,
                            ..*placed
                        },
                    ),
                    iterator_word(
                        tail,
                        Rect {
                            x: placed.x + placed.width - piece,
                            width: piece,
                            ..*placed
                        },
                    ),
                ]
            })
            .collect();
        read_of(words)
    }

    #[test]
    fn should_not_own_a_native_word_that_glues_read_words_a_cell_apart() {
        let pdf = pdf_with("");
        assert!(!ink_owned(&pdf, LAYER, &read_splitting_widest_word(&pdf, true)));
    }

    #[test]
    fn should_own_a_native_word_whose_read_words_touch() {
        let pdf = pdf_with("");
        assert!(ink_owned(&pdf, LAYER, &read_splitting_widest_word(&pdf, false)));
    }

    #[test]
    fn should_not_own_a_word_over_a_fill_that_ends_inside_it() {
        let pdf = pdf_between("0.6 g 72 696 10 14 re f 0 g", "");
        assert!(!owns(&pdf, None));
    }

    #[test]
    fn should_own_text_on_a_flat_shaded_background() {
        let pdf = pdf_between("0.6 g 60 670 200 45 re f 0 g", "");
        assert!(owns(&pdf, None));
    }

    #[test]
    fn should_not_own_a_word_painted_over_by_an_opaque_rectangle() {
        let pdf = pdf_with("1 g 72 696 43 14 re f");
        assert!(!owns(&pdf, None));
    }

    #[test]
    fn should_not_own_a_word_struck_by_a_vector_stroke() {
        let pdf = pdf_with("0 G 1 w 70 702 m 130 702 l S");
        assert!(!owns(&pdf, None));
    }

    #[test]
    fn should_not_own_a_stroked_box_outside_every_native_line() {
        let pdf = pdf_with("0 G 1 w 150 676 m 190 676 l 190 690 l 150 690 l h S");
        let mut boxes = native_word_boxes(&pdf);
        let outlined = Rect {
            x: 150.0,
            y: 676.0,
            width: 40.0,
            height: 14.0,
        };
        boxes.push(("outlined".into(), outlined));
        let words = boxes
            .iter()
            .map(|(text, placed)| iterator_word(text, *placed))
            .collect();
        assert!(!ink_owned(
            &pdf,
            "mov eax, ebx
call puts",
            &read_of(words)
        ));
    }

    #[test]
    fn should_own_a_read_word_on_blank_paper_inside_a_native_line() {
        let pdf = pdf_with("");
        let boxes = native_word_boxes(&pdf);
        let (first, second) = (boxes[0].1, boxes[1].1);
        let gap = Rect {
            x: first.x + first.width + 1.0,
            y: first.y,
            width: second.x - first.x - first.width - 2.0,
            height: first.height,
        };
        assert!(gap.width > 1.0, "{first:?} {second:?}");
        let mut words: Vec<_> = boxes
            .iter()
            .map(|(text, placed)| iterator_word(text, *placed))
            .collect();
        words.push(iterator_word("ghost", gap));
        assert!(ink_owned(&pdf, LAYER, &read_of(words)));
    }

    #[test]
    fn should_not_own_a_read_when_no_word_shows_glyph_ink() {
        let pdf = pdf_with("");
        let boxes = native_word_boxes(&pdf);
        let first = boxes[0].1;
        let blank = Rect {
            x: first.x,
            y: first.y + 3.0 * first.height,
            width: first.width,
            height: first.height,
        };
        assert!(!ink_owned(
            &pdf,
            "mov eax, ebx",
            &read_of(vec![iterator_word("mov", blank)])
        ));
    }

    #[test]
    fn should_not_own_a_word_box_shifted_mostly_off_its_line() {
        let pdf = pdf_with("");
        let mut boxes = native_word_boxes(&pdf);
        let last = boxes.len() - 1;
        boxes[last].1.y -= 0.6 * boxes[last].1.height;
        let words = boxes
            .iter()
            .map(|(text, placed)| iterator_word(text, *placed))
            .collect();
        assert!(!ink_owned(&pdf, LAYER, &read_of(words)));
    }

    #[test]
    fn should_not_own_a_page_whose_native_text_has_a_single_undecodable_character() {
        let pdf = pdf_with("");
        let words = native_word_boxes(&pdf)
            .iter()
            .map(|(text, placed)| iterator_word(text, *placed))
            .collect();
        assert!(!ink_owned(
            &pdf,
            &LAYER.replace("budget", "bud\u{E000}et"),
            &read_of(words)
        ));
    }

    fn read_matching_ink(pdf: &PdfDocument) -> ExtractedDocument {
        let words = native_word_boxes(pdf)
            .iter()
            .map(|(text, placed)| iterator_word(text, *placed))
            .collect();
        read_of(words)
    }

    /// `layer` with each word for which `wrong(index)` holds replaced by one the page never shows.
    fn layer_with_words_wrong(wrong: impl Fn(usize) -> bool) -> String {
        LAYER
            .split(' ')
            .enumerate()
            .map(|(at, word)| if wrong(at) { "zzzz" } else { word })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// A read of `pdf` whose first `misread` words with letters or digits are words the page never shows.
    fn read_misreading(pdf: &PdfDocument, misread: usize) -> (ExtractedDocument, usize) {
        let mut left = misread;
        let mut solid = 0;
        let words = native_word_boxes(pdf)
            .iter()
            .map(|(text, placed)| {
                if text.chars().any(char::is_alphanumeric) {
                    solid += 1;
                    if left > 0 {
                        left -= 1;
                        return iterator_word("zzzz", *placed);
                    }
                }
                iterator_word(text, *placed)
            })
            .collect();
        (read_of(words), solid)
    }

    #[test]
    fn should_own_a_read_that_misreads_up_to_the_agreement_floor() {
        let pdf = pdf_with("");
        let (_, solid) = read_misreading(&pdf, 0);
        let allowed = ((1.0 - MIN_NATIVE_READ_AGREEMENT) * solid as f64).floor() as usize;
        assert!(allowed > 0, "{solid} words leave room for a misread");
        assert!(ink_owned(&pdf, LAYER, &read_misreading(&pdf, allowed).0));
    }

    #[test]
    fn should_not_own_a_read_that_misreads_past_the_agreement_floor() {
        let pdf = pdf_with("");
        let (_, solid) = read_misreading(&pdf, 0);
        let allowed = ((1.0 - MIN_NATIVE_READ_AGREEMENT) * solid as f64).floor() as usize;
        assert!(!ink_owned(&pdf, LAYER, &read_misreading(&pdf, allowed + 1).0));
    }

    #[test]
    fn should_not_own_a_read_that_contradicts_a_third_of_the_native_words() {
        let pdf = pdf_with("");
        let (_, solid) = read_misreading(&pdf, 0);
        assert!(!ink_owned(&pdf, LAYER, &read_misreading(&pdf, solid / 3).0));
    }

    #[test]
    fn should_not_own_a_page_whose_native_word_lies_under_a_fill_the_read_omits() {
        let pdf = pdf_with("1 g 72 696 43 14 re f");
        let covered = |placed: &Rect| placed.x + placed.width <= 72.0 + 43.0;
        let words: Vec<_> = native_word_boxes(&pdf)
            .iter()
            .filter(|(_, placed)| !covered(placed))
            .map(|(text, placed)| iterator_word(text, *placed))
            .collect();
        assert!(words.len() < native_word_boxes(&pdf).len(), "a native word is covered");
        assert!(!ink_owned(&pdf, LAYER, &read_of(words)));
    }

    #[test]
    fn should_not_prefer_a_native_layer_the_read_does_not_confirm() {
        let pdf = pdf_with("");
        let read = read_matching_ink(&pdf);
        assert!(!ink_owned(&pdf, &layer_with_words_wrong(|_| true), &read));
    }

    const LOST_AT: [(i32, i32, i32); 4] = [(300, 740, 12), (300, 720, 12), (300, 700, 12), (300, 680, 12)];

    /// The layer and a read of `pdf` with the text `read_as(index)` over each lost glyph, if any.
    fn lost_glyph_page_over(
        lost: &[(i32, i32, i32)],
        over_text: &str,
        read_as: impl Fn(usize) -> Option<&'static str>,
    ) -> (PdfDocument, String, ExtractedDocument) {
        let pdf = pdf_with_lost_glyphs("", over_text, lost);
        let mut words = Vec::new();
        let mut seen = 0;
        for (text, placed) in native_word_boxes(&pdf) {
            if text.contains('\u{FFFD}') {
                let at = lost
                    .iter()
                    .position(|(x, y, size)| {
                        placed.x >= *x as f32 - 1.0
                            && placed.x <= (*x + *size) as f32
                            && placed.y >= *y as f32 - *size as f32
                            && placed.y <= (*y + *size) as f32
                    })
                    .expect("a lost glyph of the page");
                if let Some(read) = read_as(at) {
                    words.push(iterator_word(read, placed));
                }
                seen += 1;
            } else {
                words.push(iterator_word(&text, placed));
            }
        }
        assert_eq!(seen, lost.len());
        let layer = format!("{LAYER}{}", "\n\u{FFFD}".repeat(lost.len()));
        (pdf, layer, read_of(words))
    }

    fn lost_glyph_page_where(
        lost: &[(i32, i32, i32)],
        read_if: impl Fn(usize) -> bool,
    ) -> (PdfDocument, String, ExtractedDocument) {
        lost_glyph_page_over(lost, "", |at| read_if(at).then_some("O"))
    }

    fn lost_glyph_page(lost: &[(i32, i32, i32)], read_at: &[usize]) -> (PdfDocument, String, ExtractedDocument) {
        lost_glyph_page_where(lost, |at| read_at.contains(&at))
    }

    #[test]
    fn should_keep_native_text_whose_lost_glyphs_the_read_does_not_see() {
        let (pdf, layer, read) = lost_glyph_page(&LOST_AT, &[]);
        assert!(ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_keep_native_text_when_the_read_sees_few_of_its_lost_glyphs() {
        let (pdf, layer, read) = lost_glyph_page(&LOST_AT, &[0]);
        assert!(ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_take_the_read_when_it_sees_half_of_the_lost_glyphs() {
        let (pdf, layer, read) = lost_glyph_page(&LOST_AT, &[0, 2]);
        assert!(!ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_take_the_read_when_it_sees_most_of_the_lost_glyphs() {
        let (pdf, layer, read) = lost_glyph_page(&LOST_AT, &[0, 1, 3]);
        assert!(!ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_keep_native_text_when_the_read_sees_one_of_three_lost_glyphs() {
        let (pdf, layer, read) = lost_glyph_page(&LOST_AT[..3], &[1]);
        assert!(ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_take_the_read_when_it_sees_two_of_three_lost_glyphs() {
        let (pdf, layer, read) = lost_glyph_page(&LOST_AT[..3], &[0, 2]);
        assert!(!ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_keep_native_text_when_a_kind_of_one_glyph_is_read_among_many_unread() {
        let lost = [LOST_AT[0], LOST_AT[1], LOST_AT[2], LOST_AT[3], (420, 740, 18)];
        let (pdf, layer, read) = lost_glyph_page_where(&lost, |at| at == 4);
        assert!(ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_take_the_read_when_it_recovers_most_lost_glyphs_across_two_kinds() {
        let lost = [(420, 740, 18), LOST_AT[0], LOST_AT[1], LOST_AT[2]];
        let (pdf, layer, read) = lost_glyph_page_where(&lost, |at| at != 0);
        assert!(!ink_owned(&pdf, &layer, &read));
    }

    /// A recovered heading of twelve glyphs beside twenty unread symbols of another size.
    fn heading_beside_symbols() -> Vec<(i32, i32, i32)> {
        let heading = (0..12).map(|at| (590, 780 - 24 * at, 18));
        let symbols = (0..20).map(|at| (40, 760 - 30 * at, 12));
        heading.chain(symbols).collect()
    }

    #[test]
    fn should_take_the_read_that_recovers_a_heading_beside_many_unread_symbols() {
        let lost = heading_beside_symbols();
        let (pdf, layer, read) = lost_glyph_page_where(&lost, |at| at < 12);
        assert!(!ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_keep_native_text_when_only_the_symbols_beside_a_heading_are_unread_and_the_heading_is_too() {
        let lost = heading_beside_symbols();
        let (pdf, layer, read) = lost_glyph_page_where(&lost, |_| false);
        assert!(ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_not_count_a_stray_mark_over_a_lost_glyph_as_a_read() {
        let (pdf, layer, read) = lost_glyph_page_over(&LOST_AT, "", |at| (at < 3).then_some("|"));
        assert!(ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_count_a_letter_over_a_lost_glyph_as_a_read() {
        let (pdf, layer, read) = lost_glyph_page_over(&LOST_AT, "", |at| (at < 2).then_some("l"));
        assert!(!ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_keep_native_text_whose_two_lost_glyphs_are_unread() {
        let (pdf, layer, read) = lost_glyph_page(&LOST_AT[..2], &[]);
        assert!(ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_not_own_a_page_whose_lost_glyph_lies_under_a_fill() {
        let lost = [LOST_AT[0], LOST_AT[1]];
        let (pdf, layer, read) = lost_glyph_page_over(&lost, "1 g 296 738 16 14 re f", |_| None);
        assert!(!ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_not_own_a_faint_mark_between_native_words_that_the_read_sees() {
        let pdf = pdf_with("");
        let boxes = native_word_boxes(&pdf);
        let (first, second) = (boxes[0].1, boxes[1].1);
        let gap = Rect {
            x: first.x + first.width + 1.0,
            y: first.y,
            width: second.x - first.x - first.width - 2.0,
            height: first.height,
        };
        let faint = format!("0.84 g {} {} {} {} re f 0 g", gap.x, gap.y, gap.width / 2.0, gap.height);
        let pdf = pdf_with(&faint);
        let mut words: Vec<_> = native_word_boxes(&pdf)
            .iter()
            .map(|(text, placed)| iterator_word(text, *placed))
            .collect();
        words.push(iterator_word("ghost", gap));
        assert!(!ink_owned(&pdf, LAYER, &read_of(words)));
    }

    #[test]
    fn should_take_the_read_that_recovers_heading_glyphs_whose_sizes_differ_beside_unread_symbols() {
        let heading = (0..6).map(|at| (590, 780 - 24 * at, 18 + at));
        let symbols = (0..3).map(|at| (40, 760 - 30 * at, 12));
        let lost: Vec<_> = heading.chain(symbols).collect();
        let (pdf, layer, read) = lost_glyph_page_where(&lost, |at| at < 6);
        assert!(!ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_keep_native_text_when_only_a_few_fragmented_glyphs_are_read_among_unread_ones() {
        let heading = (0..6).map(|at| (590, 780 - 24 * at, 18 + at));
        let symbols = (0..3).map(|at| (40, 760 - 30 * at, 12));
        let lost: Vec<_> = heading.chain(symbols).collect();
        let (pdf, layer, read) = lost_glyph_page_where(&lost, |at| at == 0);
        assert!(ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_take_the_read_over_a_word_that_holds_a_lost_glyph_even_when_the_read_drops_it() {
        let pdf = pdf_with_lost_glyphs("", "BT /F2 12 Tf 300 500 Td <0063006100660044> Tj ET", &[]);
        let boxes = native_word_boxes(&pdf);
        assert!(boxes.iter().any(|(text, _)| text == "caf\u{FFFD}"), "{boxes:?}");
        let words = boxes
            .iter()
            .map(|(text, placed)| iterator_word(if text.contains('\u{FFFD}') { "caf" } else { text }, *placed))
            .collect();
        let layer = format!("{LAYER}\ncaf\u{FFFD}");
        assert!(!ink_owned(&pdf, &layer, &read_of(words)));
    }

    fn solid_words_of_prose_page() -> usize {
        native_word_boxes(&pdf_with(""))
            .iter()
            .filter(|(text, _)| text.chars().any(char::is_alphanumeric))
            .count()
    }

    fn margin_grid(count: usize) -> Vec<(i32, i32, i32)> {
        (0..count)
            .map(|at| (10 + 15 * (at / 40) as i32, 780 - 16 * (at % 40) as i32, 12))
            .collect()
    }

    #[test]
    fn should_keep_native_text_with_as_many_lost_glyphs_as_words() {
        let (pdf, layer, read) = lost_glyph_page(&margin_grid(solid_words_of_prose_page()), &[]);
        assert!(ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_take_the_read_when_lost_glyphs_outnumber_the_native_words() {
        let (pdf, layer, read) = lost_glyph_page(&margin_grid(solid_words_of_prose_page() + 1), &[]);
        assert!(!ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_take_the_read_that_sees_the_only_lost_glyph() {
        let (pdf, layer, read) = lost_glyph_page(&LOST_AT[..1], &[0]);
        assert!(!ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_keep_native_text_whose_only_lost_glyph_is_unread() {
        let (pdf, layer, read) = lost_glyph_page(&LOST_AT[..1], &[]);
        assert!(ink_owned(&pdf, &layer, &read));
    }

    #[test]
    fn should_not_own_a_layer_with_a_lost_glyph_that_no_native_word_holds() {
        let (pdf, layer, read) = lost_glyph_page(&LOST_AT, &[]);
        assert!(!ink_owned(&pdf, &format!("{layer}\n\u{FFFD}"), &read));
    }

    #[test]
    fn should_not_render_when_cancelled() {
        let pdf = pdf_with("");
        let words = native_word_boxes(&pdf)
            .iter()
            .map(|(text, placed)| iterator_word(text, *placed))
            .collect();
        assert!(!native_text_paints_all_read_ink(
            &pdf,
            0,
            LAYER,
            &read_of(words),
            InkProofLimits {
                thresholds: &OcrQualityThresholds::default(),
                max_working_bytes: WORKING_BYTES,
                cancelled: &|| true,
            },
        ));
    }
}
