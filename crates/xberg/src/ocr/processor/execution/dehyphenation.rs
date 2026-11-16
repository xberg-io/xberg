use crate::ocr::hocr_parser::HocrWordOccurrence;
use crate::pdf::structure::pipeline::{HyphenWitnesses, should_preserve_lexical_hyphen};
use crate::types::internal::{ElementKind, InternalElement};
use std::collections::HashSet;
use std::ops::Range;

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

pub(super) fn join_scanned_wraps(
    elements: &mut [InternalElement],
    words: &[HocrWordOccurrence],
    page_width: u32,
    sparse: bool,
) -> bool {
    if !elements.iter().any(|element| {
        element.text.contains("-\n")
            || element.text.contains("\u{00ad}\n")
            || (sparse && element.text.ends_with(['-', '\u{00ad}']))
    }) {
        return false;
    }
    let page = elements.first().and_then(|element| element.page);
    if elements.iter().any(|element| element.page != page) {
        return false;
    }
    let Some(lines) = physical_lines(elements, words) else {
        return false;
    };
    let spellings: HashSet<String> = words
        .iter()
        .filter_map(|word| word_text(elements, word))
        .map(|word| lexical_core(word).to_ascii_lowercase())
        .collect();
    let compounds = spellings
        .iter()
        .filter_map(|word| word.split_once('-'))
        .filter(|(left, right)| ascii_word_part(left) && ascii_word_part(right))
        .map(|(left, right)| (left.to_owned(), right.to_owned()))
        .collect();
    let witnesses = SpellingWitnesses {
        words: spellings,
        compounds,
    };

    let joins: Vec<_> = lines
        .windows(2)
        .filter_map(|pair| witnessed_join(&pair[0], &pair[1], elements, &witnesses, page_width, sparse))
        .collect();
    let changed = !joins.is_empty();
    for join in joins.into_iter().rev() {
        if let Some(leading) = join.leading {
            leading.apply(elements);
        }
        join.trailing.apply(elements);
    }
    changed
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
    prior: &Line<'_>,
    next: &Line<'_>,
    elements: &[InternalElement],
    witnesses: &SpellingWitnesses,
    page_width: u32,
    sparse: bool,
) -> Option<Join> {
    let former_element = elements.get(prior.element)?;
    let latter_element = elements.get(next.element)?;
    if next.words.len() < 2
        || former_element.page != latter_element.page
        || !can_edit(former_element)
        || !can_edit(latter_element)
    {
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
    if !latter.as_bytes().first().is_some_and(u8::is_ascii_lowercase) {
        return None;
    }
    let edit_start = witnessed_edit_start(former, lexical_core(latter), witnesses)?;
    same_full_width_body(prior, next, page_width)?;
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
    let end = next.words.get(1)?.bytes.start;
    if latter_element.text.get(leading.bytes.end..end)? != " " {
        return None;
    }
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

fn same_full_width_body(prior: &Line<'_>, next: &Line<'_>, page_width: u32) -> Option<()> {
    let before = LineBounds::from_words(prior.words)?;
    let after = LineBounds::from_words(next.words)?;
    (prior.words.last()?.bounds[2] == before.right
        && next.words.first()?.bounds[0] == after.left
        && before.full_width(page_width)
        && after.full_width(page_width)
        && after.follows(before)
        && before.right.abs_diff(after.right) <= before.height().max(after.height()).saturating_mul(2))
    .then_some(())
}

fn lexical_core(word: &str) -> &str {
    word.trim_matches(['.', ',', ';', ':', '!', '?', '"', '\'', '(', ')', '[', ']', '{', '}'])
}

fn ascii_word_part(word: &str) -> bool {
    word.len() >= 2 && word.bytes().all(|byte| byte.is_ascii_alphabetic())
}

fn witnessed_edit_start(former: &str, continuation: &str, witnesses: &SpellingWitnesses) -> Option<usize> {
    let prefix = former.strip_suffix(['-', '\u{00ad}'])?;
    if !ascii_word_part(prefix) || !ascii_word_part(continuation) {
        return None;
    }
    if should_preserve_lexical_hyphen(prefix, continuation, &witnesses.compounds) {
        return former.ends_with('-').then_some(former.len());
    }

    let prefix_lower = prefix.to_ascii_lowercase();
    let continuation_lower = continuation.to_ascii_lowercase();
    witnesses
        .words
        .contains(&format!("{prefix_lower}{continuation_lower}"))
        .then_some(prefix.len())
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

    #[test]
    fn sparse_lines_can_share_a_wrap_without_merging_their_elements() {
        let text = "A passage ends with recog-\nnition and explanatory text\nEarlier recognition supports it";
        let full = (160, 840);
        let (_, mut words) = scan(text, &[full; 3]);
        let mut elements = Vec::new();
        let mut start = 0;
        for line in text.lines() {
            let end = start + line.len();
            for word in words.iter_mut().filter(|word| (start..end).contains(&word.bytes.start)) {
                word.element = elements.len() as u32;
                word.bytes = word.bytes.start - start..word.bytes.end - start;
            }
            elements.push(InternalElement::text(
                ElementKind::OcrText {
                    level: OcrElementLevel::Block,
                },
                line,
                0,
            ));
            start = end + 1;
        }
        assert!(!join_scanned_wraps(&mut elements, &words, 1000, false));
        assert!(join_scanned_wraps(&mut elements, &words, 1000, true));
        assert_eq!(elements[0].text, "A passage ends with recognition");
        assert_eq!(elements[1].text, "and explanatory text");
        assert_eq!(elements[2].text, "Earlier recognition supports it");
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
