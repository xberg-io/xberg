# License, overlap and leakage audit — Federal Register corpus

Manifest: `manifest.json` (408 documents, digest
`a963a4fe2d94fd6989456a50d3e6f276b2859ab72c8502cf2bdd7dc4025ac175`).

## 1. License

- **What is used:** the document metadata the Office of the Federal Register publishes (document
  number, type, title, publication date, issuing agencies) and the plain-text rendering of each
  document served at `raw_text_url`.
- **Status:** works of the U.S. Government prepared by officers or employees of the United States
  are not subject to copyright within the United States (17 U.S.C. §105). The Federal Register is
  published by the Office of the Federal Register / U.S. Government Publishing Office.
- **Redistribution:** the corpus bytes are public-domain U.S. Government works. They are still not
  committed: the repository stores only digests, and the acquisition script re-fetches the exact
  documents from the pinned URLs.
- **Third-party material inside a document:** a Federal Register document may quote or reproduce
  third-party text. Quotation inside a government work does not make the quoted work public
  domain. The corpus is used for model evaluation only and is not redistributed, so this is
  recorded as a residual risk rather than a cleared right.
- **No dataset license is inherited.** The corpus is not derived from a licensed dataset; there is
  no upstream annotation license to comply with.

## 2. Training overlap

The candidate is `xberg-io/classification-models@a29f2096…`, a copy of
`cross-encoder/nli-MiniLM2-L6-H768@b95119ce…`, whose base is
`nreimers/MiniLMv2-L6-H768-distilled-from-RoBERTa-Large`. That model family was trained no later
than 2021.

- **Temporal separation:** every document in this corpus was published between 2026-09-01 and
  2026-11-20. A 2021 training corpus cannot contain a 2026 Federal Register document.
- **This is a temporal argument, not a provenance audit.** It cannot exclude a model that was
  retrained later, or a derivative that scraped more recent text. The owned revision is a copy of
  the 2021 cross-encoder, and the inventory records the upstream revision that was copied.
- **No document or excerpt is known to be in the candidate's training data,** and the temporal
  argument is the reason, not an inspection of that data.

## 3. Label leakage

The gold label comes from the Federal Register's own `type` metadata, never from text, filename,
path or agency. Leakage was then audited in the frozen text:

| check | result |
| --- | --- |
| documents whose retained text still contains the metadata banner `[Rules]`, `[Proposed Rules]`, `[Notices]`, `[Presidential Documents]` | 0 / 408 |
| documents whose retained text still contains an `ACTION:` metadata line | 0 / 408 |
| exact text duplicates across splits | 0 (checked before and after token truncation) |
| families spanning two splits | 0 (61 families, 24 / 12 / 25) |

Residual **semantic** cue, disclosed rather than removed:

| class | held-out with the label phrase in the body | held-out without it |
| --- | --- | --- |
| final rule | 37 | 12 |
| proposed rule | 32 | 16 |
| public notice | 2 | 39 |
| total | 71 | 67 |

"Label phrase" means the literal class name: `final rule`, `proposed rule`, `public notice`.
Notice documents almost never say "public notice"; they say "this notice" or "notice of", so the
cue-present group is dominated by rules. Per-class F1 is not conditional on the cue, but the
scorer reports a cue-free stratum (67 of 138 known held-out documents) so a reader can see the
result without the documents that name their own type.

**Not audited:** near-duplicate detection across splits. Two agencies can publish near-identical
boilerplate; family-stable splitting prevents the obvious case (the same agency in two splits) but
does not run a similarity sweep. This is a known limitation of the corpus, not a claim of
independence.

## 4. Metadata handling

The `removed_metadata_lines` field on every document records the exact lines that were dropped, so
the stripping is inspectable document by document. The stripping rule is part of the manifest's
`selection.text_rule`, and the manifest digest changes if any retained text changes.

The issuing-agency block and the document title are **retained**: `DEPARTMENT OF …`,
`Federal Aviation Administration`, `14 CFR Part 71`, docket/RIN lines and the title line above
`AGENCY:`. They are ordinary document content, and the agency is not the gold label. Only the
lines that state the document *type* are removed.
