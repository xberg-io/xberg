use super::*;

/// Where a nested table sits: the table it is in and the cell that holds it.
#[derive(serde::Serialize)]
struct NestedParent {
    table: u32,
    row: usize,
    col: usize,
}

/// A nested table read alongside its flattened text in the outer cell.
struct NestedRecord {
    /// Index into the records of the table that holds it; `None` for the outer table.
    parent: Option<usize>,
    row: usize,
    col: usize,
    cells: Vec<Vec<String>>,
    scripts: Vec<crate::rendering::table_scripts::CellScriptAnnotation>,
    /// A row end was seen: the paragraphs had the structure of a table.
    structured: bool,
}

/// The rows of the table being read, until a paragraph outside it ends it.
#[derive(Default)]
pub(super) struct PendingTable {
    rows: Vec<Vec<String>>,
    row: Vec<String>,
    cell: String,
    columns: usize,
    reserved_cells: usize,
    scripts: Vec<crate::rendering::table_scripts::CellScriptAnnotation>,
    saw_row_end: bool,
    /// Open nested tables, innermost last, each with its record's index.
    stack: Vec<(PendingTable, usize)>,
    records: Vec<NestedRecord>,
}

impl PendingTable {
    /// Add one table paragraph.
    ///
    /// Every paragraph's text goes into the outer table's cell, nested or not,
    /// so a cell keeps the words and position it has always had. A nested
    /// paragraph (`mark.depth` > 1) is also read into a stack of tables by depth;
    /// each closed one is emitted as a table of its own after the outer table.
    /// A depth that skips a level or a nested table without a row end has no
    /// structure to emit, and its text is already in the cell.
    pub(super) fn add(
        &mut self,
        paragraph: &DocParagraph,
        mark: DocTableMark,
        budget: &mut SecurityBudget,
    ) -> Result<()> {
        let depth = usize::from(mark.depth.clamp(1, crate::extraction::doc::MAX_TABLE_DEPTH));
        if depth == 1 {
            self.close_nested(budget)?;
            return self.add_own(paragraph, mark, budget);
        }
        let flat = DocTableMark {
            cell_end: false,
            row_end: false,
            depth: mark.depth,
        };
        self.add_own(paragraph, flat, budget)?;
        while self.stack.len() + 1 > depth {
            self.close_innermost(budget)?;
        }
        if depth > self.stack.len() + 2 {
            return Ok(());
        }
        if depth == self.stack.len() + 2 {
            self.open_nested();
        }
        let Some((table, _)) = self.stack.last_mut() else {
            return Ok(());
        };
        table.add_own(paragraph, mark, budget)
    }

    fn open_nested(&mut self) {
        let (parent, row, col) = match self.stack.last() {
            Some((table, index)) => (Some(*index), table.rows.len(), table.row.len()),
            None => (None, self.rows.len(), self.row.len()),
        };
        self.records.push(NestedRecord {
            parent,
            row,
            col,
            cells: Vec::new(),
            scripts: Vec::new(),
            structured: false,
        });
        self.stack.push((PendingTable::default(), self.records.len() - 1));
    }

    fn close_innermost(&mut self, budget: &mut SecurityBudget) -> Result<()> {
        let Some((mut table, index)) = self.stack.pop() else {
            return Ok(());
        };
        let structured = table.saw_row_end;
        let (cells, scripts) = table.take_rows(budget)?;
        let record = &mut self.records[index];
        record.structured = structured;
        record.cells = cells;
        record.scripts = scripts;
        Ok(())
    }

    fn close_nested(&mut self, budget: &mut SecurityBudget) -> Result<()> {
        while !self.stack.is_empty() {
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

    /// Emit the table read so far, unless it holds no text, then the tables
    /// nested in it in the order they opened, and start over.
    pub(super) fn flush(&mut self, doc: &mut InternalDocument, budget: &mut SecurityBudget) -> Result<()> {
        self.close_nested(budget)?;
        let (cells, scripts) = self.take_rows(budget)?;
        self.saw_row_end = false;
        let mut records = std::mem::take(&mut self.records);
        let blank = |cells: &[Vec<String>]| cells.iter().flatten().all(|cell| cell.trim().is_empty());
        let outer = (!blank(&cells)).then(|| emit_table(doc, cells, scripts, None));
        let mut emitted: Vec<Option<u32>> = Vec::with_capacity(records.len());
        for i in 0..records.len() {
            if !records[i].structured || blank(&records[i].cells) {
                emitted.push(None);
                continue;
            }
            let (mut parent, mut row, mut col) = (records[i].parent, records[i].row, records[i].col);
            while let Some(p) = parent.filter(|&p| emitted[p].is_none()) {
                (parent, row, col) = (records[p].parent, records[p].row, records[p].col);
            }
            let table = parent.map_or(outer, |p| emitted[p]);
            let tag = table.map(|table| NestedParent { table, row, col });
            let cells = std::mem::take(&mut records[i].cells);
            let scripts = std::mem::take(&mut records[i].scripts);
            emitted.push(Some(emit_table(doc, cells, scripts, tag)));
        }
        Ok(())
    }
}

fn emit_table(
    doc: &mut InternalDocument,
    cells: Vec<Vec<String>>,
    scripts: Vec<crate::rendering::table_scripts::CellScriptAnnotation>,
    parent: Option<NestedParent>,
) -> u32 {
    let markdown = crate::rendering::common::render_table_markdown(&cells);
    let table_index = doc.push_table(Table {
        cells,
        markdown,
        ..Default::default()
    });
    let mut element = InternalElement::text(ElementKind::Table { table_index }, "", 0);
    let mut attributes = AHashMap::new();
    if !scripts.is_empty() {
        let encoded =
            serde_json::to_string(&scripts).expect("table script annotations contain only serializable values");
        attributes.insert(
            crate::rendering::table_scripts::TABLE_CELL_SCRIPTS_ATTRIBUTE.to_string(),
            encoded,
        );
    }
    if let Some(parent) = parent {
        let encoded = serde_json::to_string(&parent).expect("a nested table parent is serializable");
        attributes.insert(
            crate::types::internal::DOC_NESTED_TABLE_PARENT_ATTRIBUTE.to_string(),
            encoded,
        );
    }
    if !attributes.is_empty() {
        element.attributes = Some(attributes);
    }
    doc.push_element(element);
    table_index
}
