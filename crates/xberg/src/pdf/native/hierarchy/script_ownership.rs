//! Suffix ownership evidence over final preserved native spans.

use std::ops::Range;
use std::sync::Arc;

use xberg_native_pdf::layout::TextSpan;

use crate::types::document_structure::AnnotationKind;

#[derive(Debug, Clone)]
struct NativeSpanWitness {
    original_index: usize,
    snapshot: Arc<TextSpan>,
}

#[derive(Debug, Clone)]
struct NativeScriptOwner {
    base_witness: NativeSpanWitness,
    script_witness: NativeSpanWitness,
    logical_baseline: f32,
    kind: AnnotationKind,
    script_byte_range: Range<usize>,
}

fn collect_suffix_script_owners(_spans: &[TextSpan]) -> Vec<NativeScriptOwner> {
    Vec::new()
}

#[cfg(test)]
mod tests;
