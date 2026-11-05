---
status: accepted
date: 2026-11-05
decision-makers: ["Na'aman Hirschfeld"]
---

# Preserve certified painted PDF cells when text has decorative paint

## Context and Problem Statement

Some untagged PDFs draw the full backgrounds of a table's cells, then draw narrower backgrounds and underlines around the text. Using every painted rectangle as a separate grid boundary can divide one logical row into unrelated fragments. In the hash-pinned Nougat exhibit index on PDF page 91, this changes fourteen source rows into a partial thirty-four-row candidate, which the existing real-grid filter rejects. The full cell paint, independently of the text paint, identifies the original rectangular tiling.

## Considered Options

* Keep interpreting all painted rectangles as equivalent grid boundaries.
* Relax table admission for sparse or wrapped rows.
* Remove subordinate text paint only after certifying the complete painted grid and verifying its extracted source occurrences.

## Decision Outcome

Chosen option: "Remove subordinate text paint only after certifying the complete painted grid and verifying its extracted source occurrences", because the source supplies stronger boundary evidence than text gaps or the fragmented detector candidate.

The private Native detector may attempt an alternate path list for a complete, recurring, contrasting, nonoverlapping cell tiling. Only single-line inset fills and text-matched underlines inside a uniquely owned cell may be excluded. Ambiguous paint, layer or transform data, strokes, unmatched words, incomplete tilings, and exhausted work budgets keep the original path. The existing detector constructs the proposed table. Adoption requires the exact certified row and column shape and original per-cell text and geometry occurrences, with no change to the visible non-whitespace characters.

A cross-crate ownership certificate is available only for an adopted painted grid and an exact decimal word whose font box extends beyond the upper edge within the existing row tolerance. The certificate keeps the original word and painted cell bounds and is kept separate from serialized table data. Core must prove an exact projected source run and a unique residual digit; it may suppress only an isolated unstyled duplicate after paragraph grouping.

### Consequences

* Good, because source cells retain wrapped text without weakening grid or prose admission for other PDFs.
* Good, because the extraction can prove cell membership after the existing detector's snapping and row processing.
* Bad, because this deliberately does not recover arbitrary nested tables or column headings outside the painted tiling.
* Bad, because qualifying pages need a bounded preliminary scan, and a rejected proposal may run the original detector afterward.

### Confirmation

`crates/xberg-native-pdf/src/structure/spatial_table_detector/filled_cells_tests.rs` covers preservation against the undecorated source grid, rejection of uncertain paint and geometry, source occurrence substitution, visible text loss, and the counted duplicate-paint limit. The hash-pinned Nougat and PLOS public input qualification checks the recovered fourteen records, source span inventory, and plot rejection. The Core public test pins the exact source digit and neighboring page content, and the assembly controls cover annotations and other paragraph roles. No snapshots are refreshed to satisfy this decision.
