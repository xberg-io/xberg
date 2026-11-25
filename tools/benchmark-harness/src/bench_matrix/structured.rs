//! Pinned fixture lists, digests and document metadata for the structured cohorts.

pub(super) const NATIVE_STRUCTURED_COHORT: &str = "native-pdf-structured-v1";
pub(super) const NATIVE_STRUCTURED_MANIFEST_NAME: &str = "native-pdf-structured-v1";
pub(super) const NATIVE_STRUCTURED_MANIFEST_BLAKE3: &str =
    "0b35c8b97240d8e512d388ff18a7dd0ace11baf64dcb80965427f37f135a515f";
pub(super) const NATIVE_STRUCTURED_BATCH_SIZE: usize = 8;
pub(super) const NATIVE_STRUCTURED_FIXTURES: &[&str] = &[
    "pdf_tiny_memo.json",
    "pdf_tables.json",
    "pdf_embedded.json",
    "pdf_google_docs.json",
    "pdf/681693.json",
    "pdf/ft_ACN_2009_page_102_t0.json",
    "pdf/pb_fqr-retail-blackrock-global-allocation-fund-inc_page4.json",
    "pdf/pb_sample_page_16_page1.json",
    "pdf/1004.1028.json",
    "pdf/1509.07624.json",
    "pdf/2003.05541.json",
    "pdf/2305.09790.json",
    "pdf/2212.11694.json",
    "pdf/2310.02163.json",
    "pdf/2001.09113.json",
    "pdf/1602.03238.json",
    "pdf/1710.02091.json",
    "pdf/2026-13845.json",
    "pdf/2026-14578.json",
    "pdf/2207.09253.json",
    "pdf/2301.08131.json",
    "pdf/2211.13451.json",
    "pdf/198320410.json",
    "pdf/3562713.json",
    "pdf/283451672.json",
    "pdf/231425210.json",
    "pdf/pb_fqr-retail-blackrock-global-allocation-fund-inc_page8.json",
    "pdf/pb_FBLB-134215544_page112.json",
    "pdf/pb_SERFF_TX_random_pages_1_page1075.json",
    "pdf_medium_tables.json",
    "pdf_large_tables.json",
    "pdf/pdfa_001.json",
];
pub(super) const NATIVE_STRUCTURED_DOCUMENT_STEMS: &[&str] = &[
    "fake_memo",
    "tiny",
    "embedded_images_tables",
    "google_doc_document",
    "681693",
    "ft_ACN_2009_page_102_t0",
    "pb_fqr-retail-blackrock-global-allocation-fund-inc_page4",
    "pb_sample_page_16_page1",
    "1004.1028",
    "1509.07624",
    "2003.05541",
    "2305.09790",
    "2212.11694",
    "2310.02163",
    "2001.09113",
    "1602.03238",
    "1710.02091",
    "2026-13845",
    "2026-14578",
    "2207.09253",
    "2301.08131",
    "2211.13451",
    "198320410",
    "3562713",
    "283451672",
    "231425210",
    "pb_fqr-retail-blackrock-global-allocation-fund-inc_page8",
    "pb_FBLB-134215544_page112",
    "pb_SERFF_TX_random_pages_1_page1075",
    "medium",
    "large",
    "pdfa_001",
];
pub(super) const NATIVE_STRUCTURED_DOCUMENT_EXTENSIONS: &[&str] = &[
    "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf",
    "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf",
];

pub(super) const OCR_STRUCTURED_COHORT: &str = "ocr-pdf-structured-v1";
pub(super) const OCR_STRUCTURED_MANIFEST_NAME: &str = "ocr-pdf-structured-v1";
pub(super) const OCR_STRUCTURED_MANIFEST_BLAKE3: &str =
    "94fea713f0b650c138020e2beb4c266e81fae5e1f8212c961686d1d791ec8562";
pub(super) const OCR_STRUCTURED_BATCH_SIZE: usize = 4;
pub(super) const OCR_STRUCTURED_FIXTURES: &[&str] = &[
    "pdf_non_searchable.json",
    "pdf_scanned_ocr.json",
    "pdf_image_only_german.json",
    "docling_scanned.json",
    "multi_page_scanned.json",
    "nougat_001_scanned.json",
    "nougat_002_scanned.json",
    "nougat_005_scanned.json",
    "nougat_007_scanned.json",
    "nougat_009_scanned.json",
    "nougat_010_scanned.json",
    "nougat_012_scanned.json",
];
pub(super) const OCR_STRUCTURED_DOCUMENT_STEMS: &[&str] = &[
    "non_searchable",
    "scanned",
    "image_only_german_pdf",
    "docling_scanned",
    "multi_page_scanned",
    "nougat_001_scanned",
    "nougat_002_scanned",
    "nougat_005_scanned",
    "nougat_007_scanned",
    "nougat_009_scanned",
    "nougat_010_scanned",
    "nougat_012_scanned",
];
pub(super) const OCR_STRUCTURED_DOCUMENT_EXTENSIONS: &[&str] = &[
    "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf", "pdf",
];

pub(super) const IMAGES_STRUCTURED_COHORT: &str = "ocr-images-structured-v2";
pub(super) const IMAGES_STRUCTURED_MANIFEST_NAME: &str = "ocr-images-structured-v2";
pub(super) const IMAGES_STRUCTURED_MANIFEST_BLAKE3: &str =
    "9148812c7f1fbe0f7877c259e3ee2ea74a9466639f068ca8de389acb38a87d2c";
pub(super) const IMAGES_STRUCTURED_BATCH_SIZE: usize = 5;
pub(super) const IMAGES_STRUCTURED_FIXTURES: &[&str] = &[
    "doclaynet_page_01.json",
    "doclaynet_page_02.json",
    "image_table.json",
    "balance_sheet_1.json",
    "financial_table_1.json",
];
pub(super) const IMAGES_STRUCTURED_DOCUMENT_STEMS: &[&str] = &[
    "doclaynet_page_01",
    "doclaynet_page_02",
    "complex_document",
    "balance_sheet_1",
    "financial_table_1",
];
pub(super) const IMAGES_STRUCTURED_DOCUMENT_EXTENSIONS: &[&str] = &["jpg", "jpg", "png", "png", "png"];
