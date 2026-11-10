---
status: accepted
date: 2026-11-10
decision-makers: ["Na'aman Hirschfeld"]
---

# Gate Khmer numeric recovery on locally provisioned models

## Context and Problem Statement

An English OCR read can omit a number whose native PDF layer claims Khmer digits. That layer can also contain false Unicode mappings, so copying its number without independent evidence invents text. Deployments differ in which Tesseract language files they provision; adding an auxiliary download or changing the primary language would change their existing model policy.

## Considered Options

* Add a public recovery flag, disabled by default.
* Enable bounded recovery automatically when the selected English-primary directory already contains the auxiliary Khmer model.
* Copy native numbers without independent source verification.

## Decision Outcome

Chosen option: "Enable bounded recovery automatically when the selected English-primary directory already contains the auxiliary Khmer model", because local provisioning supplies the requested capability without a new public configuration field, a language-policy change, or an implicit model download.

Activation is automatic, not a separate opt-in. A deployment that ships both `eng.traineddata` and `khm.traineddata` in its selected primary directory can receive different output and additional verification work. An English-primary deployment without that Khmer file retains its ordinary OCR text and receives a page-specific warning after an otherwise eligible primary read is accepted. Other primary languages, including `deu`, `khm`, and `eng+khm`, are outside this recovery feature; their ordinary OCR selection is unchanged.

The initial scope is the built-in Tesseract backend, automatic whole-document native PDF OCR, and Plain or Markdown output without layout, custom backend options, explicit pipelines, redaction, or forced OCR. Detached, transformed and nonzero-offset stages, uncertain raster integrity, Type0 ink loss, tables/formulas, ambiguous line ownership and rows already containing numbers decline recovery.

A candidate must agree with the Khmer model's independent source-crop read and additional glyph-shape evidence. Shape agreement does not authenticate a ToUnicode value: an unchanged glyph with a false value still needs to fail the recognition comparison. Recovery inserts an unstyled numeric segment while preserving existing OCR text, including erroneous neighboring tokens and multiline paragraph bytes. Recovered Plain page strings use the finalized document rendering; a missing final page preserves its earlier read.

Auxiliary execution uses only the selected fixed model directory. If its model disappears, verification fails closed instead of resolving another root, downloading or materializing a replacement. The primary OCR resolver retains its existing behavior. A lazy verifier and its bounded native pool belong to one OCR run; glyph/crop work remains bounded and serial. The selected-read binding has a 256 KiB serialized text/geometry limit; inability to establish that proof retains the accepted read and reports the verification gap.

Eligible native-number pages bypass extraction-result caching, including when the Khmer model is absent. When the auxiliary capability is present, the primary read also bypasses OCR caching so a cached result cannot pretend to prove preprocessing geometry. Installing a model can therefore change both output and cache behavior. This deliberately conservative policy extends the operation-scoped cacheability rule in ADR 0008.

### Consequences

* Good, because an already provisioned deployment can recover independently corroborated numbers without changing its configured primary OCR language.
* Good, because absent or disappearing auxiliary files cannot trigger a recovery download or model-root fallback.
* Good, because unqualified primary languages and uncertain evidence retain the ordinary read.
* Bad, because all-model deployments automatically incur bounded extra work and lose cache hits on eligible pages.
* Bad, because insertion-only recovery leaves misread OCR neighbors visible and cannot authenticate every possible font or scan.
* Bad, because unsupported glyph profiles, geometry and proof limits leave some numbers unrecovered. The original reporter page is unavailable, so this is a partial capability improvement rather than closure of issue 2408.

### Confirmation

`mise run rust:test:khmer-number-recovery` verifies the pinned, locally provisioned Khmer model and requires every named public, recovery-unit and glyph-shape control to be collected and executed with zero ignored tests. Its controls include the genuine Khmer positive, painted Latin look-alikes, false ToUnicode values, missing and disappearing models, unsupported primary languages, accepted-only warning deduplication, retained multiline text, page-render fallback and exact Plain page ranges. Both pipeline recovery sites share one policy helper. The main Rust CI qualification action provisions the licensed pinned test model and invokes this task; the separate focused PR suite is not evidence that the model-backed qualification executed remotely.

## More Information

* [Issue 2408](https://github.com/xberg-io/xberg/issues/2408)
* [ADR 0008](0008-scope-cacheability-to-one-extraction-operation.md)
