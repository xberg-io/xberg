use crate::ocr::hocr_parser::HocrWordOccurrence;
use crate::pdf::structure::pipeline::{HyphenWitnesses, should_preserve_lexical_hyphen};
use crate::types::internal::{ElementKind, InternalElement};
use std::collections::HashSet;
use std::ops::Range;

#[path = "dehyphenation/continuation.rs"]
mod continuation;

struct Line<'a> {
    words: &'a [HocrWordOccurrence],
    element: usize,
}

#[derive(Clone, Copy)]
struct LineBounds {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl LineBounds {
    fn from_words(words: &[HocrWordOccurrence]) -> Option<Self> {
        let &[left, top, right, bottom] = &words.first()?.bounds;
        let mut line = Self {
            left,
            top,
            right,
            bottom,
        };
        for word in words {
            let [x0, y0, x1, y1] = word.bounds;
            if x0 >= x1 || y0 >= y1 {
                return None;
            }
            line.left = line.left.min(x0);
            line.top = line.top.min(y0);
            line.right = line.right.max(x1);
            line.bottom = line.bottom.max(y1);
        }
        let gap = line.height().saturating_mul(3);
        words
            .windows(2)
            .all(|pair| pair[1].bounds[0].saturating_sub(pair[0].bounds[2]) <= gap)
            .then_some(line)
    }

    fn height(self) -> u32 {
        self.bottom - self.top
    }

    fn full_width(self, page_width: u32) -> bool {
        let width = u64::from(page_width);
        width > 0
            && self.right <= page_width
            && u64::from(self.right - self.left) * 5 >= width * 3
            && u64::from(self.left) * 2 < width
            && u64::from(self.right) * 2 > width
    }

    fn follows(self, prior: Self) -> bool {
        let tolerance = prior.height().max(self.height());
        self.left.abs_diff(prior.left) <= tolerance
            && self.top > prior.top
            && self.bottom > prior.bottom
            && self.top.abs_diff(prior.bottom) <= tolerance
    }
}

struct Edit {
    element: usize,
    range: Range<usize>,
    replacement: String,
}

impl Edit {
    fn apply(self, elements: &mut [InternalElement]) {
        elements[self.element].text.replace_range(self.range, &self.replacement);
    }
}

struct Join {
    trailing: Edit,
    leading: Option<Edit>,
}

struct SpellingWitnesses {
    words: HashSet<String>,
    compounds: HyphenWitnesses,
}

/// Whether any element ends a line with a hyphen, the precondition for [`join_scanned_wraps`].
pub(super) fn may_join_scanned_wraps(elements: &[InternalElement], sparse: bool) -> bool {
    elements
        .iter()
        .any(|element| element.text.contains("-\n") || (sparse && element.text.ends_with('-')))
}

pub(super) fn join_scanned_wraps(
    elements: &mut Vec<InternalElement>,
    words: &[HocrWordOccurrence],
    page_width: u32,
    sparse: bool,
) -> bool {
    if !may_join_scanned_wraps(elements, sparse) {
        return false;
    }
    let page = elements.first().and_then(|element| element.page);
    if elements.iter().any(|element| element.page != page) {
        return false;
    }
    let Some(lines) = physical_lines(elements, words) else {
        return false;
    };
    let witnesses = collect_spelling_witnesses(elements, words);

    let joins: Vec<_> = (0..lines.len().saturating_sub(1))
        .filter_map(|index| witnessed_join(&lines, index, elements, &witnesses, page_width, sparse))
        .collect();
    let changed = !joins.is_empty();
    // Joins apply last to first, so removing a later element leaves earlier edit indices valid. ~keep
    for join in joins.into_iter().rev() {
        if let Some(leading) = join.leading {
            let element = leading.element;
            leading.apply(elements);
            // A sparse block whose only word moved up would otherwise become an empty paragraph. ~keep
            if elements[element].text.is_empty() {
                elements.remove(element);
            }
        }
        join.trailing.apply(elements);
    }
    changed
}

fn collect_spelling_witnesses(elements: &[InternalElement], words: &[HocrWordOccurrence]) -> SpellingWitnesses {
    let mut spellings = HashSet::new();
    let mut compounds = HyphenWitnesses::default();
    for core in words
        .iter()
        .filter_map(|word| word_text(elements, word))
        .map(lexical_core)
    {
        if alphabetic_word_part(core) {
            spellings.insert(lowercase_spelling(core));
        } else if let Some((left, right)) = core.split_once('-')
            && alphabetic_word_part(left)
            && alphabetic_word_part(right)
        {
            compounds.insert((lowercase_spelling(left), lowercase_spelling(right)));
        }
    }
    SpellingWitnesses {
        words: spellings,
        compounds,
    }
}

fn word_text<'a>(elements: &'a [InternalElement], word: &HocrWordOccurrence) -> Option<&'a str> {
    elements.get(word.element as usize)?.text.get(word.bytes.clone())
}

fn physical_lines<'a>(elements: &[InternalElement], words: &'a [HocrWordOccurrence]) -> Option<Vec<Line<'a>>> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, pair) in words.windows(2).enumerate() {
        let (prior, next) = (&pair[0], &pair[1]);
        let boundary = prior.element != next.element
            || elements
                .get(prior.element as usize)?
                .text
                .get(prior.bytes.end..next.bytes.start)?
                == "\n";
        if boundary {
            lines.push(Line {
                words: &words[start..=index],
                element: prior.element as usize,
            });
            start = index + 1;
        }
    }
    if let Some(last) = words.last() {
        lines.push(Line {
            words: &words[start..],
            element: last.element as usize,
        });
    }
    Some(lines)
}

fn witnessed_join(
    lines: &[Line<'_>],
    index: usize,
    elements: &[InternalElement],
    witnesses: &SpellingWitnesses,
    page_width: u32,
    sparse: bool,
) -> Option<Join> {
    let (prior, next) = (&lines[index], &lines[index + 1]);
    let former_element = elements.get(prior.element)?;
    let latter_element = elements.get(next.element)?;
    if former_element.page != latter_element.page || !can_edit(former_element) || !can_edit(latter_element) {
        return None;
    }
    let separate_elements = prior.element != next.element;
    if separate_elements
        && (!sparse
            || next.element != prior.element + 1
            || former_element.text.contains('\n')
            || latter_element.text.contains('\n'))
    {
        return None;
    }
    let trailing = prior.words.last()?;
    let leading = next.words.first()?;
    let former = word_text(elements, trailing)?;
    let latter = word_text(elements, leading)?;
    if !latter
        .chars()
        .next()
        .is_some_and(|first| first.is_alphabetic() && first.is_lowercase())
    {
        return None;
    }
    let edit_start = witnessed_edit_start(former, lexical_core(latter), witnesses)?;
    let terminal_word = next.words.len() == 1;
    if terminal_word && (leading.bytes.end != latter_element.text.len() || edit_start == former.len()) {
        return None;
    }
    if terminal_word && separate_elements {
        continuation::bracketed_terminal_fragment(lines, index, elements, page_width)?;
    } else {
        same_body_flow(prior, next, page_width, terminal_word)?;
    }
    plan_edits(prior, next, elements, edit_start, latter)
}

fn plan_edits(
    prior: &Line<'_>,
    next: &Line<'_>,
    elements: &[InternalElement],
    edit_start: usize,
    latter: &str,
) -> Option<Join> {
    let former_element = elements.get(prior.element)?;
    let latter_element = elements.get(next.element)?;
    let trailing = prior.words.last()?;
    let leading = next.words.first()?;
    let start = trailing.bytes.start + edit_start;
    let end = if let Some(following) = next.words.get(1) {
        let end = following.bytes.start;
        if latter_element.text.get(leading.bytes.end..end)? != " " {
            return None;
        }
        end
    } else {
        leading.bytes.end
    };
    if prior.element != next.element {
        return (trailing.bytes.end == former_element.text.len() && leading.bytes.start == 0).then(|| Join {
            trailing: Edit {
                element: prior.element,
                range: start..trailing.bytes.end,
                replacement: latter.to_owned(),
            },
            leading: Some(Edit {
                element: next.element,
                range: 0..end,
                replacement: String::new(),
            }),
        });
    }
    let middle = former_element.text.get(trailing.bytes.end..leading.bytes.start)?;
    if middle != "\n" {
        return None;
    }

    // Keep the physical line boundary so the paragraph's line metadata retains its indexing. ~keep
    Some(Join {
        trailing: Edit {
            element: prior.element,
            range: start..end,
            replacement: format!("{latter}\n"),
        },
        leading: None,
    })
}

fn can_edit(element: &InternalElement) -> bool {
    element.annotations.is_empty()
        && element.ocr_source_spans.is_empty()
        && matches!(element.kind, ElementKind::OcrText { .. })
}

fn same_body_flow(prior: &Line<'_>, next: &Line<'_>, page_width: u32, terminal_word: bool) -> Option<()> {
    let before = LineBounds::from_words(prior.words)?;
    let after = LineBounds::from_words(next.words)?;
    let right_edge_matches = if terminal_word {
        after.left >= before.left && after.right <= before.right
    } else {
        let tolerance = before.height().max(after.height()).saturating_mul(2);
        if prior.element == next.element {
            after.right <= before.right.saturating_add(tolerance).min(page_width)
        } else {
            // Separate sparse blocks still need full-width geometry on both sides. ~keep
            after.full_width(page_width) && before.right.abs_diff(after.right) <= tolerance
        }
    };
    (prior.words.last()?.bounds[2] == before.right
        && next.words.first()?.bounds[0] == after.left
        && before.full_width(page_width)
        && after.follows(before)
        && right_edge_matches)
        .then_some(())
}

fn lexical_core(word: &str) -> &str {
    word.trim_matches([
        '.', ',', ';', ':', '!', '?', '"', '\'', '(', ')', '[', ']', '{', '}', '‘', '’', '‚', '‛', '“', '”', '„', '‟',
        '«', '»', '‹', '›', '…', '。', '、', '！', '？', '，', '；', '：',
    ])
}

fn alphabetic_word_part(word: &str) -> bool {
    word.chars().count() >= 2 && word.chars().all(char::is_alphabetic)
}

fn lowercase_spelling(word: &str) -> String {
    // Lowercase each scalar so the result is independent of the physical wrap. ~keep
    word.chars().flat_map(char::to_lowercase).collect()
}

fn witnessed_edit_start(former: &str, continuation: &str, witnesses: &SpellingWitnesses) -> Option<usize> {
    let prefix = former.strip_suffix('-')?;
    if !alphabetic_word_part(prefix) || !alphabetic_word_part(continuation) {
        return None;
    }

    let pair = (lowercase_spelling(prefix), lowercase_spelling(continuation));
    if witnesses.compounds.contains(&pair) || should_preserve_lexical_hyphen(prefix, continuation, &witnesses.compounds)
    {
        return Some(former.len());
    }

    let (mut joined, right) = pair;
    joined.push_str(&right);
    witnesses.words.contains(&joined).then_some(prefix.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OcrElementLevel;

    fn scan(text: &str, spans: &[(u32, u32)]) -> (Vec<InternalElement>, Vec<HocrWordOccurrence>) {
        let block = ElementKind::OcrText {
            level: OcrElementLevel::Block,
        };
        let mut words = Vec::new();
        let mut line_start = 0;
        for (row, (line, &(left, right))) in text.lines().zip(spans).enumerate() {
            let mut offset = 0;
            for (column, token) in line.split(' ').enumerate() {
                let width = (right - left) / line.split(' ').count() as u32;
                let x = left + column as u32 * width;
                let y = row as u32 * 33;
                words.push(HocrWordOccurrence {
                    id: words.len(),
                    element: 0,
                    bytes: line_start + offset..line_start + offset + token.len(),
                    bounds: [x, y, x + width, y + 25],
                });
                offset += token.len() + 1;
            }
            line_start += line.len() + 1;
        }
        (vec![InternalElement::text(block, text, 0)], words)
    }

    fn sparse_blocks(text: &str, spans: &[(u32, u32)]) -> (Vec<InternalElement>, Vec<HocrWordOccurrence>) {
        let (_, mut words) = scan(text, spans);
        let mut elements = Vec::new();
        let mut start = 0;
        for line in text.lines() {
            let end = start + line.len();
            for word in words.iter_mut().filter(|word| (start..end).contains(&word.bytes.start)) {
                word.element = elements.len() as u32;
                word.bytes = word.bytes.start - start..word.bytes.end - start;
            }
            let mut block = InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Block,
                },
                line,
                0,
            );
            block.attributes = Some(
                [(
                    crate::ocr::hocr_parser::HOCR_BLOCK_ID_ATTRIBUTE.to_owned(),
                    elements.len().to_string(),
                )]
                .into_iter()
                .collect(),
            );
            elements.push(block);
            start = end + 1;
        }
        (elements, words)
    }

    #[test]
    fn sparse_lines_can_share_a_wrap_without_merging_their_elements() {
        let text = "A passage ends with recog-\nnition and explanatory text\nEarlier recognition supports it";
        let full = (160, 840);
        let (mut elements, words) = sparse_blocks(text, &[full; 3]);
        assert!(!join_scanned_wraps(&mut elements, &words, 1000, false));
        assert!(join_scanned_wraps(&mut elements, &words, 1000, true));
        assert_eq!(elements[0].text, "A passage ends with recognition");
        assert_eq!(elements[1].text, "and explanatory text");
        assert_eq!(elements[2].text, "Earlier recognition supports it");
    }

    #[test]
    fn narrow_sparse_blocks_do_not_licence_a_multiword_wrap() {
        let text = "A passage ends with recog-\nnition and explanatory text\nEarlier recognition supports it";
        let full = (160, 840);
        let (mut elements, words) = sparse_blocks(text, &[full, (160, 420), full]);
        let original = elements.clone();
        assert!(!join_scanned_wraps(&mut elements, &words, 1000, true));
        assert_eq!(elements, original);
    }

    #[test]
    fn digits_and_non_ascii_edges_do_not_license_joins() {
        for (continuation, witness) in [
            ("nition", "recognition2"),
            ("nition", "érecognition"),
            ("nition", "recognition\u{301}"),
            ("nition2", "recognition"),
            ("nitioné", "recognition"),
            ("nition\u{301}", "recognition"),
        ] {
            let text = format!(
                "A passage ends with recog-\n{continuation} and explanatory text\nEarlier {witness} supports it"
            );
            let (mut elements, words) = scan(&text, &[(160, 840); 3]);
            assert!(!join_scanned_wraps(&mut elements, &words, 1000, false), "{text}");
            assert_eq!(elements[0].text, text);
        }
    }

    #[test]
    fn joins_lexical_hyphen_wraps_without_dropping_the_dash() {
        let full = (160, 840);
        for (text, expected) in [
            (
                "This is a well-\nknown comparison with it\nThe bare spelling wellknown occurs here",
                "This is a well-known\ncomparison with it\nThe bare spelling wellknown occurs here",
            ),
            (
                "A project ships the MIT-\nlicensed library with it\nAn MIT-licensed copy is available",
                "A project ships the MIT-licensed\nlibrary with it\nAn MIT-licensed copy is available",
            ),
            (
                "A passage ends with recog-\nnition and explanatory text\nRecognition and recog-nition coexist",
                "A passage ends with recog-nition\nand explanatory text\nRecognition and recog-nition coexist",
            ),
        ] {
            let (mut elements, words) = scan(text, &[full; 3]);
            assert!(join_scanned_wraps(&mut elements, &words, 1000, false), "{text}");
            assert_eq!(elements[0].text, expected);

            let (mut elements, words) = scan(text, &[full, (550, 940), full]);
            assert!(!join_scanned_wraps(&mut elements, &words, 1000, false), "{text}");
            assert_eq!(elements[0].text, text);
        }
    }

    #[test]
    fn hard_hyphen_wraps_keep_their_dash() {
        let full = (160, 840);
        let email = "Send the report by e-\nmail to the office today\nThe email address is listed";
        let (mut elements, words) = scan(email, &[full; 3]);
        assert!(!join_scanned_wraps(&mut elements, &words, 1000, false));
        assert_eq!(elements[0].text, email);

        let unwitnessed = "We hope both groups will co-\noperate on the next step\nThey have agreed already";
        let (mut elements, words) = scan(unwitnessed, &[full; 3]);
        assert!(!join_scanned_wraps(&mut elements, &words, 1000, false));
        assert_eq!(elements[0].text, unwitnessed);

        for (text, expected) in [
            (
                "We hope both groups will co-\noperate on the next step\nThey co-operate already",
                "We hope both groups will co-operate\non the next step\nThey co-operate already",
            ),
            (
                "We hope both groups will co-\noperate on the next step\nThey co-operate and cooperate",
                "We hope both groups will co-operate\non the next step\nThey co-operate and cooperate",
            ),
            (
                "It is a well-\nknown result in the field\nNo other spelling occurs here",
                "It is a well-known\nresult in the field\nNo other spelling occurs here",
            ),
        ] {
            let (mut elements, words) = scan(text, &[full; 3]);
            assert!(join_scanned_wraps(&mut elements, &words, 1000, false), "{text}");
            assert_eq!(elements[0].text, expected);
        }
    }

    #[test]
    fn joins_an_accented_wrap_with_an_independent_case_insensitive_witness() {
        let text = "Le récit conclut sur une ré-\nélection discutée ce soir\nLa «RÉÉLECTION» figure ailleurs";
        let full = (160, 840);
        let (mut elements, words) = scan(text, &[full; 3]);
        assert!(join_scanned_wraps(&mut elements, &words, 1000, false));
        assert_eq!(
            elements[0].text,
            "Le récit conclut sur une réélection\ndiscutée ce soir\nLa «RÉÉLECTION» figure ailleurs"
        );

        let (mut elements, words) = sparse_blocks(text, &[full; 3]);
        assert!(join_scanned_wraps(&mut elements, &words, 1000, true));
        assert_eq!(elements[0].text, "Le récit conclut sur une réélection");
        assert_eq!(elements[1].text, "discutée ce soir");
        assert_eq!(elements[2].text, "La «RÉÉLECTION» figure ailleurs");

        for control in [
            text.replace("«RÉÉLECTION»", "«REELECTION»"),
            text.replace("élection discutée", "Élection discutée"),
            text.replace("ré-", "r-").replace("«RÉÉLECTION»", "«RÉLECTION»"),
            text.replace("ré-", "é-").replace("«RÉÉLECTION»", "«ÉÉLECTION»"),
            text.replace("ré-", "ré\u{2010}"),
            text.replace("ré-", "re\u{301}-"),
        ] {
            let (mut elements, words) = scan(&control, &[full; 3]);
            assert!(!join_scanned_wraps(&mut elements, &words, 1000, false), "{control}");
            assert_eq!(elements[0].text, control);
        }
    }

    #[test]
    fn keeps_an_accented_lexical_hyphen_when_both_spellings_are_witnessed() {
        let text = "Le récit décrit un accord pré-\nétabli avant la réunion\nPRÉ-ÉTABLI et préétabli coexistent";
        let full = (160, 840);
        let (mut elements, words) = scan(text, &[full; 3]);
        assert!(join_scanned_wraps(&mut elements, &words, 1000, false));
        assert_eq!(
            elements[0].text,
            "Le récit décrit un accord pré-établi\navant la réunion\nPRÉ-ÉTABLI et préétabli coexistent"
        );

        let without_compound = text.replace("PRÉ-ÉTABLI", "PRÉALABLE");
        let (mut elements, words) = scan(&without_compound, &[full; 3]);
        assert!(join_scanned_wraps(&mut elements, &words, 1000, false));
        assert_eq!(
            elements[0].text,
            "Le récit décrit un accord préétabli\navant la réunion\nPRÉALABLE et préétabli coexistent"
        );
    }

    #[test]
    fn a_soft_hyphen_is_outside_the_scanned_wrap_policy() {
        let text = "A passage ends with recog\u{00ad}\nnition and explanatory text\nEarlier recognition supports it";
        let (mut elements, words) = scan(text, &[(160, 840); 3]);
        assert!(!join_scanned_wraps(&mut elements, &words, 1000, false));
        assert_eq!(elements[0].text, text);
    }

    #[test]
    fn repairs_only_a_witnessed_terminal_word_within_the_same_block() {
        let line = "The earlier structures support the table struc-";
        let text = format!("{line}\ntures");
        let full = (160, 840);
        let tail = (178, 223);
        let (mut elements, words) = scan(&text, &[full, tail]);
        assert!(join_scanned_wraps(&mut elements, &words, 1000, false));
        assert_eq!(
            elements[0].text,
            "The earlier structures support the table structures\n"
        );

        for (control, spans) in [
            (text.as_str(), [full, (187, 232), full]),
            (text.as_str(), [full, (159, 204), full]),
            (text.as_str(), [(160, 500), tail, full]),
            (
                "The earlier tables explain the method struc-\ntures",
                [full, tail, full],
            ),
            (
                "The earlier structures support the table struc-\ntures\nAnother line",
                [full, tail, full],
            ),
            (
                "An opensource project can also say open-source or open-\nsource",
                [full, tail, full],
            ),
        ] {
            let (mut elements, words) = scan(control, &spans);
            assert!(!join_scanned_wraps(&mut elements, &words, 1000, false), "{control}");
            assert_eq!(elements[0].text, control);
        }
    }

    #[test]
    fn sparse_lines_do_not_consume_an_entire_following_block() {
        let line = "The earlier structures support the table struc-";
        let text = format!("{line}\ntures");
        let (mut elements, mut words) = scan(&text, &[(160, 840), (178, 223)]);
        elements[0].text.truncate(line.len());
        let last = words.last_mut().unwrap();
        last.element = 1;
        last.bytes = 0.."tures".len();
        elements.push(InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Block,
            },
            "tures",
            0,
        ));
        assert!(!join_scanned_wraps(&mut elements, &words, 1000, true));
        assert_eq!([elements[0].text.as_str(), elements[1].text.as_str()], [line, "tures"]);
    }

    #[test]
    fn joins_a_short_paragraph_final_continuation_after_a_full_line() {
        let text = "Earlier recognition grounds the reading\nA passage ends with recog-\nnition of the tables.";
        let full = (160, 840);
        let short = (160, 420);
        let (mut elements, words) = scan(text, &[full, full, short]);
        assert!(join_scanned_wraps(&mut elements, &words, 1000, false));
        assert_eq!(
            elements[0].text,
            "Earlier recognition grounds the reading\nA passage ends with recognition\nof the tables."
        );

        for spans in [
            [full, (160, 520), short],
            [full, full, (190, 450)],
            [full, full, (160, 960)],
        ] {
            let (mut elements, words) = scan(text, &spans);
            assert!(!join_scanned_wraps(&mut elements, &words, 1000, false), "{spans:?}");
            assert_eq!(elements[0].text, text);
        }
    }

    #[test]
    fn uses_a_spelling_witness_with_unicode_surrounding_punctuation() {
        for witness in ["recognition”", "«recognition»", "recognition。"] {
            let text =
                format!("A passage ends with recog-\nnition and explanatory text\nEarlier {witness} supports it");
            let (mut elements, words) = scan(&text, &[(160, 840); 3]);
            assert!(join_scanned_wraps(&mut elements, &words, 1000, false), "{witness}");
            assert_eq!(
                elements[0].text,
                format!("A passage ends with recognition\nand explanatory text\nEarlier {witness} supports it")
            );
        }
    }

    #[test]
    fn joins_only_witnessed_aligned_page_width_wraps() {
        let text = "A passage ends with recog-
nition and explanatory text
Earlier recognition supports it";
        let full = (160, 840);
        let (mut elements, words) = scan(text, &[full, full, full]);
        assert!(join_scanned_wraps(&mut elements, &words, 1000, false));
        assert_eq!(
            elements[0].text,
            "A passage ends with recognition
and explanatory text
Earlier recognition supports it"
        );

        for (control, spans) in [
            (text, [(100, 440), (100, 440), full]),
            (text, [(100, 510), full, full]),
            (text, [full, (550, 940), full]),
            (
                "A passage ends with recog-
nition and explanatory text
No independent witness here",
                [full; 3],
            ),
        ] {
            let (mut elements, words) = scan(control, &spans);
            assert!(!join_scanned_wraps(&mut elements, &words, 1000, false), "{control}");
            assert_eq!(elements[0].text, control);
        }
    }
}

#[cfg(test)]
#[path = "dehyphenation/continuation_tests.rs"]
mod continuation_tests;
