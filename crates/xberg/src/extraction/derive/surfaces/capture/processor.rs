use super::{DerivedResult, Fingerprint, RedactionSurfaces, SurfaceContext};
use crate::types::ExtractedDocument;
use std::sync::Arc;

pub(crate) struct ProcessorWitness {
    identity: Arc<()>,
    plain: Fingerprint,
    formatted: Option<Fingerprint>,
}

impl SurfaceContext {
    pub(super) fn pages_match(&self, document: &ExtractedDocument) -> bool {
        self.capture.pages.iter().all(|expected| {
            document
                .pages
                .as_ref()
                .and_then(|pages| pages.get(expected.index))
                .is_some_and(|page| page.page_number == expected.number && expected.rendered.matches(&page.content))
        })
    }
}

impl DerivedResult {
    pub(super) fn begin_processor_mutation(&self) -> Option<ProcessorWitness> {
        if !self.witness_active {
            return None;
        }
        let context = self.context.as_ref()?;
        let identity = self.identity.as_ref()?;
        if !Arc::ptr_eq(identity, &context.identity) || context.validate(&self.document).is_err() {
            return None;
        }
        Some(ProcessorWitness {
            identity: Arc::clone(identity),
            plain: context.plain_document.clone(),
            formatted: context.capture.formatted.clone(),
        })
    }

    pub(super) fn finish_processor_mutation(&mut self, witness: Option<ProcessorWitness>) {
        let Some(witness) = witness else {
            return;
        };
        if !self.witness_active
            || !self
                .identity
                .as_ref()
                .is_some_and(|identity| Arc::ptr_eq(identity, &witness.identity))
        {
            return;
        }
        let Some(context) = &mut self.context else {
            return;
        };
        if !Arc::ptr_eq(&context.identity, &witness.identity)
            || context.plain_document != witness.plain
            || context.capture.formatted != witness.formatted
            || witness.plain.matches(&self.document.content)
            || !context.pages_match(&self.document)
        {
            return;
        }
        // Only the actual invocation can advance authoritative plain text; its rendering never regains trust. ~keep
        if context.formatted_markdown() {
            context.formatted_invalidated = true;
            context.capture.formatted = self.document.formatted_content.as_deref().map(Fingerprint::new);
        }
        context.plain_document = Fingerprint::new(&self.document.content);
    }
}
