---
type: Decision
title: Partner Corpus Handling
paths:
  - "tools/benchmark-harness/**"
  - "tools/ocr-measure/**"
  - "tools/generate_test_fixtures/**"
  - ".gitmodules"
  - "**/ATTRIBUTIONS.md"
x-ai-rulez:
  kind: rule
  id: partner-corpus-handling
  metadata:
    priority: critical
---

Design-partner document corpora arrive under a commercial agreement, but that says nothing about
the underlying material. **Know which you are holding before deciding anything.**

**Municipal corpora scraped from public authority websites are public records, not confidential
data.** US local-government ordinances, agendas, minutes and resolutions are *edicts of government*
and carry no copyright at all — the corpus attributions file (`ATTRIBUTIONS.md` in the test documents submodule) already vendors City of Sugar Land
records into a **public** repo on exactly that basis. For this class of material:

- Using it for testing, benchmarking and tuning is unproblematic.
- Rendering pages to PNG and reviewing them with vision is unproblematic, and is the **only**
  reliable way to adjudicate OCR output (see `measurement-discipline`).
- Sharing by sha256 between agents is unproblematic.

Do not treat public-record material as radioactive. Over-caution here has a real cost: it blocks
the page-level review that catches content-destroying extraction bugs no text metric detects.

**Two things still need a decision rather than an assumption.**

1. *Redistribution is not the same question as use.* Edicts of government cover the authority's own
   official works. Third-party material merely **filed with** an authority — a contractor's bid, a
   consultant's report, a campaign-finance form completed by an individual — can retain the filer's
   copyright even though the authority publishes it. Bulk republication of personal details also
   remains a live consideration under GDPR/CCPA-style regimes, which public availability weakens but
   does not fully answer. And the partner agreement governs independently of all of it.
2. *Repo visibility is a fact to check, not to infer.* `xberg-io/test_documents` is **PUBLIC**. Run
   `gh repo view <repo> --json visibility` before proposing to commit corpus-derived bytes, and
   state the result when asking a human to approve — an approval given without it is not informed.

**Share by content hash, not by copy.** Corpus objects live content-addressed in the private bucket:

```sh
gcloud storage cp gs://xberg-internal-test-documents/objects/<sha256> ./<local-name>
```

A sha256 identifies the exact file, lets another agent fetch it with credentials it already has,
and publishes nothing new. `scripts/fetch_corpus.py --auth` uses Application Default Credentials for
private buckets; without `--auth` it uses anonymous HTTPS and will not serve them. Keep anything
materialised out of git.

**Identify documents by opaque id, not by filename, in anything you write.** This is naming
hygiene, not a privacy emergency — the material is public record. It earns its place for two
practical reasons: corpus filenames are long, unstable and often embed a person's name, so they make
poor identifiers; and a findings doc that names files cannot be shared outward without a re-read.

The convention is `sha1(relpath)[:10]`, quoted alongside the sha256 when the recipient needs to
fetch the bytes. **`relpath` is relative to the corpus root** — hashing an absolute path yields a
different id that will not match anything already published. Verify a new id against a known pair
before trusting a batch of them.

Applies to findings docs, commit messages, code comments and messages to other agents. When quoting
extracted text as evidence, prefer headings, numbers or boilerplate, and write a private
individual's name as `[NAME]` — an official acting in their official capacity (a mayor signing an
ordinance) is not a private individual and needs no redaction.

Gate the commit on an exact-bytes check over everything a corpus-derived run writes, not a text
grep — a filename can sit inside a generated PDF's metadata where a text search will not find it,
and a check that only *prints* while the commit proceeds anyway has already let two filenames
through here. `[ "$BAD" = "0" ] || exit 1`.
