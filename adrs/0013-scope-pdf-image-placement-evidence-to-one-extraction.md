---
status: accepted
date: 2026-11-06
decision-makers: ["Na'aman Hirschfeld"]
---

# Scope PDF image placement evidence to one extraction

## Context and Problem Statement

The native PDF extractor can verify which image was painted, its position, and whether later paint could cover it. The shared embedded-image OCR stage runs after native extraction and receives images whose public fields alone cannot prove those properties. An additive OCR read needs the native proof and must preserve the original nested OCR result.

## Considered Options

* Attach placement evidence to a public image field or serialized internal document payload.
* Pass a typed placement proof within the selected extraction operation and consume it in the embedded-image fold.

## Decision Outcome

Chosen option: "Pass a typed placement proof within the selected extraction operation and consume it in the embedded-image fold", because paint authority belongs to the actual native extraction and must survive its blocking handoff without becoming a cache, plug-in wire, or public API contract.

The blocking native phase returns evidence for a bounded single-page audit with one final opaque RGB image. The selected async Native extractor publishes it into a scope for that invocation. The pipeline receives it once and requires the matching page, image index, shared byte allocation, and unique OCR text anchor. A disabled or nested scope cannot write into an enclosing scope. Multiple writes discard the proof.

The first consumer requires cache-off, explicit PSM 6 and the same built-in Tesseract backend instance for both reads. It inserts one nonoverlapping line into the document and page fold only when exact literal occurrences and both anchor lines are accounted for. The nested image OCR remains the primary result. Unsupported paint, placement, projection, redaction, backend, or OCR evidence retains the ordinary result.

### Consequences

* Good, because placement authority cannot enter through an image DTO, cached extraction, or serialized plug-in result.
* Good, because unsupported evidence can be discarded without changing either the ordinary document or the original nested OCR.
* Bad, because other page shapes and rendering paths need their own source qualification.

### Confirmation

The public `pdf_embedded_psm6_addition_2327` tests check bytes and URI extraction, exact document and page text, unchanged nested JSON, and refusal of alternate PSM, later paint, repeated paint, explicit Decode, placeholders, and cache-enabled extraction. The `pdf_gap` unit tests check isolated scopes, ambiguous writes, and forced document or page OCR.

## More Information

[ADR 0008](0008-scope-cacheability-to-one-extraction-operation.md) established operation-local state for cacheability. Placement evidence uses its own narrower lifetime and does not participate in the cacheability protocol.
