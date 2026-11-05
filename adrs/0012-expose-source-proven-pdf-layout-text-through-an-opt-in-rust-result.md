---
status: accepted
date: 2026-11-05
decision-makers: ["Na'aman Hirschfeld"]
---

# Expose source-proven PDF layout text through an opt-in Rust result

## Context and Problem Statement

Layout regions describe geometry and class. Adding text to the existing region struct would break exhaustive Rust literals, and populating an existing serialized field by default would change the extraction wire contract. A region can also cross a redaction boundary, so independent filtering of region strings cannot establish that a fragment is safe to publish.

## Considered Options

* Add an optional text field to the existing layout region type.
* Attach a region index to ordinary extraction results or to their cache entries.
* Return a separate source-proven region projection from the explicit native PDF extraction operation.

## Decision Outcome

Chosen option: "Return a separate source-proven region projection from the explicit native PDF extraction operation", because this preserves the existing public and wire shapes and reuses an operation that already bypasses both extraction caches.

The opt-in request collects ranges from the same pass that assembles native page text in reading order. The result includes a region only when the source spans have exclusive ownership, the range is independently readable, and the final same-page text still matches the source fingerprint. Requests with configured or scoped redaction omit this projection.

### Consequences

* Good, because existing callers can continue to construct and serialize the original layout region type.
* Good, because the projection can be omitted on uncertain ownership or transformed text without discarding the ordinary extraction.
* Bad, because image and OCR paths, rotated or cropped PDF pages, and language bindings need their own qualified support.

### Confirmation

The external Rust compatibility test constructs the original exhaustive literal and checks its exact JSON. Native PDF tests exercise exact selected-page reading order. A selected-page Engine test carries the original chart class, and controls reject ambiguous geometry, invalid UTF-8 boundaries, different final pages and redaction.

## More Information

* [Issue #2357](https://github.com/xberg-io/xberg/issues/2357)
