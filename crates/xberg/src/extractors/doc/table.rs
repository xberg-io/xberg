use super::*;

/// The rows of the table being read, until a paragraph outside it ends it.
#[derive(Default)]
pub(super) struct PendingTable {
    rows: Vec<Vec<String>>,
    row: Vec<String>,
    cell: String,
    columns: usize,
    reserved_cells: usize,
    scripts: Vec<crate::rendering::table_scripts::CellScriptAnnotation>,
    /// A row end was seen: this table has the structure of a table, not just text.
    saw_row_end: bool,
    /// Tables nested in this one, innermost last; `nested[0]` sits at depth two.
    nested: Vec<PendingTable>,
    /// Nested tables read so far, emitted ahead of this table.
    finished_nested: Vec<(
        Vec<Vec<String>>,
        Vec<crate::rendering::table_scripts::CellScriptAnnotation>,
    )>,
}

impl PendingTable {
    /// Add one table paragraph to the table at the paragraph's depth.
    ///
    /// A nested table (`mark.depth` > 1) gets its own table on a stack and is
    /// emitted as a separate table once a shallower paragraph (or the end of
    /// the outer table) closes it, because the document model has no
    /// table-in-cell. Depths that skip a level, or a nested table without a
    /// single row end, keep the old behaviour: their text is flattened into the
    /// enclosing cell, so no text is ever dropped.
    pub(super) fn add(
        &mut self,
        paragraph: &DocParagraph,
        mark: DocTableMark,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let depth = usize::from(mark.depth.clamp(1, crate::extraction::doc::MAX_TABLE_DEPTH));
        while self.nested.len() + 1 > depth {
            self.close_innermost(budget)?;
        }
        if depth > self.nested.len() + 2 {
            // The depth jumps a level: flatten into the innermost open cell.
            let flat = DocTableMark {
                cell_end: false,
                row_end: false,
                depth: mark.depth,
            };
            return self.innermost().add_own(paragraph, flat, budget);
        }
        if depth == self.nested.len() + 2 {
            self.nested.push(PendingTable::default());
        }
        self.innermost().add_own(paragraph, mark, budget)
    }

    fn innermost(&mut self) -> &mut PendingTable {
        if self.nested.is_empty() {
            return self;
        }
        let last = self.nested.len() - 1;
        &mut self.nested[last]
    }

    /// Close the innermost nested table into its parent: as a table of its
    /// own when it has rows, else as text in the parent's current cell.
    fn close_innermost(&mut self, budget: &mut SecurityBudget) -> Result<()> {
        let Some(mut table) = self.nested.pop() else {
            return Ok(());
        };
        table.close_nested(budget)?;
        let saw_row_end = table.saw_row_end;
        let parent = self.innermost();
        let (cells, scripts) = table.take_rows(budget)?;
        if saw_row_end {
            parent.finished_nested.append(&mut table.finished_nested);
            if !cells.iter().flatten().all(|cell| cell.trim().is_empty()) {
                parent.finished_nested.push((cells, scripts));
            }
            return Ok(());
        }
        let text = cells
            .iter()
            .flatten()
            .map(|cell| cell.trim())
            .filter(|cell| !cell.is_empty());
        let text = text.collect::<Vec<_>>().join("\n");
        parent.finished_nested.append(&mut table.finished_nested);
        if !text.is_empty() {
            if !parent.cell.is_empty() {
                parent.cell.push('\n');
            }
            parent.cell.push_str(&text);
        }
        Ok(())
    }

    fn close_nested(&mut self, budget: &mut SecurityBudget) -> Result<()> {
        while !self.nested.is_empty() {
            self.close_innermost(budget)?;
        }
        Ok(())
    }

    fn take_rows(
        &mut self,
        budget: &mut SecurityBudget,
    ) -> Result<(
        Vec<Vec<String>>,
        Vec<crate::rendering::table_scripts::CellScriptAnnotation>,
    )> {
        self.end_row(budget)?;
        self.columns = 0;
        self.reserved_cells = 0;
        Ok((std::mem::take(&mut self.rows), std::mem::take(&mut self.scripts)))
    }

    /// Add one paragraph to this table's own cell. A cell's paragraphs are
    /// joined by line breaks.
    fn add_own(&mut self, paragraph: &DocParagraph, mark: DocTableMark, budget: &mut SecurityBudget) -> Result<()> {
        let text = paragraph.content.trim();
        if !text.is_empty() {
            if !self.cell.is_empty() {
                self.cell.push('\n');
            }
            self.add_scripts(paragraph, text);
            self.cell.push_str(text);
        }
        if mark.cell_end {
            self.end_cell(budget)?;
        }
        if mark.row_end {
            self.saw_row_end = true;
            self.end_row(budget)?;
        }
        Ok(())
    }

    fn add_scripts(&mut self, paragraph: &DocParagraph, text: &str) {
        let trimmed_start = paragraph.content.len() - paragraph.content.trim_start().len();
        let trimmed_end = trimmed_start + text.len();
        for annotation in &paragraph.annotations {
            if !matches!(
                annotation.kind,
                crate::types::AnnotationKind::Superscript | crate::types::AnnotationKind::Subscript
            ) {
                continue;
            }
            let start = (annotation.start as usize).max(trimmed_start);
            let end = (annotation.end as usize).min(trimmed_end);
            if start >= end || paragraph.content.get(start..end).is_none() {
                continue;
            }
            let Some(start) = self
                .cell
                .len()
                .checked_add(start - trimmed_start)
                .and_then(|value| u32::try_from(value).ok())
            else {
                continue;
            };
            let Some(end) = self
                .cell
                .len()
                .checked_add(end - trimmed_start)
                .and_then(|value| u32::try_from(value).ok())
            else {
                continue;
            };
            let mut annotation = annotation.clone();
            annotation.start = start;
            annotation.end = end;
            self.scripts
                .push(crate::rendering::table_scripts::CellScriptAnnotation {
                    row: self.rows.len(),
                    col: self.row.len(),
                    annotation,
                });
        }
    }

    fn end_cell(&mut self, budget: &mut SecurityBudget) -> Result<()> {
        let grid = self
            .row
            .len()
            .checked_add(1)
            .map(|columns| columns.max(self.columns))
            .and_then(|columns| {
                self.rows
                    .len()
                    .checked_add(1)?
                    .checked_mul(columns)
                    .map(|cells| (columns, cells))
            });
        let (columns, cells) = grid.ok_or_else(|| crate::XbergError::security("DOC table cell count overflow"))?;
        // Markdown pads every row to the widest one, including rows already read. ~keep
        // Reserve that whole rectangle before allocating another cell. ~keep
        budget.add_cells(cells - self.reserved_cells)?;
        self.columns = columns;
        self.reserved_cells = cells;
        self.row.push(std::mem::take(&mut self.cell));
        Ok(())
    }

    fn end_row(&mut self, budget: &mut SecurityBudget) -> Result<()> {
        if !self.cell.is_empty() {
            self.end_cell(budget)?;
        }
        if !self.row.is_empty() {
            self.rows.push(std::mem::take(&mut self.row));
        }
        Ok(())
    }

    /// Emit the table read so far, unless it holds no text, and start over.
    pub(super) fn flush(&mut self, doc: &mut InternalDocument, budget: &mut SecurityBudget) -> Result<()> {
        self.close_nested(budget)?;
        let (cells, scripts) = self.take_rows(budget)?;
        self.saw_row_end = false;
        for (cells, scripts) in std::mem::take(&mut self.finished_nested) {
            emit_table(doc, cells, scripts);
        }
        if cells.iter().flatten().all(|cell| cell.trim().is_empty()) {
            return Ok(());
        }
        emit_table(doc, cells, scripts);
        Ok(())
    }
}

fn emit_table(
    doc: &mut InternalDocument,
    cells: Vec<Vec<String>>,
    scripts: Vec<crate::rendering::table_scripts::CellScriptAnnotation>,
) {
    let markdown = crate::rendering::common::render_table_markdown(&cells);
    let table_index = doc.push_table(Table {
        cells,
        markdown,
        ..Default::default()
    });
    let mut element = InternalElement::text(ElementKind::Table { table_index }, "", 0);
    if !scripts.is_empty() {
        let encoded =
            serde_json::to_string(&scripts).expect("table script annotations contain only serializable values");
        element.attributes = Some(AHashMap::from_iter([(
            crate::rendering::table_scripts::TABLE_CELL_SCRIPTS_ATTRIBUTE.to_string(),
            encoded,
        )]));
    }
    doc.push_element(element);
}
