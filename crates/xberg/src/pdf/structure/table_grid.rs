#[cfg(any(feature = "layout-detection", test))]
use crate::types::Table;
#[cfg(test)]
use crate::types::TableGrid;
#[cfg(test)]
use crate::types::internal::table_grid::retained_grid;
pub(crate) use crate::types::internal::table_grid::{TableGridSidecar, attach_table_grids};
#[cfg(test)]
use crate::types::internal::{ElementKind, InternalDocument};

#[cfg(feature = "layout-detection")]
#[derive(Default)]
pub(crate) struct RecognizedNativeTables {
    pub(crate) tables: Vec<Table>,
    pub(crate) grids: Vec<TableGridSidecar>,
}

#[cfg(feature = "layout-detection")]
impl From<Vec<Table>> for RecognizedNativeTables {
    fn from(tables: Vec<Table>) -> Self {
        Self { tables, grids: vec![] }
    }
}

#[cfg(test)]
#[path = "table_grid/tests.rs"]
mod tests;
