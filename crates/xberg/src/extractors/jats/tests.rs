use super::*;
use elements::extract_jats_all_in_one;

fn element_texts(jats: &str) -> Vec<(String, String)> {
    let mut budget = SecurityBudget::with_defaults();
    build_jats_internal_document(jats, &mut budget)
        .expect("parse")
        .elements
        .iter()
        .map(|e| (format!("{:?}", e.kind), e.text.clone()))
        .collect()
}

#[test]
fn should_keep_inline_markup_inside_the_word_it_belongs_to() {
    let jats = r#"<article><front><article-meta><title-group><article-title>Effects of CO<sub>2</sub> on <italic>E. coli</italic></article-title></title-group></article-meta></front>
<body><p>We studied H<sub>2</sub>O uptake<xref ref-type="bibr" rid="b1">1</xref> in
   <italic>vitro</italic> and <bold>in</bold> <bold>vivo</bold>.</p></body></article>"#;
    let texts = element_texts(jats);

    assert!(
        texts.iter().any(|(_, t)| t == "Effects of CO2 on E. coli"),
        "got: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|(_, t)| t == "We studied H2O uptake1 in vitro and in vivo."),
        "got: {texts:?}"
    );
}

#[test]
fn should_keep_block_and_line_break_boundaries_as_separators() {
    let jats = r#"<article><body><p><bold>first<break/>second</bold> and a list<list><list-item><p>item</p></list-item></list>after</p>
<table-wrap><table><tr><td>first<p>second</p></td><td>x</td></tr></table></table-wrap></body></article>"#;
    let mut budget = SecurityBudget::with_defaults();
    let doc = build_jats_internal_document(jats, &mut budget).expect("parse");
    let texts: Vec<&str> = doc.elements.iter().map(|e| e.text.as_str()).collect();
    let markdown = crate::rendering::render_markdown(&doc);

    assert!(texts.contains(&"first second and a list item after"), "got: {texts:?}");
    assert!(markdown.contains("| first second | x |"), "got: {markdown}");
}

#[test]
fn should_read_table_rows_outside_a_row_group() {
    let jats = r#"<article><body><table-wrap><table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td>2</td></tr><tfoot><tr><td>Total</td><td>3</td></tr></tfoot></table></table-wrap></body></article>"#;
    let mut budget = SecurityBudget::with_defaults();
    let doc = build_jats_internal_document(jats, &mut budget).expect("parse");
    let markdown = crate::rendering::render_markdown(&doc);

    assert!(markdown.contains("| A | B |"), "got: {markdown}");
    assert!(markdown.contains("| 1 | 2 |"), "got: {markdown}");
    assert!(markdown.contains("| Total | 3 |"), "got: {markdown}");
}

/// A journal writes its equation inside the sentence that refers to it.
/// The paragraph reader consumed the whole `<p>`, so an article whose
/// mathematics is inline reported none of it.
#[test]
fn test_formula_inside_a_paragraph_reaches_the_formula_list() {
    let jats = r#"<article><body><sec><p>The value <inline-formula><alternatives>
<tex-math><![CDATA[J]]></tex-math>
<mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>J</mml:mi></mml:math></alternatives></inline-formula> is the cost.</p></sec></body></article>"#;

    let mut budget = SecurityBudget::with_defaults();
    let doc = build_jats_internal_document(jats, &mut budget).expect("parse");

    let formulas: Vec<&str> = doc
        .elements
        .iter()
        .filter(|e| matches!(e.kind, crate::types::internal::ElementKind::Formula))
        .map(|e| e.text.as_str())
        .collect();
    assert_eq!(
        formulas,
        vec!["J"],
        "the inline equation is one of the document's formulas"
    );
}

#[test]
fn test_jats_extractor_plugin_interface() {
    let extractor = JatsExtractor::new();
    assert_eq!(extractor.name(), "jats-extractor");
    assert!(extractor.initialize().is_ok());
    assert!(extractor.shutdown().is_ok());
}

#[test]
fn test_jats_extractor_supported_mime_types() {
    let extractor = JatsExtractor::new();
    let mime_types = extractor.supported_mime_types();
    assert_eq!(mime_types.len(), 2);
    assert!(mime_types.contains(&"application/x-jats+xml"));
    assert!(mime_types.contains(&"text/jats"));
}

#[test]
fn test_jats_extractor_priority() {
    let extractor = JatsExtractor::new();
    assert_eq!(extractor.priority(), 50);
}

#[test]
fn test_parse_simple_jats_article() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Test Article Title</article-title>
    </article-meta>
  </front>
  <body>
    <p>Test content paragraph.</p>
  </body>
</article>"#;

    let (metadata, content, title, _tables) = extract_jats_all_in_one(jats).expect("Parse failed");
    assert_eq!(title, "Test Article Title");
    assert_eq!(metadata.title, "Test Article Title");
    assert!(content.contains("Test Article Title"));
    assert!(content.contains("Test content"));
}

#[test]
fn test_extract_jats_title() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Effects of Caffeine on Human Health</article-title>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert_eq!(metadata.title, "Effects of Caffeine on Human Health");
}

#[test]
fn test_extract_jats_subtitle() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Main Title</article-title>
      <subtitle>A Systematic Review</subtitle>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert_eq!(metadata.title, "Main Title");
    assert_eq!(metadata.subtitle, Some("A Systematic Review".to_string()));
}

#[test]
fn test_extract_jats_authors() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <contrib-group>
        <contrib contrib-type="author">
          <name>
            <surname>Smith</surname>
            <given-names>John A.</given-names>
          </name>
        </contrib>
        <contrib contrib-type="author">
          <name>
            <surname>Johnson</surname>
            <given-names>Jane B.</given-names>
          </name>
        </contrib>
      </contrib-group>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert_eq!(metadata.authors.len(), 2);
    assert!(metadata.authors[0].contains("Smith"));
    assert!(metadata.authors[1].contains("Johnson"));
}

#[test]
fn test_extract_jats_affiliations() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <aff id="aff1">Department of Medicine, Harvard University, Cambridge, MA</aff>
      <aff id="aff2">Center for Health Research, Boston Medical Center, Boston, MA</aff>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert_eq!(metadata.affiliations.len(), 2);
    assert!(metadata.affiliations[0].contains("Harvard"));
    assert!(metadata.affiliations[1].contains("Boston"));
}

#[test]
fn test_extract_jats_doi_and_pii() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-id pub-id-type="doi">10.1371/journal.pmed.0020124</article-id>
      <article-id pub-id-type="pii">05-PLME-RA-0071R2</article-id>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert_eq!(metadata.doi, Some("10.1371/journal.pmed.0020124".to_string()));
    assert_eq!(metadata.pii, Some("05-PLME-RA-0071R2".to_string()));
}

#[test]
fn test_extract_jats_keywords() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <kwd-group>
        <kwd>caffeine</kwd>
        <kwd>meta-analysis</kwd>
        <kwd>systematic review</kwd>
      </kwd-group>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert_eq!(metadata.keywords.len(), 3);
    assert!(metadata.keywords.contains(&"caffeine".to_string()));
    assert!(metadata.keywords.contains(&"meta-analysis".to_string()));
}

#[test]
fn test_extract_jats_publication_info() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <pub-date pub-type="epub">
        <day>18</day>
        <month>04</month>
        <year>2005</year>
      </pub-date>
      <volume>2</volume>
      <issue>4</issue>
      <fpage>e124</fpage>
      <lpage>e132</lpage>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert!(metadata.publication_date.is_some());
    assert_eq!(metadata.volume, Some("2".to_string()));
    assert_eq!(metadata.issue, Some("4".to_string()));
    assert!(metadata.pages.is_some());
}

#[test]
fn test_extract_jats_abstract() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <abstract>
        <sec>
          <title>Background</title>
          <p>This is the background information of the study.</p>
        </sec>
        <sec>
          <title>Methods</title>
          <p>We used quantitative analysis to evaluate the hypothesis.</p>
        </sec>
      </abstract>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert!(metadata.abstract_text.is_some());
    let abstract_text = metadata.abstract_text.expect("abstract_text should be present");
    assert!(abstract_text.contains("background"));
    assert!(abstract_text.contains("quantitative"));
}

#[test]
fn test_extract_jats_tables_basic() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <body>
    <table-wrap id="tbl1">
      <table>
        <thead>
          <tr>
            <th>Study</th>
            <th>Year</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Study A</td>
            <td>2003</td>
          </tr>
          <tr>
            <td>Study B</td>
            <td>2004</td>
          </tr>
        </tbody>
      </table>
    </table-wrap>
  </body>
</article>"#;

    let (_metadata, _content, _title, tables) = extract_jats_all_in_one(jats).expect("Table extraction failed");
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].cells.len(), 3);
}

#[test]
fn test_extract_jats_corresponding_author() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <author-notes>
        <corresp id="cor1">To whom correspondence should be addressed. E-mail: rwilliams@yale.edu</corresp>
      </author-notes>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert!(metadata.corresponding_author.is_some());
    let corresp = metadata
        .corresponding_author
        .expect("corresponding_author should be present");
    assert!(corresp.contains("rwilliams"));
}

#[test]
fn test_extract_jats_section_hierarchy() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Article Title</article-title>
    </article-meta>
  </front>
  <body>
    <sec id="s1">
      <title>Introduction</title>
      <p>Intro content.</p>
    </sec>
    <sec id="s2">
      <title>Methods</title>
      <p>Methods content.</p>
    </sec>
    <sec id="s3">
      <title>Results</title>
      <p>Results content.</p>
    </sec>
  </body>
</article>"#;

    let (_metadata, content, _title, _tables) = extract_jats_all_in_one(jats).expect("Parse failed");
    assert!(content.contains("Introduction"));
    assert!(content.contains("Methods"));
    assert!(content.contains("Results"));
}

#[test]
fn test_jats_extractor_full_metadata_extraction() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Sample Article</article-title>
      <contrib-group>
        <contrib contrib-type="author">
          <name>
            <surname>Smith</surname>
            <given-names>John</given-names>
          </name>
        </contrib>
      </contrib-group>
      <article-id pub-id-type="doi">10.1234/test</article-id>
      <kwd-group>
        <kwd>test</kwd>
      </kwd-group>
      <abstract>
        <p>Test abstract.</p>
      </abstract>
    </article-meta>
  </front>
  <body>
    <p>Sample content.</p>
  </body>
</article>"#;

    let (metadata_extracted, _content, _title, _tables) =
        extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert_eq!(metadata_extracted.title, "Sample Article");
    assert_eq!(metadata_extracted.authors.len(), 1);
    assert_eq!(metadata_extracted.doi, Some("10.1234/test".to_string()));
    assert_eq!(metadata_extracted.keywords.len(), 1);
    assert!(metadata_extracted.abstract_text.is_some());
}

#[test]
fn test_jats_extractor_empty_article() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
    </article-meta>
  </front>
  <body>
  </body>
</article>"#;

    let (metadata, content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert!(metadata.title.is_empty());
    assert!(content.is_empty() || content.trim().is_empty());
}

#[test]
fn test_extract_jats_journal_title() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Test Article</article-title>
      <journal-title>Nature Medicine</journal-title>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert_eq!(metadata.journal_title, Some("Nature Medicine".to_string()));
}

#[test]
fn test_extract_jats_article_type() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article article-type="research-article">
  <front>
    <article-meta>
      <article-title>Test Article</article-title>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");
    assert_eq!(metadata.article_type, Some("research-article".to_string()));
}

#[test]
fn test_extract_all_13_metadata_fields() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article article-type="research-article">
  <front>
    <article-meta>
      <article-title>Full Metadata Test</article-title>
      <subtitle>A Complete Example</subtitle>
      <contrib-group>
        <contrib contrib-type="author">
          <name>
            <surname>Author</surname>
            <given-names>First</given-names>
          </name>
        </contrib>
      </contrib-group>
      <aff>Department of Testing, Test University</aff>
      <article-id pub-id-type="doi">10.1234/full-test</article-id>
      <article-id pub-id-type="pii">TEST-001</article-id>
      <kwd-group>
        <kwd>testing</kwd>
        <kwd>metadata</kwd>
      </kwd-group>
      <pub-date pub-type="epub">
        <year>2024</year>
      </pub-date>
      <volume>5</volume>
      <issue>3</issue>
      <fpage>100</fpage>
      <lpage>110</lpage>
      <journal-title>Test Journal</journal-title>
      <abstract>
        <p>This is a test abstract for all metadata fields.</p>
      </abstract>
      <author-notes>
        <corresp>Correspondence: test@example.com</corresp>
      </author-notes>
    </article-meta>
  </front>
  <body>
    <p>Test content.</p>
  </body>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Metadata extraction failed");

    assert_eq!(metadata.title, "Full Metadata Test");
    assert_eq!(metadata.subtitle, Some("A Complete Example".to_string()));
    assert_eq!(metadata.authors.len(), 1);
    assert!(metadata.authors[0].contains("Author"));
    assert_eq!(metadata.affiliations.len(), 1);
    assert!(metadata.affiliations[0].contains("Testing"));
    assert_eq!(metadata.doi, Some("10.1234/full-test".to_string()));
    assert_eq!(metadata.pii, Some("TEST-001".to_string()));
    assert_eq!(metadata.keywords.len(), 2);
    assert!(metadata.publication_date.is_some());
    assert_eq!(metadata.volume, Some("5".to_string()));
    assert_eq!(metadata.issue, Some("3".to_string()));
    assert!(metadata.pages.is_some());
    assert_eq!(metadata.journal_title, Some("Test Journal".to_string()));
    assert_eq!(metadata.article_type, Some("research-article".to_string()));
    assert!(metadata.abstract_text.is_some());
    assert!(metadata.corresponding_author.is_some());
}

#[test]
fn test_extract_jats_history_dates() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Test Article</article-title>
      <history>
        <date date-type="received">
          <day>15</day>
          <month>01</month>
          <year>2024</year>
        </date>
        <date date-type="accepted">
          <day>20</day>
          <month>03</month>
          <year>2024</year>
        </date>
      </history>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Parse failed");
    assert_eq!(metadata.history_dates.len(), 2);
    assert_eq!(metadata.history_dates[0].0, "received");
    assert!(metadata.history_dates[0].1.contains("2024"));
    assert_eq!(metadata.history_dates[1].0, "accepted");
}

#[test]
fn test_extract_jats_permissions() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Test Article</article-title>
      <permissions>
        <copyright-statement>Copyright 2024 The Authors</copyright-statement>
        <license>
          <license-p>This is an open-access article distributed under the CC BY 4.0 license.</license-p>
        </license>
      </permissions>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Parse failed");
    assert_eq!(
        metadata.copyright_statement,
        Some("Copyright 2024 The Authors".to_string())
    );
    assert!(metadata.license.is_some());
    assert!(
        metadata
            .license
            .expect("license should be present")
            .contains("CC BY 4.0")
    );
}

#[test]
fn test_extract_jats_inline_formula() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Formula Test</article-title>
    </article-meta>
  </front>
  <body>
    <p>The equation <inline-formula>E = mc^2</inline-formula> is famous.</p>
  </body>
</article>"#;

    let (_metadata, content, _title, _tables) = extract_jats_all_in_one(jats).expect("Parse failed");
    assert!(content.contains("E = mc^2"), "expected inline formula, got: {content}");
}

#[test]
fn test_extract_jats_contributor_roles() {
    let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Test Article</article-title>
      <contrib-group>
        <contrib contrib-type="author">
          <name>
            <surname>Smith</surname>
            <given-names>John</given-names>
          </name>
        </contrib>
        <contrib contrib-type="editor">
          <name>
            <surname>Jones</surname>
            <given-names>Mary</given-names>
          </name>
        </contrib>
      </contrib-group>
    </article-meta>
  </front>
</article>"#;

    let (metadata, _content, _title, _tables) = extract_jats_all_in_one(jats).expect("Parse failed");
    assert_eq!(metadata.contributor_roles.len(), 2);

    let author_role = metadata
        .contributor_roles
        .iter()
        .find(|(_, role)| role == "author")
        .expect("expected author role");
    assert!(author_role.0.contains("Smith"));

    let editor_role = metadata
        .contributor_roles
        .iter()
        .find(|(_, role)| role == "editor")
        .expect("expected editor role");
    assert!(editor_role.0.contains("Jones"));
}
