use super::*;
use crate::ocr::table::table_retains_region_words;

fn fragment_policy(config: &TesseractConfig) -> FragmentedTablePolicy<'_> {
    FragmentedTablePolicy {
        config,
        allow_dot_leader: false,
    }
}

#[test]
fn fragmented_bound_conversion_requires_finite_integral_u32_coordinates() {
    let valid = BoundingBox {
        x0: 0.0,
        y0: 1.0,
        x1: f64::from(u32::MAX),
        y1: 40.0,
    };
    assert_eq!(fragmented_bound_coordinates(&valid), Some([0, 1, u32::MAX, 40]));
    for invalid in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        0.5,
        f64::from(u32::MAX) + 1.0,
    ] {
        for index in 0..4 {
            let mut coords = [valid.x0, valid.y0, valid.x1, valid.y1];
            coords[index] = invalid;
            let bbox = BoundingBox {
                x0: coords[0],
                y0: coords[1],
                x1: coords[2],
                y1: coords[3],
            };
            assert!(
                fragmented_bound_coordinates(&bbox).is_none(),
                "coordinate {index}: {invalid}"
            );
        }
    }
}

#[test]
fn fragmented_mismatched_tight_bounds_leave_document_and_all_tables_unchanged() {
    let (hocr, tsv, original, tables) = two_fragmented_hocr_tables();
    let (reconciled, bounds) = reconcile_fragmented_hocr_table_rows(
        &hocr,
        &original,
        &tables,
        &tsv,
        fragment_policy(&Default::default()),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(bounds.len(), tables.len());
    assert_ne!(
        serde_json::to_value(&original).unwrap(),
        serde_json::to_value(&reconciled).unwrap()
    );
    for count in [1, 3] {
        let mut document = original.clone();
        let mut changed_tables = tables.clone();
        let mismatched = vec![bounds[0]; count];
        let before = serde_json::to_value((&document, &changed_tables)).unwrap();
        assert!(!apply_fragmented_reconciliation(
            &mut document,
            &mut changed_tables,
            reconciled.clone(),
            mismatched
        ));
        assert_eq!(serde_json::to_value((&document, &changed_tables)).unwrap(), before);
    }
}

#[test]
fn fragmented_empty_bounds_keep_boxes_and_exact_bounds_update_every_table() {
    let (hocr, tsv, mut document, mut tables) = two_fragmented_hocr_tables();
    let (reconciled, bounds) = reconcile_fragmented_hocr_table_rows(
        &hocr,
        &document,
        &tables,
        &tsv,
        fragment_policy(&Default::default()),
        &Default::default(),
    )
    .unwrap();
    let original_tables = serde_json::to_value(&tables).unwrap();
    assert!(apply_fragmented_reconciliation(
        &mut document,
        &mut tables,
        reconciled.clone(),
        Vec::new()
    ));
    assert_eq!(serde_json::to_value(&tables).unwrap(), original_tables);
    assert_eq!(
        serde_json::to_value(&document).unwrap(),
        serde_json::to_value(&reconciled).unwrap()
    );
    assert!(apply_fragmented_reconciliation(
        &mut document,
        &mut tables,
        reconciled.clone(),
        bounds.clone()
    ));
    assert_eq!(
        serde_json::to_value(&document).unwrap(),
        serde_json::to_value(&reconciled).unwrap()
    );
    for (table, bbox) in tables.iter().zip(bounds) {
        assert_eq!(
            serde_json::to_value(table.bounding_box).unwrap(),
            serde_json::to_value(Some(bbox)).unwrap()
        );
    }
}

#[test]
fn fragmented_source_rejects_malformed_bbox_tokens() {
    let valid = r#"span class="ocrx_word" title="bbox 10 20 30 40">Item</span>"#;
    let mut budget = usize::MAX;
    let end = valid.find('>').unwrap();
    assert!(fragmented_hocr_word(valid, end, "span", &mut budget).is_some());
    for malformed in ["NaN", "inf", "-1", "1.5", "4294967296"] {
        let mut budget = usize::MAX;
        let source = valid.replace("bbox 10", &format!("bbox {malformed} 10"));
        assert!(
            fragmented_hocr_word(&source, source.find('>').unwrap(), "span", &mut budget).is_none(),
            "must reject malformed bbox token {malformed} rather than discard it"
        );
    }
}

#[test]
fn fragmented_source_rejects_duplicate_bbox_fields() {
    let mut budget = usize::MAX;
    let source = r#"span class="ocrx_word" title="bbox 10 20 30 40; bbox 10 20 30 40">Item</span>"#;
    assert!(fragmented_hocr_word(source, source.find('>').unwrap(), "span", &mut budget).is_none());
}

fn positioned_dot_leader_row() -> [HocrWord; 5] {
    [
        word_at(10, 30, 22, 10, "Signal"),
        word_at(35, 30, 24, 10, "timing"),
        word_at(65, 37, 29, 2, "........"),
        word_at(101, 30, 11, 10, "-64"),
        word_at(130, 30, 26, 10, "143.92"),
    ]
}

#[test]
fn dot_leader_requires_literal_anchors_and_isolated_geometry() {
    let cells = ["Signal timing", "-64", "143.92"].map(str::to_owned);
    let accepts = |words: &[HocrWord; 5]| row_has_only_extra_dot_leader(&words.each_ref(), &cells);
    let words = positioned_dot_leader_row();
    assert!(accepts(&words));
    for (index, token) in [(0, "Signals"), (2, ".."), (2, "...7"), (3, "64"), (4, "14392")] {
        let mut changed = words.clone();
        changed[index].text = token.to_owned();
        assert!(!accepts(&changed), "must preserve source token {token:?}");
    }
    for (left, top, width) in [(58, 37, 29), (65, 41, 29), (95, 37, 29), (65, 37, 8)] {
        let mut changed = words.clone();
        changed[2].left = left;
        changed[2].top = top;
        changed[2].width = width;
        assert!(!accepts(&changed), "must reject leader box {left},{top},{width}");
    }
}

#[test]
fn dot_leader_row_ownership_does_not_depend_on_label_letters_or_value_digits() {
    let accepts = |label: [&str; 2], values: [&str; 2]| {
        let cells = [
            format!("{} {}", label[0], label[1]),
            values[0].to_owned(),
            values[1].to_owned(),
        ];
        let mut words = positioned_dot_leader_row();
        words[0].text = label[0].to_owned();
        words[1].text = label[1].to_owned();
        words[3].text = values[0].to_owned();
        words[4].text = values[1].to_owned();
        let accepted = row_has_only_extra_dot_leader(&words.each_ref(), &cells);
        for (index, token) in [(0, "Other"), (3, "x"), (4, "3.2")] {
            let mut changed = words.clone();
            changed[index].text = token.to_owned();
            assert!(
                !row_has_only_extra_dot_leader(&changed.each_ref(), &cells),
                "source token changed to {token:?}"
            );
        }
        let mut overlapping = words.clone();
        overlapping[2].left = 95;
        assert!(!row_has_only_extra_dot_leader(&overlapping.each_ref(), &cells));
        accepted
    };
    assert!(accepts(["Signal", "timing"], ["-64", "143.92"]));
    assert!(accepts(["Item", "4(b)"], ["$1,010", "3.2%"]));
    assert!(accepts(["Total:", "net"], ["alpha", "beta"]));
}

#[test]
fn dot_leader_box_is_at_most_half_the_text_height() {
    let cells = ["Signal timing", "-64", "143.92"].map(str::to_owned);
    let mut words = positioned_dot_leader_row();
    words[2].top = 30;
    words[2].height = 5;
    assert!(
        row_has_only_extra_dot_leader(&words.each_ref(), &cells),
        "half of 10 is the limit"
    );
    words[2].height = 6;
    assert!(
        !row_has_only_extra_dot_leader(&words.each_ref(), &cells),
        "a box taller than half the text inside the shared band is a stroke, not leader dots"
    );
}

#[test]
fn hocr_reconcile_claims_a_symbol_dot_leader_row_only_for_a_standalone_leader_scan() {
    let fragments = [
        ("Inventory", 10, 10, 60, 10),
        ("Item", 10, 40, 22, 10),
        ("4(b)", 35, 40, 24, 10),
        ("........", 65, 47, 29, 2),
        ("$1,010", 101, 40, 30, 10),
        ("3.2%", 140, 40, 26, 10),
    ];
    let (hocr, tsv) = super::super::tests::word_table_markup(
        fragments
            .into_iter()
            .map(|(text, left, top, width, height)| (text, left, top, width, height, 1)),
    );
    let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document(&hocr);
    let table = OcrTable {
        cells: vec![vec!["Item 4(b)".into(), "$1,010".into(), "3.2%".into()]],
        markdown: String::new(),
        page_number: 1,
        bounding_box: Some(OcrTableBoundingBox {
            left: 10,
            top: 40,
            right: 170,
            bottom: 52,
        }),
    };
    let config = TesseractConfig::default();
    let limits = SecurityLimits::default();
    let reconcile = |allow_dot_leader| {
        reconcile_fragmented_hocr_table_rows(
            &hocr,
            &document,
            std::slice::from_ref(&table),
            &tsv,
            FragmentedTablePolicy {
                config: &config,
                allow_dot_leader,
            },
            &limits,
        )
    };
    let (reconciled, _) = reconcile(true).expect("the leader row is claimed whatever its label and value symbols");
    assert_eq!(reconciled.elements.len(), 1);
    assert_eq!(reconciled.elements[0].text, "Inventory");
    assert!(
        reconcile(false).is_none(),
        "only a standalone leader scan may skip the dots between the cells"
    );
}

#[test]
fn dot_leader_proof_refuses_to_choose_one_of_two_identical_physical_rows() {
    let first = positioned_dot_leader_row();
    let mut second = positioned_dot_leader_row();
    for word in &mut second {
        word.top += 25;
    }
    let rows = [first.iter().collect::<Vec<_>>(), second.iter().collect()];
    let cells = vec![["Signal timing", "-64", "143.92"].map(str::to_owned).to_vec()];
    assert!(unique_table_row_window(&rows[..1], &cells, false).is_none());
    assert!(unique_table_row_window(&rows[..1], &cells, true).is_some());
    assert!(unique_table_row_window(&rows, &cells, true).is_none());
}

#[test]
fn repeated_rows_beyond_alignment_budget_leave_hocr_unclaimed() {
    let mut words: Vec<_> = (0..64)
        .map(|row| word_at(10, row * 20, 12, 10, if row == 63 { "end" } else { "same" }))
        .collect();
    let mut cells = vec![vec!["same".to_string()]; 31];
    cells.push(vec!["end".to_string()]);
    let bbox = BoundingBox {
        x0: 0.0,
        y0: 0.0,
        x1: 25.0,
        y1: 1280.0,
    };
    assert!(prove_fragmented_table_rows(&mut words, bbox, &cells, 0.5, usize::MAX, false).is_none());
}

#[test]
fn split_hocr_table_rows_are_claimed_without_removing_heading() {
    let fragments = [
        ("Inventory", 10, 10),
        ("Apples", 10, 40),
        ("20", 70, 40),
        ("Pears", 10, 70),
        ("30", 70, 70),
    ];
    let (hocr, tsv) = fragmented_source_fixture(&fragments);
    let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document(&hocr);
    let table = OcrTable {
        cells: vec![vec!["Apples".into(), "20".into()], vec!["Pears".into(), "30".into()]],
        markdown: String::new(),
        page_number: 1,
        bounding_box: Some(OcrTableBoundingBox {
            left: 10,
            top: 10,
            right: 85,
            bottom: 80,
        }),
    };
    let config = TesseractConfig::default();
    let limits = SecurityLimits::default();
    let (reconciled, _) = reconcile_fragmented_hocr_table_rows(
        &hocr,
        &document,
        std::slice::from_ref(&table),
        &tsv,
        fragment_policy(&config),
        &limits,
    )
    .expect("the exact visual rows are split into hOCR fragments");
    assert_eq!(reconciled.elements.len(), 1);
    assert_eq!(reconciled.elements[0].text, "Inventory");
    assert_eq!(reconciled.elements[0].bbox, document.elements[0].bbox);
    assert!(!reconciled.ocr_text_claimed_by_tables);

    assert_fragmented_limits(&hocr, &tsv, &document, table, &config);
}

fn assert_fragmented_limits(
    hocr: &str,
    tsv: &str,
    document: &InternalDocument,
    mut table: OcrTable,
    config: &TesseractConfig,
) {
    let limits = SecurityLimits::default();
    for limits in [
        SecurityLimits {
            max_content_size: hocr.len(),
            ..limits.clone()
        },
        SecurityLimits {
            max_iterations: (hocr.len() + tsv.len()) * 4,
            ..limits.clone()
        },
        SecurityLimits {
            max_table_cells: 3,
            ..limits.clone()
        },
    ] {
        assert!(
            reconcile_fragmented_hocr_table_rows(
                hocr,
                document,
                std::slice::from_ref(&table),
                tsv,
                fragment_policy(config),
                &limits,
            )
            .is_none()
        );
    }

    table.cells[1][1].clear();
    assert!(
        reconcile_fragmented_hocr_table_rows(hocr, document, &[table], tsv, fragment_policy(config), &limits,)
            .is_none()
    );
}

fn fragmented_source_fixture(fragments: &[(&str, u32, u32)]) -> (String, String) {
    let mut hocr = r#"<div class="ocr_page" title="ppageno 0">"#.to_owned();
    let mut tsv = concat!(
        "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\t",
        "left\ttop\twidth\theight\tconf\ttext\n"
    )
    .to_owned();
    for (text, left, top) in fragments {
        hocr.push_str(r#"<p class="ocr_par"><span class="ocr_line">"#);
        hocr.push_str(&format!(
            r#"<span class="ocrx_word" title="bbox {left} {top} {} {}">{text}</span></span></p>"#,
            left + 15,
            top + 10
        ));
        tsv.push_str(&format!("5\t1\t1\t1\t1\t1\t{left}\t{top}\t15\t10\t90\t{text}\n"));
    }
    hocr.push_str("</div>");
    (hocr, tsv)
}

fn two_fragmented_hocr_tables() -> (String, String, InternalDocument, Vec<OcrTable>) {
    let fragments = [
        ("Inventory", 10, 10),
        ("Apples", 10, 40),
        ("20", 70, 40),
        ("Pears", 10, 70),
        ("30", 70, 70),
        ("Between", 10, 105),
        ("Peaches", 10, 140),
        ("60", 70, 140),
        ("Grapes", 10, 170),
        ("70", 70, 170),
        ("Footer", 10, 205),
    ];
    let (hocr, tsv) = fragmented_source_fixture(&fragments);
    let tables = [
        (10, 80, [["Apples", "20"], ["Pears", "30"]]),
        (105, 180, [["Peaches", "60"], ["Grapes", "70"]]),
    ]
    .into_iter()
    .map(|(top, bottom, cells)| OcrTable {
        cells: cells.into_iter().map(|row| row.map(str::to_owned).to_vec()).collect(),
        markdown: String::new(),
        page_number: 1,
        bounding_box: Some(OcrTableBoundingBox {
            left: 10,
            top,
            right: 85,
            bottom,
        }),
    })
    .collect();
    let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document(&hocr);
    (hocr, tsv, document, tables)
}

#[test]
fn fragmented_rows_of_two_tables_preserve_heading_interlude_and_footer() {
    let (hocr, tsv, document, tables) = two_fragmented_hocr_tables();
    let (reconciled, _) = reconcile_fragmented_hocr_table_rows(
        &hocr,
        &document,
        &tables,
        &tsv,
        fragment_policy(&Default::default()),
        &Default::default(),
    )
    .expect("both disjoint source grids prove their own fragmented words");
    assert_eq!(
        reconciled
            .elements
            .iter()
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>(),
        ["Inventory", "Between", "Footer"]
    );
    assert!(!reconciled.ocr_text_claimed_by_tables);
    assert_eq!(reconciled.elements[0].bbox, document.elements[0].bbox);
}

#[test]
fn fragmented_tables_allow_an_already_complete_sibling_region() {
    let (hocr, tsv, _, tables) = two_fragmented_hocr_tables();
    let hocr = hocr.replace("bbox 10 105 25 115", "bbox 10 90 25 100");
    let tsv = tsv.replace("\t10\t105\t15\t10\t", "\t10\t90\t15\t10\t");
    let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document(&hocr);
    assert!(table_retains_region_words(
        &document.elements,
        &ocr_table_bbox(&tables[1]).unwrap(),
        &tables[1].cells
    ));
    let (reconciled, _) = reconcile_fragmented_hocr_table_rows(
        &hocr,
        &document,
        &tables,
        &tsv,
        fragment_policy(&Default::default()),
        &Default::default(),
    )
    .expect("a complete sibling must not block exact removal of fragmented source rows");
    assert_eq!(
        reconciled
            .elements
            .iter()
            .map(|element| element.text.as_str())
            .collect::<Vec<_>>(),
        ["Inventory", "Between", "Footer"]
    );
}

#[test]
fn fragmented_multiple_table_claims_are_atomic_and_disjoint() {
    let (hocr, tsv, document, tables) = two_fragmented_hocr_tables();
    let prove = |tables: &[OcrTable], limits: &SecurityLimits| {
        reconcile_fragmented_hocr_table_rows(
            &hocr,
            &document,
            tables,
            &tsv,
            fragment_policy(&Default::default()),
            limits,
        )
    };
    assert!(
        prove(&tables, &Default::default()).is_some(),
        "valid combined source must reach the rejection guards"
    );
    assert!(prove(&tables[..1], &Default::default()).is_some());
    let mut wrong = tables.clone();
    wrong[1].cells[1][1] = "71".into();
    assert!(
        prove(&wrong, &Default::default()).is_none(),
        "never partially claim the first table"
    );
    let mut overlaps = tables.clone();
    overlaps[1].bounding_box.as_mut().unwrap().top = 60;
    assert!(prove(&overlaps, &Default::default()).is_none());
    assert!(prove(&[tables[0].clone(), tables[0].clone()], &Default::default()).is_none());
    let mut foreign_page = tables.clone();
    foreign_page[1].page_number = 2;
    assert!(prove(&foreign_page, &Default::default()).is_none());
    assert!(
        prove(
            &tables,
            &SecurityLimits {
                max_table_cells: 7,
                ..Default::default()
            }
        )
        .is_none()
    );
}

#[test]
fn fragmented_table_does_not_partially_claim_disagreeing_hocr_and_tsv() {
    let (hocr, tsv, document, tables) = two_fragmented_hocr_tables();
    assert_fragmented_source_control(&hocr, &tsv, &document, &tables);
    let hocr = hocr.replace(">70<", ">71<");
    let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document(&hocr);
    assert!(
        reconcile_fragmented_hocr_table_rows(
            &hocr,
            &document,
            &tables[1..],
            &tsv,
            fragment_policy(&Default::default()),
            &Default::default(),
        )
        .is_none(),
        "one mismatched source word must prevent the entire claim"
    );
}

#[test]
fn fragmented_tables_share_one_work_budget() {
    let (hocr, tsv, document, tables) = two_fragmented_hocr_tables();
    let prove = |tables: &[OcrTable], iterations| {
        reconcile_fragmented_hocr_table_rows(
            &hocr,
            &document,
            tables,
            &tsv,
            fragment_policy(&Default::default()),
            &SecurityLimits {
                max_iterations: iterations,
                ..Default::default()
            },
        )
    };
    let minimum = |table: &[OcrTable]| {
        let (mut low, mut high) = (0, SecurityLimits::default().max_iterations);
        assert!(prove(table, high).is_some());
        while low < high {
            let middle = low + (high - low) / 2;
            if prove(table, middle).is_some() {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        low
    };
    let single_budget = minimum(&tables[..1]).max(minimum(&tables[1..]));
    assert!(
        prove(&tables, SecurityLimits::default().max_iterations).is_some(),
        "combined proof must pass before testing exhaustion"
    );
    assert!(prove(&tables[..1], single_budget).is_some());
    assert!(prove(&tables[1..], single_budget).is_some());
    assert!(
        prove(&tables, single_budget).is_none(),
        "two claims cannot each reuse the full budget"
    );
}

#[test]
fn fragmented_table_refuses_duplicate_hocr_source_occurrences() {
    let (hocr, tsv, document, tables) = two_fragmented_hocr_tables();
    assert_fragmented_source_control(&hocr, &tsv, &document, &tables);
    let duplicate = r#"<p class="ocr_par"><span class="ocr_line"><span class="ocrx_word" title="bbox 70 40 85 50">20</span></span></p></div>"#;
    let hocr = hocr.replace("</div>", duplicate);
    let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document(&hocr);
    assert!(
        reconcile_fragmented_hocr_table_rows(
            &hocr,
            &document,
            &tables[..1],
            &tsv,
            fragment_policy(&Default::default()),
            &Default::default(),
        )
        .is_none(),
        "one TSV occurrence cannot certify duplicated hOCR source nodes"
    );
}

#[test]
fn fragmented_table_refuses_foreign_tsv_source_pages() {
    let (hocr, tsv, document, tables) = two_fragmented_hocr_tables();
    assert_fragmented_source_control(&hocr, &tsv, &document, &tables);
    let tsv = tsv.replace("5\t1\t", "5\t2\t");
    assert!(
        reconcile_fragmented_hocr_table_rows(
            &hocr,
            &document,
            &tables[..1],
            &tsv,
            fragment_policy(&Default::default()),
            &Default::default(),
        )
        .is_none()
    );
}

#[test]
fn fragmented_table_refuses_foreign_or_missing_hocr_page_ids() {
    let (hocr, tsv, document, tables) = two_fragmented_hocr_tables();
    assert_fragmented_source_control(&hocr, &tsv, &document, &tables);
    for hocr in [hocr.replace("ppageno 0", "ppageno 1"), hocr.replace("ppageno 0", "")] {
        let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document(&hocr);
        assert!(
            reconcile_fragmented_hocr_table_rows(
                &hocr,
                &document,
                &tables[..1],
                &tsv,
                fragment_policy(&Default::default()),
                &Default::default(),
            )
            .is_none()
        );
    }
}

#[test]
fn fragmented_table_maps_local_source_page_one_to_the_logical_pdf_page() {
    let (hocr, tsv, _, mut tables) = two_fragmented_hocr_tables();
    let document = crate::ocr::hocr_parser::parse_hocr_to_internal_document_with_page_offset(&hocr, None, 4);
    tables[0].page_number = 4;
    let config = TesseractConfig {
        page_number: 4,
        ..Default::default()
    };
    let (reconciled, _) = reconcile_fragmented_hocr_table_rows(
        &hocr,
        &document,
        &tables[..1],
        &tsv,
        fragment_policy(&config),
        &Default::default(),
    )
    .unwrap();
    assert!(reconciled.elements.iter().all(|element| element.page == Some(4)));
    assert_eq!(reconciled.elements[0].text, "Inventory");
}

fn assert_fragmented_source_control(hocr: &str, tsv: &str, document: &InternalDocument, tables: &[OcrTable]) {
    assert!(
        reconcile_fragmented_hocr_table_rows(
            hocr,
            document,
            &tables[..1],
            tsv,
            fragment_policy(&Default::default()),
            &Default::default(),
        )
        .is_some(),
        "unmodified source must pass before testing a source disagreement"
    );
}

fn word_at(left: u32, top: u32, width: u32, height: u32, text: &str) -> crate::table_core::HocrWord {
    crate::table_core::HocrWord {
        text: text.to_string(),
        left,
        top,
        width,
        height,
        confidence: 95.0,
    }
}
