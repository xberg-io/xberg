//! Tracked-deletion filtering for piece characters. ~keep

use super::{CELL_MARK, FIELD_BEGIN, FIELD_END, FIELD_SEPARATOR, PieceTableContext};

impl PieceTableContext<'_> {
    /// Whether a character stays in the text. A tracked deletion drops it, except for field
    /// markers, which keep begin/separator/end balanced so a half-deleted field cannot leak its
    /// instruction as text, and for the end mark of a table row that still has surviving
    /// content, which keeps the next row from running into it.
    pub(super) fn keeps(&self, character: char, fc: u32, deleted: bool) -> bool {
        let in_table = self.lists.table_for_paragraph_end(fc);
        if character == CELL_MARK && in_table.is_some_and(|table| table.row_end) {
            let keep = !deleted || self.row_has_content.get();
            self.row_has_content.set(false);
            return keep;
        }
        let keep = !deleted || matches!(character, FIELD_BEGIN | FIELD_SEPARATOR | FIELD_END);
        if keep && in_table.is_some() {
            self.row_has_content.set(true);
        }
        keep
    }
}
