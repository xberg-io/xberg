pub mod blank_detection;
pub mod structured;
pub mod text;
pub mod transform;

#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
pub mod image;

/// Capacity estimation utilities for string pre-allocation.
///
/// This module provides functions to estimate the capacity needed for string buffers
/// based on input file sizes and content types. This enables pre-allocation, reducing
/// reallocation cycles during string building operations.
pub mod capacity;

#[cfg(feature = "archives")]
pub mod archive;

#[cfg(feature = "email")]
pub mod email;

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
pub mod excel;

#[cfg(feature = "html")]
pub mod html;

#[cfg(feature = "office")]
pub mod doc;

#[cfg(feature = "office")]
pub mod docx;

#[cfg(feature = "office")]
pub mod office_metadata;

#[cfg(feature = "office")]
pub mod ooxml_constants;

#[cfg(feature = "office")]
pub mod image_format;

#[cfg(all(feature = "ocr", feature = "tokio-runtime"))]
pub mod image_ocr;

#[cfg(feature = "office")]
pub mod ppt;

#[cfg(feature = "office")]
pub mod pptx;

#[cfg(feature = "xml")]
pub mod xml;

#[cfg(any(feature = "office", feature = "html", feature = "xml"))]
pub mod markdown;

pub use structured::{JsonExtractionConfig, StructuredDataResult, parse_json, parse_toml, parse_yaml};
pub use text::parse_text;
pub use transform::{
    ListItemMetadata, ListType, detect_list_items, generate_element_id, transform_extraction_result_to_elements,
    transform_to_document_structure,
};

#[cfg(any(feature = "ocr", feature = "ocr-wasm"))]
pub use image::{ImageMetadata, extract_image_metadata};

#[cfg(feature = "archives")]
pub use archive::{
    ArchiveEntry, ArchiveMetadata, extract_7z_metadata, extract_7z_text_content, extract_tar_metadata,
    extract_tar_text_content, extract_zip_metadata, extract_zip_text_content,
};

#[cfg(feature = "email")]
pub use email::{build_email_text_output, extract_email_content, parse_eml_content, parse_msg_content};

#[cfg(any(feature = "excel", feature = "excel-wasm"))]
pub use excel::{excel_to_markdown, read_excel_bytes, read_excel_file};

#[cfg(feature = "html")]
pub use html::convert_html_to_markdown;

#[cfg(feature = "office")]
pub use doc::extract_doc_text;

#[cfg(feature = "office")]
pub use ppt::extract_ppt_text;

#[cfg(feature = "office")]
pub use office_metadata::{
    CoreProperties, CustomProperties, DocxAppProperties, OdtProperties, PptxAppProperties, XlsxAppProperties,
    extract_core_properties, extract_custom_properties, extract_docx_app_properties, extract_odt_properties,
    extract_pptx_app_properties, extract_xlsx_app_properties,
};

#[cfg(feature = "office")]
pub use pptx::{extract_pptx_from_bytes, extract_pptx_from_path};

#[cfg(feature = "xml")]
pub use xml::parse_xml;

#[cfg(any(feature = "office", feature = "html", feature = "xml"))]
pub use markdown::cells_to_markdown;
#[cfg(any(feature = "office", feature = "html", feature = "xml"))]
pub use markdown::cells_to_text;

pub use capacity::{
    estimate_content_capacity, estimate_html_markdown_capacity, estimate_presentation_capacity,
    estimate_spreadsheet_capacity, estimate_table_markdown_capacity,
};
