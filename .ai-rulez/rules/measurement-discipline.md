---
type: Decision
title: Measurement Discipline
paths:
  - "tools/benchmark-harness/**"
  - "tools/ocr-measure/**"
  - "tools/perf/**"
  - "crates/xberg/src/ocr/**"
x-ai-rulez:
  kind: rule
  id: measurement-discipline
  metadata:
    priority: high
---

Quality comparisons decide whether fixes ship. Three errors have each produced a confidently wrong
conclusion in this codebase; all three are cheap to avoid.

**Never judge quality by a ratio over output whose length the change itself altered.** A ratio rises
whenever the dropped material was the low-quality part, so a change that deletes real content can
look like an improvement. Judge by the absolute count of the good thing — valid words, correct
rows, passing cases — not by its share. A measured example: scoring an OCR change by
dictionary-valid-word *ratio* gave a 8-8 tie across 16 documents; scoring the same outputs by
absolute valid-word *count* gave 15-1 against the change, at −22.8%. One document read 406 → 225
valid words while its ratio rose 0.9355 → 0.9657.

**A sample selected by largest delta is the tail, not the population.** Reporting its magnitude as
typical overstates the effect, often by an order of magnitude. Characterise the distribution —
median, count beyond a threshold, and how many cases moved the *other* way — before quoting any
single number. The same change can be net-positive overall and severely damaging to a small tail;
both facts belong in the report, and a lone headline number will misrepresent one of them.

**Attribute a change to a specific commit by bracketing preserved binaries**, not by commit
ordering. Build or keep one binary per candidate revision and run them against the same input; a
revision whose output is byte-identical to its predecessor did not cause the change. Preserve those
binaries outside any shared build tree before rebuilding — a peer wiping `target/` turns a
measurement back into an inference.

State the limits of the metric alongside the result. A reference-free proxy is not grounded truth;
say which one was used, and say when no ground truth exists for the population being measured
rather than letting a proxy stand in silently.
