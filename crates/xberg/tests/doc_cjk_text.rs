//! Regression test for #2073: Chinese text in a `.doc` is not re-decoded as cp1252.
//!
//! A UTF-16 piece in which over a quarter of the characters were CJK ideographs was re-read as
//! cp1252, one byte per character, so it came out garbled and only its first half was read. The
//! fixture, written with LibreOffice, holds a Swedish paragraph, a Chinese sentence and a closing
//! paragraph in one piece that is 29% ideographs; it came back as its first 58 of 115 characters.

#![cfg(feature = "office")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::ExtractionConfig;

#[test]
fn mixed_swedish_and_chinese_text_comes_back_whole() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/office/doc_cjk_text.doc");
    let bytes = std::fs::read(&path).expect("Word 97 CJK fixture must be present");

    let content = extract_bytes_document_blocking(&bytes, "application/msword", &ExtractionConfig::default())
        .expect("extraction must succeed")
        .content;

    assert_eq!(
        content,
        "Det här är ett svenskt stycke om föreningens årsmöte och arbete.\n\n\
         这是一个中文句子用于测试混合语言文档中的文本提取是否正确完成全部处理。\n\n\
         Sista stycket."
    );
}
