---
status: accepted
date: 2026-11-05
decision-makers: ["Na'aman Hirschfeld"]
---

# Bundle regular and bold Latin fallback faces on all targets

## Context and Problem Statement

PDFs may reference unembedded fonts on hosts without a font provider. The regular DejaVu Sans fallback previously
required every character in a text run to be WinAnsi, so one unsupported character replaced the entire run with
boxes. It also painted bold text with the regular face. WASM has the same need, but including the bold font increases
the distributed binary; the existing publish gate limits each WASM file to 51,380,224 bytes (49 MiB).

## Considered Options

* Bundle regular and bold DejaVu Sans on every rendering target
* Keep only the regular face in WASM
* Require callers to supply both fallback fonts

## Decision Outcome

Chosen option: "Bundle regular and bold DejaVu Sans on every rendering target", because a fontless environment
should retain supported glyphs and distinguish bold text consistently without a host font installation. The existing
publish size gate remains the acceptance criterion; this decision does not increase that limit. Large CJK fonts
remain separately controlled by their existing features.

Select the bundled face only after embedded fonts and the supplied provider fail. Use the existing font-weight
classification, including PDF ForceBold bit 19 (one-based, value 262,144), without a separate renderer classifier.
Allow supported glyphs in a mixed run to render; unsupported glyphs keep a visible placeholder or use an available
CJK fallback. This decision does not change the shaping cache or claim that concurrent cold misses parse once.

### Consequences

* Good, because supported Latin characters and mathematical symbols survive mixed runs on native and WASM hosts.
* Good, because font names, descriptor weights and ForceBold select the actual bold outline.
* Bad, because WASM includes the additional DejaVu Sans Bold bytes even when a document does not use bold text.
* Bad, because this fallback does not provide every script or reproduce the absent original font exactly.

### Confirmation

`crates/xberg-native-pdf/src/rendering/latin_font_fallback_tests.rs` compares actual rendered outlines against
embedded fonts with an empty provider, checks provider precedence, and isolates CJK-provider presence and absence
in separate processes. The font-weight tests in `fonts/font_dict.rs` cover the specified ForceBold bit and reject the
adjacent reserved bit. Adobe's ISO 32000-1:2008 section 9.8.2 numbers flags from the low-order bit 1; table 123 and
the following example show ForceBold as bit 19 and `/Flags 262178` for bits 2, 6 and 19.

The full optimized web WASM baseline at `efc3c8915502ac0ce6aa7600565fc156450d941e` is 40,161,684 bytes; the
npm package tarball is 16,534,747 bytes. Both baseline and candidate use the committed web build profile with
wasm-pack 0.15.1, wasm-bindgen 0.2.130 and wasm-opt 133. The candidate at
`8fe9db612a53e8a566a555e0c9676154cddd48c5` is 40,868,073 bytes, an increase of 706,389 bytes; its npm tarball is
16,887,492 bytes, an increase of 352,745 bytes. The WASM file retains 10,512,151 bytes of headroom under the unchanged
gate. Both packaged modules initialize and report one page for the same valid PDF, with identical export names.
`.github/workflows/publish.yaml` checks the unpacked WASM file against `WASM_MAX_FILE_BYTES` and the package
against its existing npm size limit.

The canonical 322 hash-checked PDF inputs produce byte-identical Markdown at main
`9cc98f2899a602df453ceb70f2faf3539e8ec239` and source candidate `a0026cc39aa41ecfe1e56737fc72e0ccf02eef1b`.
The existing 189 golden differences are unchanged; no snapshots were updated.

## More Information

* [Issue 2510](https://github.com/xberg-io/xberg/issues/2510)
* [Adobe ISO 32000-1:2008](https://developer.adobe.com/document-services/docs/assets/35e4369068f86065372c18787171a17e/PDF_ISO_32000-1.pdf)
