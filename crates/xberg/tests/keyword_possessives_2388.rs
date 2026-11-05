#![cfg(feature = "keywords-yake")]

use xberg::keywords::{Keyword, KeywordAlgorithm, KeywordConfig, NgramRange, extract_keywords};

fn check_results(keywords: &[Keyword], config: &KeywordConfig) {
    assert!(!keywords.is_empty(), "the real extractor must return keywords");
    assert!(keywords.len() <= config.max_keywords);
    for keyword in keywords {
        assert_eq!(keyword.algorithm, KeywordAlgorithm::Yake);
        assert!(keyword.score.is_finite() && (0.0..=1.0).contains(&keyword.score));
    }
}

#[test]
fn default_keywords_do_not_invent_possessive_prefix_ngrams() {
    let text = "COMMISSION'S CONTRACT requires CONSULTANT'S INSURANCE. \
                COMMISSION approves the CONTRACT. CONSULTANT maintains INSURANCE.";
    let config = KeywordConfig::default();
    assert_eq!(config.algorithm, KeywordAlgorithm::Yake);
    let forbidden = [
        "commission commission's contract",
        "commission commission's",
        "consultant consultant's",
        "consultant consultant's insurance",
    ];
    let source = text.to_ascii_lowercase();
    for phrase in forbidden {
        assert!(!source.contains(phrase), "the fixture never contains this adjacency");
    }
    let keywords = extract_keywords(text, &config).unwrap();
    check_results(&keywords, &config);
    assert!(keywords.iter().any(|keyword| keyword.text.contains("contract")));
    let invented: Vec<_> = keywords
        .iter()
        .filter(|keyword| forbidden.contains(&keyword.text.to_ascii_lowercase().as_str()))
        .map(|keyword| keyword.text.as_str())
        .collect();
    assert_eq!(
        invented,
        Vec::<&str>::new(),
        "default output must follow source word adjacency"
    );
}

#[test]
fn public_unigrams_merge_ascii_curly_and_case_possessive_variants() {
    let text = "cat cat's cat’s CAT’S";
    assert_eq!(text.split_whitespace().count(), 4);
    for (start, expected) in [(0, "cat"), (4, "cat"), (10, "cat"), (16, "CAT")] {
        assert_eq!(text.chars().skip(start).take(3).collect::<String>(), expected);
    }
    let config = KeywordConfig {
        max_keywords: 32,
        ngram_range: NgramRange { min: 1, max: 1 },
        ..Default::default()
    };
    let keywords = extract_keywords(text, &config).unwrap();
    check_results(&keywords, &config);
    let mut terms: Vec<_> = keywords.iter().map(|keyword| keyword.text.as_str()).collect();
    terms.sort_unstable();
    assert_eq!(
        terms,
        vec!["cat"],
        "canonical terms must agree before approximate deduplication"
    );
    assert_eq!(keywords[0].positions, Some(vec![0, 4, 10, 16]));
}

#[test]
fn public_keywords_preserve_genuinely_repeated_names() {
    let text = "Baden Baden hosts an annual congress.";
    assert_eq!(&text[..11], "Baden Baden");
    let config = KeywordConfig {
        max_keywords: 32,
        ngram_range: NgramRange { min: 2, max: 2 },
        ..Default::default()
    };
    let keywords = extract_keywords(text, &config).unwrap();
    check_results(&keywords, &config);
    let name = keywords
        .iter()
        .find(|keyword| keyword.text == "baden baden")
        .expect("actual repeated proper names must remain eligible");
    assert_eq!(name.positions, Some(vec![0]));
}
