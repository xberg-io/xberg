use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::ops::Range;

pub(super) struct SourceRanges {
    code_blocks: Vec<Range<usize>>,
    code_text: Vec<Range<usize>>,
    boundary_comments: Vec<Range<usize>>,
}

impl SourceRanges {
    pub(super) fn new(markdown: &str) -> Self {
        let mut ranges = Self {
            code_blocks: Vec::new(),
            code_text: Vec::new(),
            boundary_comments: Vec::new(),
        };
        let mut in_code_block = false;
        for (event, range) in Parser::new(markdown).into_offset_iter() {
            match event {
                Event::Start(Tag::CodeBlock(_)) => {
                    ranges.code_blocks.push(range);
                    in_code_block = true;
                }
                Event::End(TagEnd::CodeBlock) => in_code_block = false,
                Event::Text(_) if in_code_block => ranges.code_text.push(range),
                Event::Html(html) | Event::InlineHtml(html) if html.trim() == "<!-- end list -->" => {
                    ranges.boundary_comments.push(range);
                }
                _ => {}
            }
        }
        ranges
    }

    pub(super) fn code_block(&self, line: &Range<usize>) -> Option<&Range<usize>> {
        self.code_blocks.iter().find(|block| overlaps(block, line))
    }

    pub(super) fn is_code_text(&self, line: &Range<usize>) -> bool {
        self.code_text.iter().any(|text| overlaps(text, line))
    }

    pub(super) fn is_boundary_comment(&self, line: &Range<usize>) -> bool {
        self.boundary_comments.iter().any(|comment| overlaps(comment, line))
    }
}

fn overlaps(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}
