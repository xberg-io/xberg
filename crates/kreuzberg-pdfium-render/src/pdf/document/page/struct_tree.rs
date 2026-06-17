//! Defines the [PdfStructTree] struct, exposing the structure tree of a PDF page.

use crate::bindgen::FPDF_STRUCTTREE;
use crate::bindings::PdfiumLibraryBindings;
use crate::pdf::document::page::struct_element::PdfStructElement;
use std::os::raw::c_int;

/// The structure tree of a PDF page, providing semantic document structure
/// for tagged PDFs.
///
/// Elements in the tree have types like P (paragraph), H1-H6 (headings),
/// Table, Figure, etc. The tree must be closed when it is no longer needed;
/// this is handled automatically via the `Drop` implementation.
///
/// The tree handle is owned by this struct and will be released when it
/// goes out of scope.
pub struct PdfStructTree<'a> {
    tree_handle: FPDF_STRUCTTREE,
    bindings: &'a dyn PdfiumLibraryBindings,
}

impl<'a> PdfStructTree<'a> {
    pub(crate) fn from_pdfium(tree_handle: FPDF_STRUCTTREE, bindings: &'a dyn PdfiumLibraryBindings) -> Self {
        Self { tree_handle, bindings }
    }

    /// Returns the number of top-level children in this structure tree.
    pub fn children_count(&self) -> usize {
        let count = self.bindings.FPDF_StructTree_CountChildren(self.tree_handle);
        if count < 0 { 0 } else { count as usize }
    }

    /// Returns the top-level child element at the given index, or `None` on error
    /// or out-of-bounds index.
    pub fn child_at_index(&self, index: usize) -> Option<PdfStructElement<'_>> {
        let handle = self
            .bindings
            .FPDF_StructTree_GetChildAtIndex(self.tree_handle, index as c_int);
        if handle.is_null() {
            None
        } else {
            Some(PdfStructElement::from_pdfium(handle, self.bindings))
        }
    }

    /// Returns an iterator over the top-level children of this structure tree.
    pub fn children(&self) -> PdfStructTreeChildrenIterator<'_> {
        PdfStructTreeChildrenIterator {
            tree: self,
            count: self.children_count(),
            index: 0,
        }
    }

    /// Returns a depth-first iterator over all elements in the structure tree.
    ///
    /// Each item is a tuple of `(element, depth)` where depth starts at 0 for
    /// top-level (root) elements and increases by 1 for each level of nesting.
    pub fn iter(&self) -> PdfStructTreeIterator<'_> {
        // Initialize stack with root children in reverse order so that the
        // first child is popped first (depth-first, left-to-right traversal).
        let mut stack = Vec::new();
        let count = self.children_count();
        for i in (0..count).rev() {
            if let Some(child) = self.child_at_index(i) {
                stack.push((child, 0usize));
            }
        }
        PdfStructTreeIterator { stack }
    }
}

impl Drop for PdfStructTree<'_> {
    fn drop(&mut self) {
        self.bindings.FPDF_StructTree_Close(self.tree_handle);
    }
}

/// An iterator over the top-level children of a [PdfStructTree].
pub struct PdfStructTreeChildrenIterator<'a> {
    tree: &'a PdfStructTree<'a>,
    count: usize,
    index: usize,
}

impl<'a> Iterator for PdfStructTreeChildrenIterator<'a> {
    type Item = PdfStructElement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.count {
            let current = self.index;
            self.index += 1;
            if let Some(child) = self.tree.child_at_index(current) {
                return Some(child);
            }
        }
        None
    }
}

/// A depth-first iterator over all elements in a [PdfStructTree].
///
/// Each item yielded is a tuple of `(PdfStructElement, depth)` where depth
/// starts at 0 for root-level elements and increases for nested elements.
pub struct PdfStructTreeIterator<'a> {
    stack: Vec<(PdfStructElement<'a>, usize)>,
}

impl<'a> Iterator for PdfStructTreeIterator<'a> {
    type Item = (PdfStructElement<'a>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let (element, depth) = self.stack.pop()?;

        // Push children in reverse order so the first child is processed next.
        let child_count = element.children_count();
        for i in (0..child_count).rev() {
            if let Some(child) = element.child_at_index(i) {
                self.stack.push((child, depth + 1));
            }
        }

        Some((element, depth))
    }
}
