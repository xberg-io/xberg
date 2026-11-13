use super::super::document::{apply_ocr_page_replacements, boundaries_after_replacements};

pub(in crate::extractors::pdf) fn render_mixed_plain_pages(
    method: crate::types::ExtractionMethod,
    text: &mut String,
    boundaries: &[crate::types::PageBoundary],
    replacements: &mut ahash::AHashMap<u32, String>,
    documents: &ahash::AHashMap<u32, crate::types::internal::InternalDocument>,
    format: &crate::core::config::OutputFormat,
) {
    if method != crate::types::ExtractionMethod::Mixed || *format != crate::core::config::OutputFormat::Plain {
        return;
    }
    let adjusted = boundaries_after_replacements(boundaries, replacements);
    let mut rendered = ahash::AHashMap::new();
    for boundary in &adjusted {
        let page = boundary.page_number;
        let (Some(accepted), Some(document)) = (replacements.get(&page), documents.get(&page)) else {
            continue;
        };
        if document.tables.is_empty() {
            continue;
        }
        let candidate = (|| {
            if text.get(boundary.byte_start..boundary.byte_end) != Some(accepted.as_str()) {
                return Err("page range differs from the accepted transcript");
            }
            reconcile_mixed_plain_page(accepted, document, page)
        })();
        match candidate {
            Ok(plain) => {
                rendered.insert(page, plain);
            }
            // ~keep The page keeps its Markdown transcript in Plain output, so the decline is a
            // degraded result, not routine flow.
            Err(reason) => tracing::warn!(page, reason, "Declined mixed OCR Plain page rendering"),
        }
    }
    if !rendered.is_empty() {
        *text = apply_ocr_page_replacements(text, &adjusted, &rendered);
        replacements.extend(rendered);
    }
}

fn reconcile_mixed_plain_page(
    accepted: &str,
    document: &crate::types::internal::InternalDocument,
    page: u32,
) -> Result<String, &'static str> {
    let (plain, spans, indices) = crate::extraction::derive::render_plain_page_elements(document, page)
        .filter(|(plain, _, _)| !plain.is_empty())
        .ok_or("page elements produced no Plain text")?;
    let writers: Vec<_> = indices
        .into_iter()
        .zip(spans.by_element)
        .filter_map(|(index, span)| {
            let (start, end) = span?;
            Some(PlainWriter {
                start,
                end: end.min(plain.len()),
                element: &document.elements[index],
            })
        })
        .collect();
    let retained = check_accepted_ocr_tokens(accepted, &plain, document, &writers)?;
    let repeated = retained.repeated_for_page(&writers)?;
    if repeated.is_empty() {
        return Ok(plain);
    }
    let filtered = without_repeated_words(&plain, &repeated);
    check_accepted_ocr_tokens(accepted, &filtered, document, &[])?;
    Ok(filtered)
}

struct PlainWriter<'a> {
    start: usize,
    end: usize,
    element: &'a crate::types::internal::InternalElement,
}

impl PlainWriter<'_> {
    fn is_prose(&self) -> bool {
        use crate::types::internal::ElementKind;
        matches!(self.element.kind, ElementKind::Paragraph | ElementKind::Heading { .. })
    }

    fn bounds(&self) -> Option<crate::types::BoundingBox> {
        self.element.bbox.filter(valid_plain_source_bounds)
    }
}

fn valid_plain_source_bounds(bbox: &crate::types::BoundingBox) -> bool {
    bbox.x0 < bbox.x1 && bbox.y0 < bbox.y1 && [bbox.x0, bbox.y0, bbox.x1, bbox.y1].into_iter().all(f64::is_finite)
}

fn plain_source_key(bbox: crate::types::BoundingBox) -> [u64; 4] {
    [bbox.x0, bbox.y0, bbox.x1, bbox.y1].map(f64::to_bits)
}

fn plain_source_contains(outer: crate::types::BoundingBox, inner: crate::types::BoundingBox) -> bool {
    outer.x0 <= inner.x0 && inner.x1 <= outer.x1 && outer.y0 <= inner.y0 && inner.y1 <= outer.y1
}

fn check_accepted_ocr_tokens<'a>(
    accepted: &'a str,
    plain: &'a str,
    document: &crate::types::internal::InternalDocument,
    writers: &[PlainWriter<'_>],
) -> Result<PlainWords<'a>, &'static str> {
    use crate::types::internal::ElementKind;
    use pulldown_cmark::{Event, Options, Parser, Tag};

    // ~keep PDF table assembly may suppress a paragraph covered by cell tokens, or keep one it
    // failed to match to a table. Require the accepted token order, including punctuation,
    // before replacing its transcript: no word dropped, and none added unless the transcript
    // lacks it (see `PlainWords`).
    let mut tables = document.elements.iter().filter_map(|element| match element.kind {
        ElementKind::Table { table_index } => document
            .tables
            .get(table_index as usize)
            .map(|table| (table_index, table)),
        _ => None,
    });
    let mut retained = PlainWords::new(plain, accepted);
    let mut events = Parser::new_ext(accepted, Options::ENABLE_TABLES).into_offset_iter();
    let mut offset = 0;
    let mut depth = 0;
    while let Some((event, range)) = events.next() {
        match event {
            Event::Start(tag)
                if depth == 0
                    && matches!(
                        tag,
                        Tag::Paragraph | Tag::Heading { .. } | Tag::List(_) | Tag::BlockQuote(_) | Tag::Table(_)
                    ) =>
            {
                if !retained.retains(&accepted[offset..range.start], PROSE) {
                    return Err(PROSE);
                }
                offset = range.end;
                if !matches!(tag, Tag::Table(_)) {
                    let mut block_depth = 1;
                    let block = events.by_ref().take_while(|(event, _)| {
                        match event {
                            Event::Start(_) => block_depth += 1,
                            Event::End(_) => block_depth -= 1,
                            _ => {}
                        }
                        block_depth > 0
                    });
                    let is_paragraph = matches!(tag, Tag::Paragraph);
                    let first_match = retained.matched_prose.len();
                    if is_paragraph && retained.retains_exact_prose_writer(&accepted[range.clone()], writers) {
                        block.for_each(drop);
                    } else {
                        if !retains_inline_block(accepted, range, &tag, block, &mut retained) {
                            return Err(PROSE);
                        }
                        retained.preserve_ambiguous_prose(first_match, writers);
                    }
                    continue;
                }

                let (index, table) = tables.next().ok_or("accepted table has no typed table element")?;
                if !source_table_has_cells(&accepted[range], &table.cells)
                    || !retained.retains_table(index, table, writers)
                {
                    return Err(CELLS);
                }
                depth += 1;
            }
            Event::Start(_) => depth += 1,
            Event::End(_) => depth -= 1,
            _ => {}
        }
    }
    if !retained.retains(&accepted[offset..], PROSE) {
        return Err(PROSE);
    }
    Ok(retained)
}

const PROSE: &str = "Plain page omitted accepted prose";
const CELLS: &str = "Plain page omitted or changed accepted table cells";

/// The Plain words read in the accepted order. A word out of that order is skipped and noted.
/// A decline keeps the transcript, so it loses every skipped word the transcript lacks: such a
/// word is page text that the elements carry and the transcript dropped (GH#3046). A page that
/// adds only words the transcript already holds repeats them, and is declined (GH#2990).
struct PlainWords<'a> {
    plain: &'a str,
    words: std::str::SplitWhitespace<'a>,
    accepted_runs: ahash::AHashSet<&'a str>,
    prose_runs: ahash::AHashSet<&'a str>,
    matched_prose: Vec<&'a str>,
    ambiguous_prose_writers: ahash::AHashSet<usize>,
    table_substitutions: ahash::AHashSet<u32>,
    unowned_table_words: ahash::AHashSet<usize>,
    first_skip: Option<&'static str>,
    skipped: Vec<&'a str>,
}

impl<'a> PlainWords<'a> {
    fn new(plain: &'a str, accepted: &'a str) -> Self {
        Self {
            plain,
            words: plain.split_whitespace(),
            accepted_runs: Self::runs(accepted).collect(),
            prose_runs: ahash::AHashSet::new(),
            matched_prose: Vec::new(),
            ambiguous_prose_writers: ahash::AHashSet::new(),
            table_substitutions: ahash::AHashSet::new(),
            unowned_table_words: ahash::AHashSet::new(),
            first_skip: None,
            skipped: Vec::new(),
        }
    }

    /// The letter and digit runs of `text`. A mark such as a heading marker carries none.
    fn runs(text: &str) -> impl Iterator<Item = &str> {
        text.split(|character: char| !character.is_alphanumeric())
            .filter(|run| !run.is_empty())
    }

    /// Whether the Plain words hold the words of `text` in order. `reason` names the check
    /// that a skipped word would fail.
    fn retains(&mut self, text: &str, reason: &'static str) -> bool {
        text.split_whitespace().all(|word| {
            while let Some(candidate) = self.words.next() {
                if candidate == word {
                    self.record_prose_match(candidate);
                    return true;
                }
                self.skip(candidate, reason);
            }
            false
        })
    }

    fn retains_exact_prose_writer(&mut self, text: &str, writers: &[PlainWriter<'_>]) -> bool {
        let Some(next) = self.words.clone().next() else {
            return false;
        };
        let start = word_offset(self.plain, next);
        // ~keep Exact paragraph matching disambiguates nearby captions. The ordered token
        // matcher is the fallback when a distant writer would require an unbounded scan.
        const EXACT_PROSE_WRITER_LIMIT: usize = 8;
        let mut candidates = writers[writers.partition_point(|writer| writer.end <= start)..]
            .iter()
            .take(EXACT_PROSE_WRITER_LIMIT)
            .take_while(|writer| writer.is_prose() && writer.bounds().is_some());
        let Some(first) = candidates.next() else { return false };
        if first.start > start || !self.plain[first.start..start].trim().is_empty() {
            return false;
        }
        if self.plain[start..first.end]
            .split_whitespace()
            .eq(text.split_whitespace())
        {
            return self.retains(text, PROSE);
        }
        // ~keep An earlier bounded caption can share the first word of the accepted paragraph.
        let Some(writer) = candidates.find(|writer| {
            self.plain[writer.start..writer.end]
                .split_whitespace()
                .eq(text.split_whitespace())
        }) else {
            return false;
        };
        while let Some(word) = self.words.clone().next() {
            if word_offset(self.plain, word) >= writer.start {
                break;
            }
            self.words.next();
            self.skip(word, PROSE);
        }
        self.retains(text, PROSE)
    }

    fn preserve_ambiguous_prose(&mut self, first_match: usize, writers: &[PlainWriter<'_>]) {
        let plain = self.plain;
        let writer_at = |word: &&str| {
            let offset = word_offset(plain, word);
            let index = writers.partition_point(|writer| writer.end <= offset);
            writers
                .get(index)
                .filter(|writer| writer.start <= offset)
                .map(|_| index)
        };
        let matched = &self.matched_prose[first_match..];
        let (Some(first_word), Some(last_word)) = (matched.first(), matched.last()) else {
            return;
        };
        let (Some(first), Some(last)) = (writer_at(first_word), writer_at(last_word)) else {
            return;
        };
        if first == last
            && !self.reuses_a_skipped_occurrence(matched, word_offset(plain, first_word)..word_offset(plain, last_word))
        {
            return;
        }
        // ~keep A scattered match establishes order without proving which occurrences it owns.
        self.ambiguous_prose_writers.extend(
            matched
                .iter()
                .filter_map(writer_at)
                .map(|index| &writers[index])
                .filter(|writer| !matches!(writer.element.kind, crate::types::internal::ElementKind::Table { .. }))
                .map(|writer| writer.start),
        );
    }

    fn reuses_a_skipped_occurrence(&self, matched: &[&str], range: std::ops::Range<usize>) -> bool {
        let offset = |word: &&str| word_offset(self.plain, word);
        let left = self.skipped.partition_point(|word| offset(word) < range.start);
        let right = self.skipped.partition_point(|word| offset(word) < range.end);
        if left == right {
            return false;
        }
        let matched: ahash::AHashSet<_> = matched.iter().copied().collect();
        self.skipped[left..right].iter().any(|word| matched.contains(word))
    }

    fn preserve_remaining_ambiguous_prose(&mut self, writers: &[PlainWriter<'_>]) {
        let mut matched = self.matched_prose.as_slice();
        for writer in writers {
            let start = matched.partition_point(|word| word_offset(self.plain, word) < writer.start);
            let end = matched.partition_point(|word| word_offset(self.plain, word) < writer.end);
            let writer_matches = &matched[start..end];
            matched = &matched[end..];
            if !writer_matches.is_empty()
                && !matches!(writer.element.kind, crate::types::internal::ElementKind::Table { .. })
                && !self.ambiguous_prose_writers.contains(&writer.start)
                && self.reuses_a_skipped_occurrence(writer_matches, writer.start..writer.end)
            {
                self.ambiguous_prose_writers.insert(writer.start);
            }
        }
    }

    fn record_prose_match(&mut self, word: &'a str) {
        self.accepted_runs.extend(Self::runs(word));
        self.prose_runs.extend(Self::runs(word));
        self.matched_prose.push(word);
    }

    fn retains_table(&mut self, index: u32, source: &crate::types::Table, writers: &[PlainWriter<'_>]) -> bool {
        let rendered = crate::rendering::common::render_table_plain(&source.cells);
        let table = rendered.trim();
        let word_count = table.split_whitespace().count();
        if word_count == 0 {
            return true;
        }
        while let Some(candidate) = self.words.next() {
            let start = word_offset(self.plain, candidate);
            let prefix = self.plain[..start].trim_end_matches(|c: char| c != '\n' && c.is_whitespace());
            if (prefix.is_empty() || prefix.ends_with('\n')) && self.table_matches(index, source, table, start, writers)
            {
                self.accepted_runs.extend(Self::runs(candidate));
                self.accepted_runs
                    .extend(self.words.by_ref().take(word_count - 1).flat_map(Self::runs));
                return true;
            }
            self.skip(candidate, CELLS);
        }
        false
    }

    fn table_matches(
        &mut self,
        index: u32,
        source: &crate::types::Table,
        table: &str,
        start: usize,
        writers: &[PlainWriter<'_>],
    ) -> bool {
        use crate::types::internal::ElementKind;
        if writers.is_empty() {
            return matches_plain_rows(table, &self.plain[start..]);
        }
        let Some(writer) = writers.get(writers.partition_point(|writer| writer.end <= start)) else {
            return false;
        };
        if writer.start > start || !matches_plain_rows(table, &self.plain[start..writer.end]) {
            return false;
        }
        if let ElementKind::Table { table_index } = writer.element.kind {
            return table_index == index;
        }
        if !writer.is_prose() {
            return false;
        }
        let embedded = !self.ambiguous_prose_writers.contains(&writer.start)
            && self
                .matched_prose
                .last()
                .is_some_and(|word| (writer.start..start).contains(&word_offset(self.plain, word)));
        let shares_source = writer
            .bounds()
            .zip(source.bounding_box.filter(valid_plain_source_bounds))
            .is_some_and(|(prose, grid)| {
                plain_source_contains(grid, prose) || (embedded && plain_source_contains(prose, grid))
            });
        if shares_source {
            self.table_substitutions.insert(index);
        } else {
            // ~keep Keep both occurrences when the equal prose row cannot be attributed to the table.
            self.unowned_table_words.extend(
                self.plain[start..writer.end]
                    .split_whitespace()
                    .take(table.split_whitespace().count())
                    .map(|word| word_offset(self.plain, word)),
            );
        }
        shares_source
    }

    fn skip(&mut self, word: &'a str, reason: &'static str) {
        self.first_skip.get_or_insert(reason);
        self.skipped.push(word);
    }

    fn repeated_for_page(mut self, writers: &[PlainWriter<'_>]) -> Result<Vec<usize>, &'static str> {
        while let Some(word) = self.words.next() {
            self.skip(word, "Plain page added text absent from the accepted transcript");
        }
        let Some(reason) = self.first_skip else {
            return Ok(Vec::new());
        };
        // ~keep A later block or the end of the page can reveal repeats in a matched writer.
        self.preserve_remaining_ambiguous_prose(writers);
        let mut repeated = self.repeated_word_offsets();
        if !writers.is_empty() {
            repeated = self.reconcile_writers(writers, &repeated)?;
        }
        if repeated.len() == self.skipped.len() {
            return Err(reason);
        }
        Ok(repeated)
    }

    fn reconcile_writers(&self, writers: &[PlainWriter<'_>], repeated: &[usize]) -> Result<Vec<usize>, &'static str> {
        use crate::types::internal::ElementKind;
        let mut captions = ahash::AHashSet::new();
        let source_prose = self.matched_prose_by_source(writers);
        let mut adjusted = Vec::new();
        let mut remaining = repeated;
        for writer in writers {
            let before = remaining.partition_point(|&offset| offset < writer.start);
            adjusted.extend_from_slice(&remaining[..before]);
            let after = remaining.partition_point(|&offset| offset < writer.end);
            let repeats = &remaining[before..after];
            remaining = &remaining[after..];
            let written = &self.plain[writer.start..writer.end];
            if self.ambiguous_prose_writers.contains(&writer.start) {
                if writer.is_prose()
                    && let Some(bbox) = writer.bounds()
                {
                    captions.insert((written.trim(), plain_source_key(bbox)));
                }
                continue;
            }
            let offsets = || written.split_whitespace().map(|word| word_offset(self.plain, word));

            if let ElementKind::Table { table_index } = writer.element.kind {
                if !repeats.is_empty()
                    && (!self.table_substitutions.contains(&table_index) || offsets().count() != repeats.len())
                {
                    return Err(CELLS);
                }
            } else if writer.is_prose() {
                let skipped = self
                    .skipped
                    .partition_point(|word| word_offset(self.plain, word) < writer.start);
                let all_skipped = offsets().eq(self.skipped[skipped..]
                    .iter()
                    .map(|word| word_offset(self.plain, word))
                    .take(offsets().count()));
                if let Some(bbox) = writer.bounds() {
                    let source = plain_source_key(bbox);
                    if !captions.insert((written.trim(), source)) {
                        if !all_skipped {
                            return Err(PROSE);
                        }
                        adjusted.extend(offsets());
                        continue;
                    }
                    if all_skipped {
                        // ~keep A distinct source can reuse every word of accepted prose.
                        // Remove shared words only when the accepted prose has these bounds.
                        if let Some(prose) = source_prose.get(&source) {
                            adjusted.extend(written.split_whitespace().filter_map(|word| {
                                let mut runs = Self::runs(word).peekable();
                                (runs.peek().is_some() && runs.all(|run| prose.contains(run)))
                                    .then_some(word_offset(self.plain, word))
                            }));
                        }
                        continue;
                    }
                }
            }
            adjusted.extend(
                repeats
                    .iter()
                    .copied()
                    .filter(|offset| !self.unowned_table_words.contains(offset)),
            );
        }
        adjusted.extend_from_slice(remaining);
        Ok(adjusted)
    }

    fn matched_prose_by_source(
        &self,
        writers: &[PlainWriter<'_>],
    ) -> ahash::AHashMap<[u64; 4], ahash::AHashSet<&'a str>> {
        let mut sources = ahash::AHashMap::<_, ahash::AHashSet<_>>::new();
        let mut matched = self.matched_prose.iter().copied().peekable();
        for writer in writers {
            let offset = |word| word_offset(self.plain, word);
            while matched.next_if(|word| offset(word) < writer.start).is_some() {}
            let words = std::iter::from_fn(|| matched.next_if(|word| offset(word) < writer.end));
            if !self.ambiguous_prose_writers.contains(&writer.start)
                && let Some(bbox) = writer.bounds()
            {
                sources
                    .entry(plain_source_key(bbox))
                    .or_default()
                    .extend(words.flat_map(Self::runs));
            }
        }
        sources
    }

    fn repeated_word_offsets(&self) -> Vec<usize> {
        let is_known = |word| Self::runs(word).all(|run| self.accepted_runs.contains(run));
        let mut repeated = Vec::new();
        let mut skipped = self.skipped.iter().copied().peekable();
        for line in self.plain.split_inclusive('\n') {
            let end = word_offset(self.plain, line) + line.len();
            let Some(word) = skipped.peek() else { break };
            if word_offset(self.plain, word) >= end {
                continue;
            }
            // ~keep A word shared with a table cell can still be part of a recovered caption.
            let has_new_word = line.split_whitespace().any(|word| !is_known(word));
            while let Some(word) = skipped.next_if(|word| word_offset(self.plain, word) < end) {
                if is_known(word) && (!has_new_word || Self::runs(word).all(|run| self.prose_runs.contains(run))) {
                    repeated.push(word_offset(self.plain, word));
                }
            }
        }
        repeated
    }
}

fn matches_plain_rows(table: &str, plain: &str) -> bool {
    let mut lines = plain.lines();
    table.lines().all(|expected| {
        lines
            .next()
            .is_some_and(|line| line.split_whitespace().eq(expected.split_whitespace()))
    })
}

fn word_offset(plain: &str, word: &str) -> usize {
    word.as_ptr() as usize - plain.as_ptr() as usize
}

/// Drop only skipped repeats. Untouched lines keep their spacing and accepted table assets
/// remain in the document. The caller checks the resulting transcript again.
fn without_repeated_words(plain: &str, repeated: &[usize]) -> String {
    let mut blocks = Vec::new();
    for block in plain.split("\n\n") {
        let mut lines = Vec::new();
        for line in block.split('\n') {
            let mut words = line.split_whitespace();
            let is_repeat = |word: &str| repeated.binary_search(&word_offset(plain, word)).is_ok();
            if words.any(is_repeat) {
                let kept = line
                    .split_whitespace()
                    .filter(|word| !is_repeat(word))
                    .collect::<Vec<_>>();
                if !kept.is_empty() {
                    lines.push(std::borrow::Cow::Owned(kept.join(" ")));
                }
            } else {
                lines.push(std::borrow::Cow::Borrowed(line));
            }
        }
        if !lines.is_empty() {
            blocks.push(lines.join("\n"));
        }
    }
    blocks.join("\n\n")
}

fn retains_inline_block<'a>(
    source: &'a str,
    block: std::ops::Range<usize>,
    tag: &pulldown_cmark::Tag<'_>,
    events: impl Iterator<Item = (pulldown_cmark::Event<'a>, std::ops::Range<usize>)>,
    retained: &mut PlainWords<'_>,
) -> bool {
    use pulldown_cmark::{CowStr, Event, Tag};

    // ~keep Keep punctuation adjacent to inline markup in the same group, and
    // keep whitespace inside a code span or link label from splitting its group.
    let mut inline = Vec::new();
    let mut leaves = Vec::new();
    let mut markers = Vec::new();
    record_block_markers(source, tag, &block, &mut markers);
    let mut literal_depth = 0;
    for (event, range) in events {
        if literal_depth > 0 {
            match event {
                Event::Start(_) => literal_depth += 1,
                Event::End(_) => literal_depth -= 1,
                _ => {}
            }
            continue;
        }
        let value = match event {
            Event::Start(Tag::Emphasis | Tag::Strong | Tag::Link { .. } | Tag::Image { .. }) => {
                inline.push(range);
                continue;
            }
            Event::Code(text) | Event::InlineHtml(text) => {
                inline.push(range.clone());
                Some(text)
            }
            Event::Text(text) if text.as_ref() == &source[range.clone()] => None,
            Event::Text(text) => {
                inline.push(range.clone());
                Some(text)
            }
            Event::HardBreak => {
                if source[range.clone()].contains('\\') {
                    inline.push(range.clone());
                }
                Some(CowStr::Borrowed(" "))
            }
            Event::SoftBreak => Some(CowStr::Borrowed(" ")),
            Event::Start(tag @ (Tag::Heading { .. } | Tag::Item | Tag::BlockQuote(_))) => {
                record_block_markers(source, &tag, &range, &mut markers);
                continue;
            }
            Event::Start(Tag::Paragraph | Tag::List(_)) | Event::End(_) => continue,
            Event::Start(_) => {
                // ~keep Keep this child verbatim while allowing its siblings to project.
                literal_depth = 1;
                None
            }
            _ => None,
        };
        leaves.push((range, value));
    }

    markers.sort_unstable();
    let mut inline = inline.into_iter().peekable();
    let mut leaves = leaves.into_iter().peekable();
    let mut inline_end = block.start;
    let mut start = block.start;
    let mut visible = String::new();
    for (relative, separator) in source[block.clone()]
        .match_indices(char::is_whitespace)
        .chain([(block.len(), "")])
    {
        let offset = block.start + relative;
        while inline.peek().is_some_and(|range| range.start < offset) {
            if let Some(range) = inline.next() {
                inline_end = inline_end.max(range.end);
            }
        }
        if offset >= inline_end {
            project_inline_group(source, start..offset, &mut leaves, &mut visible);
            let is_marker = markers.binary_search(&start).is_ok();
            if !retains_inline_group(&source[start..offset], &visible, is_marker, retained) {
                return false;
            }
            start = offset + separator.len();
        }
    }
    true
}

fn project_inline_group<'a>(
    source: &str,
    group: std::ops::Range<usize>,
    leaves: &mut std::iter::Peekable<
        impl Iterator<Item = (std::ops::Range<usize>, Option<pulldown_cmark::CowStr<'a>>)>,
    >,
    visible: &mut String,
) {
    visible.clear();
    while let Some((range, value)) = leaves.peek() {
        if range.start >= group.end {
            break;
        }
        if let Some(value) = value {
            if group.start <= range.start && range.end <= group.end {
                visible.push_str(value);
            }
        } else {
            let start = group.start.max(range.start);
            let end = group.end.min(range.end);
            if start < end {
                visible.push_str(&source[start..end]);
            }
        }
        if range.end > group.end {
            break;
        }
        leaves.next();
    }
}

fn retains_inline_group(source: &str, visible: &str, is_marker: bool, retained: &mut PlainWords<'_>) -> bool {
    // ~keep Reference definitions and other unrepresented text can lie inside a block.
    // An empty event projection only accounts for recognized block markers.
    if source == visible || (visible.is_empty() && !is_marker) {
        return retained.retains(source, PROSE);
    }

    let mut literal = source.split_whitespace().peekable();
    let mut projected = visible.split_whitespace().peekable();
    if projected.peek().is_none() {
        // ~keep A block marker projects to nothing, but a Plain element may still spell it.
        let mut spelled = retained.words.clone();
        if source.split_whitespace().all(|word| spelled.next() == Some(word)) {
            retained.words = spelled;
        }
        return true;
    }
    // ~keep Advance both alternatives together; once the group starts, each Plain word must
    // continue one of them, so a word the accepted group lacks fails the group instead of being
    // skipped. A word before the group starts is an extra word like any other.
    let (mut literal_live, mut projected_live) = (true, true);
    let mut started = false;
    while let Some(candidate) = retained.words.next() {
        if !started && literal.peek() != Some(&candidate) && projected.peek() != Some(&candidate) {
            retained.skip(candidate, PROSE);
            continue;
        }
        started = true;
        literal_live &= literal.next_if_eq(&candidate).is_some();
        projected_live &= projected.next_if_eq(&candidate).is_some();
        retained.record_prose_match(candidate);
        if literal_live && literal.peek().is_none() || projected_live && projected.peek().is_none() {
            return true;
        }
        if !literal_live && !projected_live {
            return false;
        }
    }
    false
}

fn record_block_markers(
    source: &str,
    tag: &pulldown_cmark::Tag<'_>,
    range: &std::ops::Range<usize>,
    markers: &mut Vec<usize>,
) {
    use pulldown_cmark::Tag;

    let block = &source[range.clone()];
    if matches!(tag, Tag::BlockQuote(_)) {
        // ~keep The quote range includes its continuation lines. Only the leading
        // quote token within CommonMark's three-column indentation can be syntax.
        const MAX_MARKER_INDENT: usize = 3;
        let opening_column = source[..range.start].rsplit('\n').next().map_or(0, str::len);
        let mut offset = range.start;
        for line in block.split_inclusive('\n') {
            let content = line.trim_start_matches(' ');
            let indent = line.len() - content.len();
            if indent <= opening_column + MAX_MARKER_INDENT && content.split_whitespace().next() == Some(">") {
                markers.push(offset + indent);
            }
            offset += line.len();
        }
        return;
    }
    let start = block.trim_start();
    let first = start.split_whitespace().next().unwrap_or("");
    let leading_marker = match tag {
        Tag::Heading { .. } => !first.is_empty() && first.bytes().all(|byte| byte == b'#'),
        Tag::Item => match first.as_bytes() {
            [b'+' | b'*' | b'-'] => true,
            [digits @ .., b'.' | b')'] => (1..=9).contains(&digits.len()) && digits.iter().all(u8::is_ascii_digit),
            _ => false,
        },
        _ => false,
    };
    if leading_marker {
        markers.push(range.start + block.len() - start.len());
    }
    if matches!(tag, Tag::Heading { .. }) {
        let end = block.trim_end();
        let last = end.split_whitespace().next_back().unwrap_or("");
        if let [marker @ (b'#' | b'=' | b'-'), rest @ ..] = last.as_bytes()
            && rest.iter().all(|byte| byte == marker)
        {
            markers.push(range.start + end.len() - last.len());
        }
    }
}

fn source_table_has_cells(source: &str, cells: &[Vec<String>]) -> bool {
    // ~keep Verify every source cell before relying on the typed table. GFM can omit excess
    // cells from its events, and a source <br> can represent either a line break or literal text.
    // ~keep The shared writer copies existing backslashes when escaping a cell pipe.
    // Its full serialization proves those cells even when the pipe has even parity.
    if source.trim() == crate::rendering::common::render_table_markdown(cells).trim() {
        return true;
    }
    let mut rows = source.lines().enumerate().filter(|(index, _)| *index != 1);
    let columns = cells.iter().map(Vec::len).max().unwrap_or(0);
    for row in cells {
        let Some((_, source_row)) = rows.next() else {
            return false;
        };
        let mut remainder = source_row.trim();
        remainder = remainder.strip_prefix('|').unwrap_or(remainder);
        for column in 0..columns {
            let delimiter = remainder.match_indices('|').find(|(index, _)| {
                remainder.as_bytes()[..*index]
                    .iter()
                    .rev()
                    .take_while(|&&byte| byte == b'\\')
                    .count()
                    .is_multiple_of(2)
            });
            let (source_cell, rest) = match delimiter {
                Some((index, _)) => (&remainder[..index], &remainder[index + 1..]),
                None => (remainder, ""),
            };
            let mut encoded = String::new();
            if let Some(cell) = row.get(column) {
                crate::rendering::common::push_escaped_cell(&mut encoded, cell);
            }
            if source_cell.trim() != encoded.trim() {
                return false;
            }
            remainder = rest;
        }
        if !remainder.trim().is_empty() {
            return false;
        }
    }
    rows.next().is_none()
}

#[cfg(test)]
#[path = "inline_group_tests.rs"]
mod inline_group_tests;
