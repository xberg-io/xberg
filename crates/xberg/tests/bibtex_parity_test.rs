#![cfg(feature = "office")]
//! Comprehensive test for BibTeX extractor parity with Pandoc

use xberg::ExtractInput;
use xberg::core::config::ExtractionConfig;
use xberg::extractors::BibtexExtractor;
use xberg::plugins::DocumentExtractor;
use xberg::types::metadata::FormatMetadata;

mod helpers;
use helpers::get_test_file_path;

#[tokio::test]
async fn test_all_entry_types() {
    let extractor = BibtexExtractor;

    let test_cases = vec![
        (
            "@article{test, author={John Doe}, title={Test}, journal={Journal}, year={2023}}",
            "article",
        ),
        (
            "@book{test, author={John Doe}, title={Test}, publisher={Publisher}, year={2023}}",
            "book",
        ),
        (
            "@inproceedings{test, author={John Doe}, title={Test}, booktitle={Conference}, year={2023}}",
            "inproceedings",
        ),
        (
            "@phdthesis{test, author={John Doe}, title={Test}, school={University}, year={2023}}",
            "phdthesis",
        ),
        (
            "@mastersthesis{test, author={John Doe}, title={Test}, school={University}, year={2023}}",
            "mastersthesis",
        ),
        (
            "@techreport{test, author={John Doe}, title={Test}, institution={Institute}, year={2023}}",
            "techreport",
        ),
        ("@manual{test, title={Test Manual}, year={2023}}", "manual"),
        ("@misc{test, author={John Doe}, title={Test}, year={2023}}", "misc"),
        (
            "@unpublished{test, author={John Doe}, title={Test}, note={Unpublished}, year={2023}}",
            "unpublished",
        ),
        (
            "@incollection{test, author={John Doe}, title={Test}, booktitle={Book}, publisher={Pub}, year={2023}}",
            "incollection",
        ),
        (
            "@inbook{test, author={John Doe}, title={Test}, chapter={5}, publisher={Pub}, year={2023}}",
            "inbook",
        ),
        (
            "@proceedings{test, title={Conference Proceedings}, year={2023}}",
            "proceedings",
        ),
        ("@booklet{test, title={Booklet}, year={2023}}", "booklet"),
    ];

    for (bibtex_content, expected_type) in test_cases {
        let config = ExtractionConfig::default();
        let input = ExtractInput::from_bytes(bibtex_content.as_bytes().to_vec(), "application/x-bibtex", None);
        let result = extractor
            .extract(input, &config)
            .await;

        assert!(result.is_ok(), "Failed to parse {} entry", expected_type);
        let result = result.expect("Operation failed");

        if let Some(FormatMetadata::Bibtex(ref bibtex)) = result.metadata.format
            && let Some(ref entry_types) = bibtex.entry_types
        {
            assert!(!entry_types.is_empty(), "Entry types should not be empty");
            println!("Entry type '{}' extracted successfully", expected_type);
        }
    }
}

#[tokio::test]
async fn test_all_common_fields() {
    let extractor = BibtexExtractor;

    let bibtex_content = r#"
@article{comprehensive,
    author = {Smith, John and Doe, Jane},
    title = {Comprehensive Test},
    journal = {Test Journal},
    year = {2023},
    volume = {42},
    number = {3},
    pages = {123--145},
    month = {June},
    doi = {10.1234/test.001},
    url = {https://example.com},
    issn = {1234-5678},
    isbn = {978-0-12-345678-9},
    abstract = {This is an abstract},
    keywords = {test, bibtex},
    note = {Additional notes},
    publisher = {Test Publisher},
    address = {Test City},
    edition = {2nd},
    editor = {Editor Name},
    series = {Test Series},
    organization = {Test Org},
    institution = {Test Institute},
    school = {Test School},
    howpublished = {Online},
    type = {Research Article},
    chapter = {5},
    booktitle = {Book Title}
}
"#;

    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bibtex_content.as_bytes().to_vec(), "application/x-bibtex", None);
    let result = extractor
        .extract(input, &config)
        .await;

    assert!(result.is_ok());
    let result = result.expect("Operation failed");

    let content = &result.content;

    let expected_fields = vec![
        "author",
        "title",
        "journal",
        "year",
        "volume",
        "number",
        "pages",
        "month",
        "doi",
        "url",
        "issn",
        "isbn",
        "abstract",
        "keywords",
        "note",
        "publisher",
        "address",
        "edition",
        "editor",
        "series",
        "organization",
        "institution",
        "school",
        "howpublished",
        "type",
        "chapter",
        "booktitle",
    ];

    let num_fields = expected_fields.len();
    for field in expected_fields {
        assert!(content.contains(field), "Field '{}' should be present in output", field);
    }

    println!("All {} fields were extracted", num_fields);
}

#[tokio::test]
async fn test_author_parsing() {
    let extractor = BibtexExtractor;

    let test_cases = vec![
        ("author = {John Doe}", vec!["John Doe"]),
        ("author = {John Doe and Jane Smith}", vec!["John Doe", "Jane Smith"]),
        ("author = {Smith, John and Doe, Jane}", vec!["Smith, John", "Doe, Jane"]),
        (
            "author = {John Doe and Jane Smith and Bob Jones}",
            vec!["John Doe", "Jane Smith", "Bob Jones"],
        ),
        ("author = {van der Berg, Hans}", vec!["van der Berg, Hans"]),
        ("author = {Smith, Jr., John}", vec!["Smith, Jr., John"]),
    ];

    for (author_field, expected_authors) in test_cases {
        let bibtex = format!("@article{{test, {}, title={{Test}}, year={{2023}}}}", author_field);

        let config = ExtractionConfig::default();
        let input = ExtractInput::from_bytes(bibtex.as_bytes().to_vec(), "application/x-bibtex", None);
        let result = extractor
            .extract(input, &config)
            .await;

        assert!(result.is_ok());
        let result = result.expect("Operation failed");

        if let Some(authors) = &result.metadata.authors {
            for expected_author in &expected_authors {
                let found = authors.iter().any(|a| a.contains(expected_author));
                assert!(
                    found,
                    "Expected author '{}' not found in {:?}",
                    expected_author, authors
                );
            }
        }
    }
}

#[tokio::test]
async fn test_special_characters() {
    let extractor = BibtexExtractor;

    let bibtex_content = r#"
@article{special,
    author = {M{\"u}ller, Hans and Sch{\"o}n, Anna and Garc{\'\i}a, Jos{\'e}},
    title = {Special Characters in {BibTeX}: {\"O}berblick},
    journal = {Test Journal},
    year = {2022}
}
"#;

    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bibtex_content.as_bytes().to_vec(), "application/x-bibtex", None);
    let result = extractor
        .extract(input, &config)
        .await;

    assert!(result.is_ok());
    let result = result.expect("Operation failed");

    if let Some(FormatMetadata::Bibtex(ref bibtex)) = result.metadata.format {
        assert_eq!(bibtex.entry_count, 1);
    } else {
        panic!("Expected BibtexMetadata in format");
    }

    if let Some(authors) = &result.metadata.authors {
        assert!(authors.len() >= 3, "Should have 3 authors");
    }
}

#[tokio::test]
async fn test_year_range_extraction() {
    let extractor = BibtexExtractor;

    let bibtex_content = r#"
@article{old, author={A}, title={Old}, year={1990}}
@article{mid, author={B}, title={Mid}, year={2005}}
@article{new, author={C}, title={New}, year={2023}}
"#;

    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bibtex_content.as_bytes().to_vec(), "application/x-bibtex", None);
    let result = extractor
        .extract(input, &config)
        .await;

    assert!(result.is_ok());
    let result = result.expect("Operation failed");

    if let Some(FormatMetadata::Bibtex(ref bibtex)) = result.metadata.format {
        let year_range = bibtex.year_range.as_ref().expect("Year range not extracted");
        assert_eq!(year_range.min, Some(1990));
        assert_eq!(year_range.max, Some(2023));
        assert_eq!(year_range.years.len(), 3, "Should have 3 unique years");
    } else {
        panic!("Expected BibtexMetadata in format");
    }
}

#[tokio::test]
async fn test_citation_keys_extraction() {
    let extractor = BibtexExtractor;

    let bibtex_content = r#"
@article{key1, author={A}, title={T1}, year={2023}}
@book{key2, author={B}, title={T2}, year={2023}}
@inproceedings{key3, author={C}, title={T3}, year={2023}}
"#;

    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bibtex_content.as_bytes().to_vec(), "application/x-bibtex", None);
    let result = extractor
        .extract(input, &config)
        .await;

    assert!(result.is_ok());
    let result = result.expect("Operation failed");

    if let Some(FormatMetadata::Bibtex(ref bibtex)) = result.metadata.format {
        assert_eq!(bibtex.citation_keys.len(), 3);

        let expected_keys = vec!["key1", "key2", "key3"];
        for expected_key in expected_keys {
            let found = bibtex.citation_keys.iter().any(|k| k == expected_key);
            assert!(found, "Citation key '{}' not found", expected_key);
        }
    } else {
        panic!("Expected BibtexMetadata in format");
    }
}

#[tokio::test]
async fn test_entry_type_distribution() {
    let extractor = BibtexExtractor;

    let bibtex_content = r#"
@article{a1, author={A}, title={T1}, year={2023}}
@article{a2, author={B}, title={T2}, year={2023}}
@book{b1, author={C}, title={T3}, year={2023}}
@inproceedings{c1, author={D}, title={T4}, year={2023}}
@inproceedings{c2, author={E}, title={T5}, year={2023}}
@inproceedings{c3, author={F}, title={T6}, year={2023}}
"#;

    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bibtex_content.as_bytes().to_vec(), "application/x-bibtex", None);
    let result = extractor
        .extract(input, &config)
        .await;

    assert!(result.is_ok());
    let result = result.expect("Operation failed");

    if let Some(FormatMetadata::Bibtex(ref bibtex)) = result.metadata.format {
        let entry_types = bibtex.entry_types.as_ref().expect("Entry types not extracted");

        assert_eq!(entry_types.get("article"), Some(&2));
        assert_eq!(entry_types.get("book"), Some(&1));
        assert_eq!(entry_types.get("inproceedings"), Some(&3));
    } else {
        panic!("Expected BibtexMetadata in format");
    }
}

#[tokio::test]
async fn test_unicode_support() {
    let extractor = BibtexExtractor;

    let bibtex_content = r#"
@article{unicode,
    author = {Müller, Hans and Søren, Kierkegård},
    title = {Unicode in BibTeX: A Global Perspective},
    journal = {International Journal},
    year = {2023}
}
"#;

    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bibtex_content.as_bytes().to_vec(), "application/x-bibtex", None);
    let result = extractor
        .extract(input, &config)
        .await;

    assert!(result.is_ok());
    let result = result.expect("Operation failed");

    if let Some(FormatMetadata::Bibtex(ref bibtex)) = result.metadata.format {
        assert_eq!(bibtex.entry_count, 1);
    } else {
        panic!("Expected BibtexMetadata in format");
    }
}

#[tokio::test]
async fn test_empty_fields() {
    let extractor = BibtexExtractor;

    let bibtex_content = r#"
@article{empty,
    author = {Smith, John},
    title = {Test},
    journal = {},
    year = {2023},
    volume = {}
}
"#;

    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bibtex_content.as_bytes().to_vec(), "application/x-bibtex", None);
    let result = extractor
        .extract(input, &config)
        .await;

    assert!(result.is_ok());
    let result = result.expect("Operation failed");
    if let Some(FormatMetadata::Bibtex(ref bibtex)) = result.metadata.format {
        assert_eq!(bibtex.entry_count, 1);
    } else {
        panic!("Expected BibtexMetadata in format");
    }
}

#[tokio::test]
async fn test_comprehensive_file() {
    let extractor = BibtexExtractor;

    let fixture_path = get_test_file_path("bibtex/comprehensive.bib");
    let bibtex_content = std::fs::read(&fixture_path)
        .unwrap_or_else(|err| panic!("Failed to read test file at {}: {}", fixture_path.display(), err));

    let config = ExtractionConfig::default();
    let input = ExtractInput::from_bytes(bibtex_content, "application/x-bibtex", None);
    let result = extractor
        .extract(input, &config)
        .await;

    assert!(result.is_ok());
    let result = result.expect("Operation failed");

    if let Some(FormatMetadata::Bibtex(ref bibtex)) = result.metadata.format {
        assert_eq!(bibtex.entry_count, 20);

        let entry_types = bibtex.entry_types.as_ref().expect("Entry types not extracted");
        assert!(entry_types.len() >= 10, "Should have at least 10 different entry types");

        if let Some(authors) = &result.metadata.authors {
            assert!(authors.len() > 10, "Should have many unique authors");
        }

        let year_range = bibtex.year_range.as_ref().expect("Year range not extracted");
        assert!(year_range.min.is_some());
        assert!(year_range.max.is_some());
    } else {
        panic!("Expected BibtexMetadata in format");
    }
}
