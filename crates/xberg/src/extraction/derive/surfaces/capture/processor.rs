use super::{DerivedResult, Fingerprint, RedactionSurfaces, SurfaceContext};
use crate::types::ExtractedDocument;
use std::sync::Arc;

pub(crate) struct ProcessorWitness {
    identity: Arc<()>,
    plain: Fingerprint,
    formatted: Option<Fingerprint>,
}

impl SurfaceContext {
    pub(super) fn page_shape_matches(&self, document: &ExtractedDocument) -> bool {
        match (&self.page_numbers, &document.pages) {
            (None, None) => true,
            (Some(expected), Some(pages)) => {
                expected.len() == pages.len()
                    && expected
                        .iter()
                        .zip(pages)
                        .all(|(number, page)| *number == page.page_number)
            }
            _ => false,
        }
    }

    fn observe_fields(&mut self, document: &ExtractedDocument, witness: &ProcessorWitness) {
        let content_changed = !witness.plain.matches(&document.content);
        let formatted = document.formatted_content.as_deref().map(Fingerprint::new);
        if content_changed {
            // The established changed-content preference does not attempt to merge conflicting rendered views. ~keep
            self.prefer_current_plain = true;
            self.formatted_literal = None;
        }
        if (content_changed || formatted != witness.formatted) && self.formatted_markdown() {
            self.formatted_invalidated = true;
            self.capture.formatted = formatted;
            self.capture.owned_tags = None;
            self.capture.plain_annotation_separator = None;
            if !self.prefer_current_plain {
                self.formatted_literal = document.formatted_content.clone();
            }
        }
        for expected in &mut self.capture.pages {
            let page = &document.pages.as_ref().expect("page shape was validated")[expected.index];
            if !expected.rendered.matches(&page.content) {
                expected.rendered = Fingerprint::new(&page.content);
                expected.plain = Some(super::PlainPageBackup(page.content.clone()));
                expected.literal = true;
                expected.tag_exemptions = false;
                expected.owned_tags = None;
            }
        }
        self.plain_document = Fingerprint::new(&document.content);
    }

    pub(super) fn pages_match(&self, document: &ExtractedDocument) -> bool {
        self.page_shape_matches(document)
            && self.capture.pages.iter().all(|expected| {
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
        // Observation cannot turn an originally untrusted rendering into literal authority. ~keep
        let untrusted = (context.capture.formatted.is_some() && !context.capture.tag_exemptions)
            || context
                .capture
                .pages
                .iter()
                .any(|page| !page.literal && !page.tag_exemptions);
        if untrusted || !Arc::ptr_eq(identity, &context.identity) || context.validate(&self.document).is_err() {
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
        {
            return;
        }
        if let Some(markup) = &context.capture.markup {
            // Verified markup witnesses record a change without advancing the original preimage. ~keep
            context.markup_mutated |= !context.markup_preimage_matches(&self.document, markup);
            return;
        }
        if !context.page_shape_matches(&self.document) {
            context.processor_changed_page_shape = true;
            return;
        }
        // Only a validated actual invocation can advance current fields. ~keep
        // Edited rendering never regains tag authority. ~keep
        context.observe_fields(&self.document, &witness);
    }
}
