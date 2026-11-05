#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::core::config::{PageConfig, PdfBackend, PdfConfig};
use xberg::{ExtractionConfig, OutputFormat};

const COMMON_RECORDS: [&str; 7] = [
    "Brilliant Hat Limited",
    "Blue Hat Interactive Entertainment Technology Limited",
    "Xiamen Duwei Consulting Management Co., Ltd.",
    "Fresh Joy Entertainment Ltd",
    "Fujian Blue Hat Interactive Entertainment Technology Ltd.",
    "Hunan Engaomei Animation Culture Development Co., Ltd.",
    "Shenyang Qimengxing Trading Co.Ltd.(China)",
];

#[test]
fn should_keep_company_records_in_source_order_across_painted_spacers() {
    let path = std::env::var_os("XBERG_2376_NOUGAT_SOURCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| helpers::get_test_file_path("pdf/nougat_014.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "05015d789a281dc7737a853d8bf468f0bd3002cf899cebd428b0f4d14d4e6817"
    );
    let config = ExtractionConfig {
        use_cache: false,
        disable_ocr: true,
        include_document_structure: true,
        output_format: OutputFormat::Markdown,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    assert_eq!(result.pages.as_ref().unwrap().len(), 105);
    for (page, tail) in [
        (
            10,
            &[
                "Fuzhou Csfctech Co., Ltd.",
                "Fuzhou UC71 Co., Ltd.",
                "Pingxiang Blue Hat Technology Co. Ltd.",
            ][..],
        ),
        (
            26,
            &["Shenyang Qimengxing Trading Co., Ltd.", "Fuzhou Csfctech Co., Ltd."][..],
        ),
    ] {
        let tables = result
            .tables
            .iter()
            .filter(|table| table.page_number == page)
            .collect::<Vec<_>>();
        let names = tables
            .iter()
            .flat_map(|table| table.cells.iter())
            .filter_map(|row| row.first())
            .map(|cell| cell.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|name| !name.is_empty() && name != "Name")
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            COMMON_RECORDS
                .into_iter()
                .chain(tail.iter().copied())
                .collect::<Vec<_>>(),
            "source-owned company records on page {page}"
        );
        assert_eq!(tables.len(), 1, "one painted company table on page {page}");
        let rows = &tables[0].cells;
        assert!(
            rows.iter()
                .all(|row| row.len() == 5 && row[1].is_empty() && row[3].is_empty())
        );
        let wrapped_ownership = rows[0][4].split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            wrapped_ownership,
            "100% owned by Blue Hat Interactive Entertainment Technology"
        );
        let last_ownership = rows.last().unwrap()[4].split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            last_ownership,
            if page == 10 {
                ""
            } else {
                "51% controlled by Fresh Joy Entertainment Ltd via VIE"
            }
        );
    }
    let cover_tables = result
        .tables
        .iter()
        .filter(|table| table.page_number == 1)
        .collect::<Vec<_>>();
    assert_eq!(cover_tables.len(), 2, "the fee and registration tables coexist");
    let registration = cover_tables
        .iter()
        .find(|table| table.cells[0][0] == "Cayman Islands")
        .expect("independent registration table");
    assert_eq!(
        registration.cells,
        [
            ["Cayman Islands", "3942", "Not Applicable"],
            [
                "(State or other jurisdiction of",
                "(Primary Standard Industrial",
                "(I.R.S. Employer",
            ],
            [
                "incorporation or organization)",
                "Classification Code Number)",
                "Identification Number)",
            ],
        ]
    );
    let fee_table = cover_tables
        .iter()
        .find(|table| table.cells[0][0] == "Ordinary Shares (3)")
        .expect("fee table");
    assert_eq!(fee_table.cells.len(), 2);
    assert_eq!(
        fee_table.cells[0],
        ["Ordinary Shares (3)", "$", "3,975,909", "$", "433.77"]
    );
    assert_eq!(fee_table.cells[1], ["Total", "$", "3,975,909", "$", "433.77"]);
    let document = result.document.as_ref().expect("document structure");
    let fee_header = "Proposed maximum aggregate Amount of Title of each class of offering registration securities to be registered price(1) fee(2)";
    assert_eq!(
        document
            .nodes
            .iter()
            .filter(|node| node.page == Some(1) && node.content.text() == Some(fee_header))
            .count(),
        1,
        "fee headings outside the recovered rows remain exact"
    );
    let duplicates = document
        .nodes
        .iter()
        .filter_map(|node| {
            let (page, bbox, text) = (node.page?, node.bbox?, node.content.text()?);
            let duplicate = match page {
                1 => text.split_whitespace().last() == Some("Total") && bbox.y0 < 62.0,
                10 => text == "T" && (684.39..684.40).contains(&bbox.y0),
                26 => matches!(text, "t T" | "ure" | "o.,") && bbox.y0 < 705.0,
                _ => false,
            };
            duplicate.then_some((page, text, bbox))
        })
        .collect::<Vec<_>>();
    assert!(
        duplicates.is_empty(),
        "recovered cells own these source occurrences: {duplicates:?}"
    );
    let intro = "The charts below summarize our corporate legal structure and identify our subsidiaries, our VIE and its subsidiaries:";
    for (page, labels) in [
        (10, &["Name Background Ownership"][..]),
        (26, &["Name", "Background Ownership"][..]),
    ] {
        let order = document
            .nodes
            .iter()
            .filter(|node| node.page == Some(page))
            .map(|node| {
                if matches!(&node.content, xberg::types::NodeContent::Table { .. }) {
                    "<company table>"
                } else {
                    node.content.text().expect("retained source paragraph")
                }
            })
            .collect::<Vec<_>>();
        let expected = std::iter::once(intro)
            .chain(labels.iter().copied())
            .chain(std::iter::once("<company table>"))
            .collect::<Vec<_>>();
        assert_eq!(order, expected, "ordered title, source labels and table on page {page}");
    }
}
