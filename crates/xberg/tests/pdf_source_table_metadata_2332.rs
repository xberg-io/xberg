#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
#[cfg(feature = "tokio-runtime")]
use helpers::extract_uri_document_blocking;
use sha2::{Digest, Sha256};
use xberg::types::document_structure::NodeContent;
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat, PdfConfig};

fn source_document(output_format: OutputFormat) -> ExtractedDocument {
    let bytes = include_bytes!("../../../fixtures/repros/table-snapshot-consistency/fixture/wildomar-p17-repro.pdf");
    assert_eq!(
        hex::encode(Sha256::digest(bytes)),
        "d21fb4bd4af939c647cd07adc4afe0344b06029879f1f347a81f0aee63776bbe"
    );
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format,
        include_document_structure: true,
        pdf_options: Some(PdfConfig {
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    extract_bytes_document_blocking(bytes, "application/pdf", &config).unwrap()
}

#[test]
#[ignore = "requires the pinned public ParseBench PDF via XBERG_2332_SIDE_TABLE_PDF"]
fn identical_ruled_tables_with_disjoint_source_bounds_reach_public_html() {
    let path = std::env::var_os("XBERG_2332_SIDE_TABLE_PDF").expect("set the pinned public PDF path");
    let bytes = std::fs::read(&path).expect("read public ParseBench fixture 7bc0fc8c64c67829dc21");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "bd5f64d7f3e329e55f6a000495169306b53e73cd99cf73476efb41e36471ef11"
    );
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "output_format": "html",
        "use_cache": false,
        "force_ocr": false,
        "disable_ocr": true,
        "enable_quality_processing": false,
        "ocr": {
            "backend": "tesseract",
            "language": ["eng"],
            "tesseract_config": {"language": ["eng"], "use_cache": false}
        },
        "concurrency": {"max_threads": 1}
    }))
    .expect("frozen native HTML extraction config");
    #[cfg(feature = "tokio-runtime")]
    let result = extract_uri_document_blocking(path, None, &config).expect("native HTML extraction");
    #[cfg(not(feature = "tokio-runtime"))]
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("native HTML extraction");
    let [first, second] = result.tables.as_slice() else {
        panic!("expected both Proposed and Current tables, got {}", result.tables.len());
    };
    let first_box = first.bounding_box.expect("Proposed table bounds");
    let second_box = second.bounding_box.expect("Current table bounds");
    assert!(
        first_box.x1 < second_box.x0,
        "Proposed is left of Current in the source"
    );
    for table in [first, second] {
        assert_eq!(table.cells.len(), 51);
        assert!(table.cells.iter().all(|row| row.len() == 8));
        assert_eq!(
            table.cells[0],
            ["Model Year", "Symbol Code", "BI", "PD", "UMBI", "MP", "COMP", "COLL"]
        );
    }
    assert_eq!(result.content.matches("<table").count(), 2);
    assert_eq!(
        result.content.matches("<th>").count(),
        16,
        "only the two column-label rows are headers"
    );
    for year in ["1998", "1999"] {
        assert_eq!(result.content.matches(&format!("<td>{year}</td>")).count(), 2);
    }
}

#[test]
fn source_headers_and_separate_table_records_reach_the_public_result() {
    let result = source_document(OutputFormat::Markdown);
    assert_eq!(result.tables.len(), 2, "the source page contains two physical tables");
    let upper = result
        .tables
        .iter()
        .find(|table| {
            table.cells.first().is_some_and(|row| {
                row.iter()
                    .map(String::as_str)
                    .eq(["Tests", "Test Method", "Requirements"])
            })
        })
        .expect("source header row survives");
    assert_eq!(
        upper.columns.as_deref(),
        Some(["Tests".into(), "Test Method".into(), "Requirements".into()].as_slice())
    );
    assert_eq!(upper.cells.len(), 5);
    assert!(upper.cells.iter().all(|row| row.len() == 3));
    assert_eq!(upper.cells[2], ["Sand Equivalent", "ASTM D2419", "60 Minimum"]);
    assert_eq!(upper.cells[4], ["Durability", "CTM 229", "55 Minimum"]);
    let lower = result.tables.iter().find(|table| !std::ptr::eq(*table, upper)).unwrap();
    assert!(lower.cells.iter().all(|row| row.len() == 3));
    assert!(
        lower
            .cells
            .iter()
            .any(|row| row == &["No. 8 (2.36 mm)", "60 - 90", "90 - 100"])
    );
    assert!(
        lower
            .cells
            .iter()
            .any(|row| row == &["No. 100 (150 µm)", "5 - 20", "15 - 30"])
    );
}

#[test]
fn source_header_bands_and_merged_label_reach_the_html_and_public_grid() {
    let result = source_document(OutputFormat::Html);
    assert_eq!(result.tables.len(), 2);
    assert_eq!(result.content.matches("<table").count(), 2);
    assert!(result.content.contains("<th rowspan=\"2\">Requirements</th>"));
    let document = result.document.as_ref().expect("requested public structure");
    let lower = document
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .find(|grid| {
            grid.cells
                .iter()
                .any(|cell| cell.row == 0 && cell.col == 0 && cell.content == "Requirements")
        })
        .expect("source lower table grid");
    assert_eq!((lower.rows, lower.cols), (11, 3));
    let label = lower.cells.iter().find(|cell| cell.row == 0 && cell.col == 0).unwrap();
    assert_eq!((label.row_span, label.col_span, label.is_header), (2, 1, true));
    assert_eq!(lower.cells.iter().filter(|cell| cell.is_header).count(), 5);
    assert!(
        lower
            .cells
            .iter()
            .filter(|cell| cell.is_header)
            .all(|cell| cell.row < 2)
    );
    assert!(!lower.cells.iter().any(|cell| cell.row == 1 && cell.col == 0));
    assert_eq!(lower.cells.iter().filter(|cell| cell.row >= 2).count(), 27);
}

#[test]
#[ignore = "requires the reference-only ParseBench source PDF outside Git"]
fn parsebench_repeated_sales_leaves_reach_html_and_grid_as_headers() {
    let config = ExtractionConfig {
        include_document_structure: true,
        ..parsebench_html_config()
    };
    let result = extract_bytes_document_blocking(&parsebench_source(), "application/pdf", &config).unwrap();
    assert_eq!(result.tables.len(), 2);
    assert_eq!(result.tables[0].cells.len(), 8);
    let upper_html = result.content.split_once("</table>").unwrap().0;
    for (label, count) in [("Product Line", 1), ("Revenue", 3), ("Units", 3)] {
        assert_eq!(upper_html.matches(&format!(">{label}</th>")).count(), count);
        assert_eq!(upper_html.matches(&format!(">{label}</td>")).count(), 0);
    }
    let bounds = result.tables[0].bounding_box.unwrap();
    assert_eq!(
        (bounds.x0, bounds.y0, bounds.x1, bounds.y1),
        (f64::from(88.2_f32), 518.0, f64::from(523.8_f32), 694.0)
    );
    let document = result.document.as_ref().unwrap();
    let grids: Vec<_> = document
        .nodes
        .iter()
        .filter_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .collect();
    assert_eq!(grids.len(), 2);
    assert_sales_header_grid(grids[0]);
    assert_eq!(upper_html.matches("<td>Enterprise Software</td>").count(), 1);
    assert_eq!(upper_html.matches("<td>Total</td>").count(), 1);
}

fn assert_sales_header_grid(grid: &xberg::TableGrid) {
    assert_eq!((grid.rows, grid.cols), (8, 7));
    assert_eq!(
        grid.cells.iter().filter(|cell| cell.row == 1 && cell.is_header).count(),
        7
    );
    assert!(
        grid.cells
            .iter()
            .filter(|cell| cell.row >= 2)
            .all(|cell| !cell.is_header)
    );
    let parents: Vec<_> = grid
        .cells
        .iter()
        .filter(|cell| cell.row == 0)
        .map(|cell| (cell.col, cell.row_span, cell.col_span))
        .collect();
    assert_eq!(parents, [(0, 1, 1), (1, 1, 2), (3, 1, 2), (5, 1, 2)]);
    assert!(grid.cells.iter().all(|cell| cell.row_span == 1));
}

#[test]
#[ignore = "requires the reference-only ParseBench source PDF outside Git"]
fn parsebench_survey_leaf_band_remains_a_header_below_group_labels() {
    let bytes = parsebench_source();
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &parsebench_html_config()).unwrap();
    let [_, survey] = result.tables.as_slice() else {
        panic!("expected exactly the two source tables");
    };
    assert_eq!(survey.cells.len(), 9);
    assert_eq!(
        survey.cells[1],
        ["Department", "2024", "2025", "Voluntary Turnover", "Avg Tenure (yrs)"]
    );
    assert_eq!(survey.cells[2], ["Engineering", "4.2", "4.5", "8.1%", "3.8"]);
    assert_eq!(survey.cells[8], ["Company-wide", "3.9", "4.1", "9.9%", "3.9"]);
    let survey_html = survey_html(&result.content);
    assert!(survey_html.contains("<th colspan=\"2\">Satisfaction Score</th>"));
    assert!(survey_html.contains("<th colspan=\"2\">Retention</th>"));
    assert!(survey_html.contains("<td>Engineering</td>"));
    assert!(survey_html.contains("<td>Company-wide</td>"));
    for label in &survey.cells[1] {
        assert_eq!(
            survey_html.matches(&format!(">{label}</th>")).count(),
            1,
            "source leaf label {label:?} must be a header in survey HTML:\n{survey_html}"
        );
    }
}

#[test]
#[ignore = "requires the reference-only ParseBench source PDF outside Git"]
fn parsebench_survey_bold_leaves_without_a_visible_separator_stay_body_cells() {
    let bytes = rewrite_parsebench_source(|source| {
        let separator = ".7 .7 .7 RG\nn 0 154 m 406.8 154 l S";
        assert_eq!(source.matches(separator).count(), 1);
        source.replace(separator, "1 1 1 RG\nn 0 154 m 406.8 154 l S")
    });
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &parsebench_html_config()).unwrap();
    let survey_html = survey_html(&result.content);
    assert!(survey_html.contains("<th colspan=\"2\">Satisfaction Score</th>"));
    assert!(survey_html.contains("<td>Department</td>"));
    assert!(survey_html.contains("<td>Company-wide</td>"));
}

#[test]
#[ignore = "requires the reference-only ParseBench source PDF outside Git"]
fn parsebench_survey_keeps_distinct_baselines_in_the_ruled_leaf_band() {
    let bytes = rewrite_parsebench_source(|source| {
        let label = "BT 1 0 0 1 194.504 163 Tm (2025) Tj T* ET";
        assert_eq!(source.matches(label).count(), 1);
        source.replace(
            label,
            &format!("{label}\nBT 1 0 0 1 126.104 157 Tm (North) Tj T* ET\nBT 1 0 0 1 194.504 157 Tm (South) Tj T* ET"),
        )
    });
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &parsebench_html_config()).unwrap();
    let [_, survey] = result.tables.as_slice() else {
        panic!("expected exactly the two source tables");
    };
    assert_eq!(survey.cells.len(), 10);
    assert_eq!(survey.cells[1][1..3], ["2024", "2025"]);
    assert_eq!(survey.cells[2][1..3], ["North", "South"]);
    assert!(survey_html(&result.content).contains("<td>Department</td>"));
}

fn rewrite_parsebench_source(rewrite: impl FnOnce(&str) -> String) -> Vec<u8> {
    let mut pdf = lopdf::Document::load_mem(&parsebench_source()).unwrap();
    let page = pdf.get_pages()[&1];
    let [stream] = <[_; 1]>::try_from(pdf.get_page_contents(page)).unwrap();
    let source = String::from_utf8(pdf.get_page_content(page)).unwrap();
    pdf.get_object_mut(stream)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .set_plain_content(rewrite(&source).into_bytes());
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn parsebench_source() -> Vec<u8> {
    let path = std::env::var_os("XBERG_PARSEBENCH_2332_SOURCE").expect("provide the pinned ParseBench source PDF");
    let bytes = std::fs::read(path).expect("read the ParseBench reference PDF");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "021397de9fc37d73c1412c47627e19dc3292dfa47d4b521610f653c4366254a4"
    );
    bytes
}

fn parsebench_html_config() -> ExtractionConfig {
    ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format: OutputFormat::Html,
        ..Default::default()
    }
}

fn survey_html(content: &str) -> &str {
    let tables = content.split("<table").skip(1).collect::<Vec<_>>();
    assert_eq!(tables.len(), 2);
    tables[1].split_once("</table>").expect("complete survey HTML table").0
}

#[cfg(all(feature = "layout-detection", feature = "ocr", feature = "tokio-runtime"))]
#[test]
#[ignore = "requires the pinned public ParseBench PDF and provisioned RT-DETR, TATR, and Tesseract models"]
fn forced_ocr_tatr_owns_words_without_exposing_ocr_elements() {
    let directory =
        std::env::var_os("XBERG_2332_PARSEBENCH_INPUTS").expect("set the pinned ParseBench input directory");
    let path = std::path::Path::new(&directory).join("7d6a55d4b9bf0316f7eb.pdf");
    assert_eq!(
        hex::encode(Sha256::digest(std::fs::read(&path).unwrap())),
        "d9b147dc465a8812de2600d707b67d36d3119e324949ed458d735afd8c33c722"
    );
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "output_format": "html",
        "use_cache": false,
        "force_ocr": true,
        "disable_ocr": false,
        "enable_quality_processing": false,
        "ocr": {
            "backend": "tesseract",
            "language": ["eng"],
            "tesseract_config": {"language": ["eng"], "use_cache": false}
        },
        "concurrency": {"max_threads": 1},
        "layout": {"strategy": "always", "table_model": "tatr", "acceleration": {"provider": "cpu"}}
    }))
    .unwrap();
    let result = extract_uri_document_blocking(&path, None, &config).unwrap();
    assert!(result.ocr_elements.as_ref().is_none_or(Vec::is_empty));
    let [table] = result.tables.as_slice() else {
        panic!("expected the forced OCR rate table, got {}", result.tables.len());
    };
    assert_eq!(table.cells.len(), 6);
    assert!(table.cells.iter().all(|row| row.len() == 12));
    assert_eq!(&table.cells[1][..4], &["2008", "$57,951", "1,022,724", "645,075"]);
    assert_eq!(table.cells[5][0], "2012 Total");
    assert_eq!(result.content.matches("<table").count(), 1);
}

#[test]
#[ignore = "requires the pinned public ParseBench PDF outside Git"]
fn insurance_wrapped_labels_render_as_headers_over_numeric_records() {
    let bytes = insurance_source();
    let config = parsebench_html_config();
    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    assert_insurance_tables(&result);
    let combined: Vec<Vec<String>> = (0..41)
        .map(|index| {
            result
                .tables
                .iter()
                .flat_map(|table| table.cells[index].iter().cloned())
                .collect()
        })
        .collect();

    let mut pdf = lopdf::Document::load_mem(&bytes).unwrap();
    let page = pdf.get_pages()[&1];
    let [stream] = <[_; 1]>::try_from(pdf.get_page_contents(page)).unwrap();
    let source = String::from_utf8(pdf.get_page_content(page)).unwrap();
    let key_paint = "0 0.5 0 rg\n";
    assert_eq!(source.matches(key_paint).count(), 120);
    pdf.get_object_mut(stream)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .set_plain_content(source.replace(key_paint, "0 g\n").into_bytes());
    let mut neutral = Vec::new();
    pdf.save_to(&mut neutral).unwrap();
    let control = extract_bytes_document_blocking(&neutral, "application/pdf", &config).unwrap();
    let [neutral_table] = control.tables.as_slice() else {
        panic!("expected one native grid after recoloring keys");
    };
    assert_eq!(neutral_table.cells, combined);
    assert_eq!(control.content.matches("<th>").count(), 0);
    assert_eq!(control.content.matches("<td>").count(), 41 * 12);
}

fn insurance_source() -> Vec<u8> {
    let directory =
        std::env::var_os("XBERG_2332_PARSEBENCH_INPUTS").expect("set the pinned ParseBench input directory");
    let path = std::path::Path::new(&directory).join("f48db8574acac2e2744d.pdf");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "0085e250b078694c7a6887245e52108e511401b90db99052fe1ee2b4c0d099a3"
    );
    bytes
}

fn assert_insurance_tables(result: &ExtractedDocument) {
    assert_eq!(result.tables.len(), 3, "source prints three independent groups");
    for (table, code) in result.tables.iter().zip(["3400", "4112", "4635"]) {
        assert_eq!(table.cells.len(), 41);
        assert!(table.cells.iter().all(|row| row.len() == 4));
        assert_eq!(table.cells[0], ["CLASS CODE", "RATE", "MIN. PREM.", "HAZ L/C CODE"]);
        assert_eq!(table.cells[1][0], code);
    }
    assert!(
        result
            .tables
            .windows(2)
            .all(|pair| pair[0].bounding_box.unwrap().x1 < pair[1].bounding_box.unwrap().x0)
    );
    assert_eq!(result.content.matches("<table").count(), 3);
    assert_eq!(result.content.matches("<th>").count(), 12);
    assert_eq!(result.content.matches("<td>").count(), 40 * 12);
}

#[cfg(feature = "layout-detection")]
#[test]
#[ignore = "requires the pinned public PDF and layout models"]
fn insurance_layout_retains_caption_text_before_three_source_tables() {
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "output_format": "html", "use_cache": false, "disable_ocr": true,
        "enable_quality_processing": false, "concurrency": {"max_threads": 1},
        "layout": {"strategy": "always", "table_model": "tatr", "acceleration": {"provider": "cpu"}}
    }))
    .unwrap();
    let result = extract_bytes_document_blocking(&insurance_source(), "application/pdf", &config).unwrap();
    assert_insurance_tables(&result);
    let (prose, tables) = result.content.split_once("<table").unwrap();
    for token in ["WORKERS", "OBI NATIONAL", "STANDARD", "CALIFORNIA", "EFFECTIVE DATE"] {
        assert_eq!(prose.matches(token).count(), 1, "{token}: {prose}");
        assert!(!tables.contains(token));
    }
}

#[test]
#[ignore = "requires the pinned public ParseBench PDF outside Git"]
fn insurance_comparison_with_a_shared_rule_keeps_one_table() {
    let mut pdf = lopdf::Document::load_mem(&insurance_source()).unwrap();
    let page = pdf.get_pages()[&1];
    let [stream] = <[_; 1]>::try_from(pdf.get_page_contents(page)).unwrap();
    let mut content = pdf.get_page_content(page);
    content.extend_from_slice(b"\nq 0 G 0.8 w 43 650 m 565 650 l S Q\n");
    pdf.get_object_mut(stream)
        .unwrap()
        .as_stream_mut()
        .unwrap()
        .set_plain_content(content);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();

    let result = extract_bytes_document_blocking(&bytes, "application/pdf", &parsebench_html_config()).unwrap();
    let [table] = result.tables.as_slice() else {
        panic!(
            "a shared rule joins the comparison table, got {} tables",
            result.tables.len()
        );
    };
    assert_eq!(table.cells.len(), 41);
    assert!(table.cells.iter().all(|row| row.len() == 12));
    assert_eq!(table.cells[1][..4], ["3400", "9.97", "1000", "2"]);
    assert_eq!(result.content.matches("<table").count(), 1);
}

#[test]
#[ignore = "requires the pinned public ParseBench PDF outside Git"]
fn insurance_image_rule_only_joins_tables_when_it_crosses_a_gutter() {
    use lopdf::dictionary;

    for (width, in_form, table_count) in [(10, false, 3), (522, false, 1), (522, true, 1)] {
        let mut pdf = lopdf::Document::load_mem(&insurance_source()).unwrap();
        let page = pdf.get_pages()[&1];
        let image = pdf.add_object(lopdf::Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => 1, "Height" => 1,
                "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
            },
            vec![0],
        ));
        let rule = if in_form {
            pdf.add_object(lopdf::Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Form", "BBox" => lopdf::Object::Array(vec![0.into(), 0.into(), 1.into(), 1.into()]),
                    "Resources" => dictionary! { "XObject" => dictionary! { "Image" => image } },
                },
                b"/Image Do".to_vec(),
            ))
        } else {
            image
        };
        pdf.add_xobject(page, "GutterRule", rule).unwrap();
        let [stream] = <[_; 1]>::try_from(pdf.get_page_contents(page)).unwrap();
        let mut content = pdf.get_page_content(page);
        content.extend_from_slice(format!("\nq {width} 0 0 1 43 650 cm /GutterRule Do Q\n").as_bytes());
        pdf.get_object_mut(stream)
            .unwrap()
            .as_stream_mut()
            .unwrap()
            .set_plain_content(content);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();

        let result = extract_bytes_document_blocking(&bytes, "application/pdf", &parsebench_html_config()).unwrap();
        assert_eq!(result.tables.len(), table_count, "image width {width}, Form {in_form}");
        if table_count == 3 {
            assert_insurance_tables(&result);
        } else {
            let table = &result.tables[0];
            assert_eq!(table.cells.len(), 41);
            assert!(table.cells.iter().all(|row| row.len() == 12));
            assert_eq!(result.content.matches("<table").count(), 1);
        }
    }
}
