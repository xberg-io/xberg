//! Vendored from text-splitter v0.30.1 (MIT, © 2023 Benjamin Brandt). See ATTRIBUTIONS.md.
/*!
# [`MarkdownSplitter`]
Semantic splitting of Markdown documents. Tries to use as many semantic units from Markdown
as possible, according to the Common Mark specification.
*/

use std::{iter::once, ops::Range};

use either::Either;
use itertools::Itertools;
use pulldown_cmark::{Event, Options, Parser, Tag};

use super::super::{
    ChunkConfig, ChunkSizer,
    splitter::{SemanticLevel, Splitter},
    trim::Trim,
};

/// Markdown splitter. Recursively splits chunks into the largest
/// semantic units that fit within the chunk size. Also will
/// attempt to merge neighboring chunks if they can fit within the
/// given chunk size.
#[derive(Debug)]
pub struct MarkdownSplitter<Sizer>
where
    Sizer: ChunkSizer,
{
    /// Method of determining chunk sizes.
    chunk_config: ChunkConfig<Sizer>,
}

impl<Sizer> MarkdownSplitter<Sizer>
where
    Sizer: ChunkSizer,
{
    /// Creates a new [`MarkdownSplitter`].
    ///
    /// ```text
    /// let splitter = MarkdownSplitter::new(512);
    /// ```
    #[must_use]
    pub fn new(chunk_config: impl Into<ChunkConfig<Sizer>>) -> Self {
        Self {
            chunk_config: chunk_config.into(),
        }
    }

    /// Generate a list of chunks from a given text. Each chunk will be up to
    /// the `max_chunk_size`.
    ///
    /// ## Method
    ///
    /// To preserve as much semantic meaning within a chunk as possible, each chunk is composed of the largest semantic units that can fit in the next given chunk. For each splitter type, there is a defined set of semantic levels. Here is an example of the steps used:
    ///
    /// 1. Characters
    /// 2. [Unicode Grapheme Cluster Boundaries](https://www.unicode.org/reports/tr29/#Grapheme_Cluster_Boundaries)
    /// 3. [Unicode Word Boundaries](https://www.unicode.org/reports/tr29/#Word_Boundaries)
    /// 4. [Unicode Sentence Boundaries](https://www.unicode.org/reports/tr29/#Sentence_Boundaries)
    /// 5. Soft line breaks (single newline) which isn't necessarily a new element in Markdown.
    /// 6. Inline elements such as: text nodes, emphasis, strong, strikethrough, link, image, table cells, inline code, footnote references, task list markers, and inline html.
    /// 7. Block elements suce as: paragraphs, code blocks, footnote definitions, metadata. Also, a block quote or row/item within a table or list that can contain other "block" type elements, and a list or table that contains items.
    /// 8. Thematic breaks or horizontal rules.
    /// 9. Headings by level
    ///
    /// Splitting doesn't occur below the character level, otherwise you could get partial bytes of a char, which may not be a valid unicode str.
    ///
    /// Markdown is parsed according to the Commonmark spec, along with some optional features such as GitHub Flavored Markdown.
    ///
    /// ```text
    /// let splitter = MarkdownSplitter::new(10);
    /// let chunks = splitter.chunks("# Header\n\nfrom a\ndocument").collect::<Vec<_>>();
    /// ```
    pub fn chunks<'splitter, 'text: 'splitter>(
        &'splitter self,
        text: &'text str,
    ) -> impl Iterator<Item = &'text str> + 'splitter {
        Splitter::<_>::chunks(self, text)
    }
}

impl<Sizer> Splitter<Sizer> for MarkdownSplitter<Sizer>
where
    Sizer: ChunkSizer,
{
    type Level = Element;

    const TRIM: Trim = Trim::PreserveIndentation;

    fn chunk_config(&self) -> &ChunkConfig<Sizer> {
        &self.chunk_config
    }

    fn parse(&self, text: &str) -> Vec<(Self::Level, Range<usize>)> {
        Parser::new_ext(text, Options::all())
            .into_offset_iter()
            .filter_map(|(event, range)| match event {
                Event::Start(
                    Tag::Emphasis
                    | Tag::Strong
                    | Tag::Strikethrough
                    | Tag::Link { .. }
                    | Tag::Image { .. }
                    | Tag::Subscript
                    | Tag::Superscript
                    | Tag::TableCell,
                )
                | Event::Text(_)
                | Event::HardBreak
                | Event::Code(_)
                | Event::InlineHtml(_)
                | Event::InlineMath(_)
                | Event::FootnoteReference(_)
                | Event::TaskListMarker(_) => Some((Element::Inline, range)),
                Event::SoftBreak => Some((Element::SoftBreak, range)),
                Event::Html(_)
                | Event::DisplayMath(_)
                | Event::Start(
                    Tag::Paragraph
                    | Tag::CodeBlock(_)
                    | Tag::FootnoteDefinition(_)
                    | Tag::MetadataBlock(_)
                    | Tag::TableHead
                    | Tag::BlockQuote(_)
                    | Tag::TableRow
                    | Tag::Item
                    | Tag::HtmlBlock
                    | Tag::List(_)
                    | Tag::Table(_)
                    | Tag::DefinitionList
                    | Tag::DefinitionListTitle
                    | Tag::DefinitionListDefinition,
                ) => Some((Element::Block, range)),
                Event::Rule => Some((Element::Rule, range)),
                Event::Start(Tag::Heading { level, .. }) => Some((Element::Heading(level.into()), range)),
                // End events are identical to start, so no need to grab them.
                Event::End(_) => None,
            })
            .collect()
    }
}

/// Heading levels in markdown.
/// Sorted in reverse order for sorting purposes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum HeadingLevel {
    H6,
    H5,
    H4,
    H3,
    H2,
    H1,
}

impl From<pulldown_cmark::HeadingLevel> for HeadingLevel {
    fn from(value: pulldown_cmark::HeadingLevel) -> Self {
        match value {
            pulldown_cmark::HeadingLevel::H1 => HeadingLevel::H1,
            pulldown_cmark::HeadingLevel::H2 => HeadingLevel::H2,
            pulldown_cmark::HeadingLevel::H3 => HeadingLevel::H3,
            pulldown_cmark::HeadingLevel::H4 => HeadingLevel::H4,
            pulldown_cmark::HeadingLevel::H5 => HeadingLevel::H5,
            pulldown_cmark::HeadingLevel::H6 => HeadingLevel::H6,
        }
    }
}

/// How a particular semantic level relates to surrounding text elements.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SemanticSplitPosition {
    /// The semantic level should be treated as its own chunk.
    Own,
    /// The semantic level should be included in the next chunk.
    Next,
}

/// Different semantic levels that text can be split by.
/// Each level provides a method of splitting text into chunks of a given level
/// as well as a fallback in case a given fallback is too large.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Element {
    /// Single line break, which isn't necessarily a new element in Markdown
    SoftBreak,
    /// An inline element that is within a larger element such as a paragraph, but
    /// more specific than a sentence.
    Inline,
    /// Paragraph, code block, metadata, a row/item within a table or list, block quote, that can contain other "block" type elements, List or table that contains items
    Block,
    /// thematic break/horizontal rule
    Rule,
    /// Heading levels in markdown
    Heading(HeadingLevel),
}

impl Element {
    fn split_position(self) -> SemanticSplitPosition {
        match self {
            Self::SoftBreak | Self::Block | Self::Rule | Self::Inline => SemanticSplitPosition::Own,
            // Attach it to the next text
            Self::Heading(_) => SemanticSplitPosition::Next,
        }
    }

    fn treat_whitespace_as_previous(self) -> bool {
        match self {
            Self::SoftBreak | Self::Inline | Self::Rule | Self::Heading(_) => false,
            Self::Block => true,
        }
    }
}

impl SemanticLevel for Element {
    fn sections(
        text: &str,
        level_ranges: impl Iterator<Item = (Self, Range<usize>)>,
    ) -> impl Iterator<Item = (usize, &str)> {
        let mut cursor = 0;
        let mut final_match = false;
        level_ranges
            .batching(move |it| {
                loop {
                    match it.next() {
                        // If we've hit the end, actually return None
                        None if final_match => return None,
                        // First time we hit None, return the final section of the text
                        None => {
                            final_match = true;
                            return text.get(cursor..).map(|t| Either::Left(once((cursor, t))));
                        }
                        // Return text preceding match + the match
                        Some((level, range)) => {
                            let offset = cursor;
                            match level.split_position() {
                                SemanticSplitPosition::Own => {
                                    if range.start < cursor {
                                        continue;
                                    }
                                    let prev_section =
                                        text.get(cursor..range.start).expect("invalid character sequence");
                                    if level.treat_whitespace_as_previous()
                                        && prev_section.chars().all(char::is_whitespace)
                                    {
                                        let section = text.get(cursor..range.end).expect("invalid character sequence");
                                        cursor = range.end;
                                        return Some(Either::Left(once((offset, section))));
                                    }
                                    let separator =
                                        text.get(range.start..range.end).expect("invalid character sequence");
                                    cursor = range.end;
                                    return Some(Either::Right(
                                        [(offset, prev_section), (range.start, separator)].into_iter(),
                                    ));
                                }
                                SemanticSplitPosition::Next => {
                                    if range.start < cursor {
                                        continue;
                                    }
                                    let prev_section =
                                        text.get(cursor..range.start).expect("invalid character sequence");
                                    // Separator will be part of the next chunk
                                    cursor = range.start;
                                    return Some(Either::Left(once((offset, prev_section))));
                                }
                            }
                        }
                    }
                }
            })
            .flatten()
            .filter(|(_, s)| !s.is_empty())
    }
}

// Vendored upstream tests reference the `fake` crate (not in our deps) and a
// `super::super::splitter` path that the v4.10 refactor removed. Tracked
// separately; integration coverage in `crates/kreuzberg/tests/chunking_*.rs`
// exercises the same surface.
#[cfg(any())]
mod tests {
    use std::cmp::min;

    use fake::{Fake, Faker};

    use super::super::splitter::SemanticSplitRanges;

    use super::*;

    #[test]
    fn returns_one_chunk_if_text_is_shorter_than_max_chunk_size() {
        let text = Faker.fake::<String>();
        let chunks = MarkdownSplitter::new(ChunkConfig::new(text.chars().count()).with_trim(false))
            .chunks(&text)
            .collect::<Vec<_>>();

        assert_eq!(vec![&text], chunks);
    }

    #[test]
    fn returns_two_chunks_if_text_is_longer_than_max_chunk_size() {
        let text1 = Faker.fake::<String>();
        let text2 = Faker.fake::<String>();
        let text = format!("{text1}{text2}");
        // Round up to one above half so it goes to 2 chunks
        let max_chunk_size = text.chars().count() / 2 + 1;

        let chunks = MarkdownSplitter::new(ChunkConfig::new(max_chunk_size).with_trim(false))
            .chunks(&text)
            .collect::<Vec<_>>();

        assert!(chunks.iter().all(|c| c.chars().count() <= max_chunk_size));

        // Check that beginning of first chunk and text 1 matches
        let len = min(text1.len(), chunks[0].len());
        assert_eq!(text1[..len], chunks[0][..len]);
        // Check that end of second chunk and text 2 matches
        let len = min(text2.len(), chunks[1].len());
        assert_eq!(text2[(text2.len() - len)..], chunks[1][chunks[1].len() - len..]);

        assert_eq!(chunks.join(""), text);
    }

    #[test]
    fn empty_string() {
        let text = "";
        let chunks = MarkdownSplitter::new(ChunkConfig::new(100).with_trim(false))
            .chunks(text)
            .collect::<Vec<_>>();

        assert!(chunks.is_empty());
    }

    #[test]
    fn can_handle_unicode_characters() {
        let text = "éé"; // Char that is more than one byte
        let chunks = MarkdownSplitter::new(ChunkConfig::new(1).with_trim(false))
            .chunks(text)
            .collect::<Vec<_>>();

        assert_eq!(vec!["é", "é"], chunks);
    }

    #[test]
    fn chunk_by_graphemes() {
        let text = "a̐éö̲\r\n";
        let chunks = MarkdownSplitter::new(ChunkConfig::new(3).with_trim(false))
            .chunks(text)
            .collect::<Vec<_>>();

        // \r\n is grouped together not separated
        assert_eq!(vec!["a̐é", "ö̲", "\r\n"], chunks);
    }

    #[test]
    fn graphemes_fallback_to_chars() {
        let text = "a̐éö̲\r\n";
        let chunks = MarkdownSplitter::new(ChunkConfig::new(1).with_trim(false))
            .chunks(text)
            .collect::<Vec<_>>();

        assert_eq!(vec!["a", "\u{310}", "é", "ö", "\u{332}", "\r", "\n"], chunks);
    }

    #[test]
    fn chunk_by_words() {
        let text = "The quick brown fox can jump 32.3 feet, right?";
        let chunks = MarkdownSplitter::new(ChunkConfig::new(10).with_trim(false))
            .chunks(text)
            .collect::<Vec<_>>();

        assert_eq!(
            vec!["The quick ", "brown fox ", "can jump ", "32.3 feet,", " right?"],
            chunks
        );
    }

    #[test]
    fn words_fallback_to_graphemes() {
        let text = "Thé quick\r\n";
        let chunks = MarkdownSplitter::new(ChunkConfig::new(2).with_trim(false))
            .chunks(text)
            .collect::<Vec<_>>();

        assert_eq!(vec!["Th", "é ", "qu", "ic", "k", "\r\n"], chunks);
    }

    #[test]
    fn chunk_by_sentences() {
        let text = "Mr. Fox jumped. The dog was too lazy.";
        let chunks = MarkdownSplitter::new(ChunkConfig::new(21).with_trim(false))
            .chunks(text)
            .collect::<Vec<_>>();

        assert_eq!(vec!["Mr. Fox jumped. ", "The dog was too lazy."], chunks);
    }

    #[test]
    fn sentences_falls_back_to_words() {
        let text = "Mr. Fox jumped. The dog was too lazy.";
        let chunks = MarkdownSplitter::new(ChunkConfig::new(16).with_trim(false))
            .chunks(text)
            .collect::<Vec<_>>();

        assert_eq!(vec!["Mr. Fox jumped. ", "The dog was too ", "lazy."], chunks);
    }

    #[test]
    fn test_no_markdown_separators() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("Some text without any markdown separators"));

        assert_eq!(
            vec![(Element::Block, 0..41), (Element::Inline, 0..41)],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_checklist() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("- [ ] incomplete task\n- [x] completed task"));

        assert_eq!(
            vec![
                (Element::Block, 0..42),
                (Element::Block, 0..22),
                (Element::Inline, 2..5),
                (Element::Inline, 6..21),
                (Element::Block, 22..42),
                (Element::Inline, 24..27),
                (Element::Inline, 28..42),
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_footnote_reference() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("Footnote[^1]"));

        assert_eq!(
            vec![
                (Element::Block, 0..12),
                (Element::Inline, 0..8),
                (Element::Inline, 8..12),
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_inline_code() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("`bash`"));

        assert_eq!(
            vec![(Element::Block, 0..6), (Element::Inline, 0..6)],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_emphasis() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("*emphasis*"));

        assert_eq!(
            vec![
                (Element::Block, 0..10),
                (Element::Inline, 0..10),
                (Element::Inline, 1..9),
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_strong() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("**emphasis**"));

        assert_eq!(
            vec![
                (Element::Block, 0..12),
                (Element::Inline, 0..12),
                (Element::Inline, 2..10),
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_strikethrough() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("~~emphasis~~"));

        assert_eq!(
            vec![
                (Element::Block, 0..12),
                (Element::Inline, 0..12),
                (Element::Inline, 2..10),
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_link() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("[link](url)"));

        assert_eq!(
            vec![
                (Element::Block, 0..11),
                (Element::Inline, 0..11),
                (Element::Inline, 1..5),
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_image() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("![link](url)"));

        assert_eq!(
            vec![
                (Element::Block, 0..12),
                (Element::Inline, 0..12),
                (Element::Inline, 2..6),
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_inline_html() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("<span>Some text</span>"));

        assert_eq!(
            vec![
                (Element::Block, 0..22),
                (Element::Inline, 0..6),
                (Element::Inline, 6..15),
                (Element::Inline, 15..22),
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_html() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("<div>Some text</div>"));

        assert_eq!(
            vec![(Element::Block, 0..20), (Element::Block, 0..20)],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_table() {
        let splitter = MarkdownSplitter::new(10);
        let markdown =
            SemanticSplitRanges::new(splitter.parse("| Header 1 | Header 2 |\n| --- | --- |\n| Cell 1 | Cell 2 |"));
        assert_eq!(
            vec![
                (Element::Block, 0..57),
                (Element::Block, 0..24),
                (Element::Inline, 1..11),
                (Element::Inline, 2..10),
                (Element::Inline, 12..22),
                (Element::Inline, 13..21),
                (Element::Block, 38..57),
                (Element::Inline, 39..47),
                (Element::Inline, 40..46),
                (Element::Inline, 48..56),
                (Element::Inline, 49..55)
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_softbreak() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("Some text\nwith a softbreak"));

        assert_eq!(
            vec![
                (Element::Block, 0..26),
                (Element::Inline, 0..9),
                (Element::SoftBreak, 9..10),
                (Element::Inline, 10..26)
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_hardbreak() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("Some text\\\nwith a hardbreak"));

        assert_eq!(
            vec![
                (Element::Block, 0..27),
                (Element::Inline, 0..9),
                (Element::Inline, 9..11),
                (Element::Inline, 11..27)
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_footnote_def() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("[^first]: Footnote"));

        assert_eq!(
            vec![
                (Element::Block, 0..18),
                (Element::Block, 10..18),
                (Element::Inline, 10..18)
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_code_block() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("```\ncode\n```"));

        assert_eq!(
            vec![(Element::Block, 0..12), (Element::Inline, 4..9)],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_block_quote() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("> quote"));

        assert_eq!(
            vec![(Element::Block, 0..7), (Element::Block, 2..7), (Element::Inline, 2..7)],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_with_rule() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("Some text\n\n---\n\nwith a rule"));

        assert_eq!(
            vec![
                (Element::Block, 0..10),
                (Element::Inline, 0..9),
                (Element::Rule, 11..15),
                (Element::Block, 16..27),
                (Element::Inline, 16..27)
            ],
            markdown.ranges_after_offset(0).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_heading() {
        for (index, (heading, level)) in [
            ("#", HeadingLevel::H1),
            ("##", HeadingLevel::H2),
            ("###", HeadingLevel::H3),
            ("####", HeadingLevel::H4),
            ("#####", HeadingLevel::H5),
            ("######", HeadingLevel::H6),
        ]
        .into_iter()
        .enumerate()
        {
            let splitter = MarkdownSplitter::new(10);
            let markdown = SemanticSplitRanges::new(splitter.parse(&format!("{heading} Heading")));

            assert_eq!(
                vec![
                    (Element::Heading(level), 0..9 + index),
                    (Element::Inline, 2 + index..9 + index)
                ],
                markdown.ranges_after_offset(0).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_ranges_after_offset_block() {
        let splitter = MarkdownSplitter::new(10);
        let markdown = SemanticSplitRanges::new(splitter.parse("- [ ] incomplete task\n- [x] completed task"));

        assert_eq!(
            vec![(Element::Block, 0..22), (Element::Block, 22..42),],
            markdown
                .level_ranges_after_offset(0, Element::Block)
                .collect::<Vec<_>>()
        );
    }
}
