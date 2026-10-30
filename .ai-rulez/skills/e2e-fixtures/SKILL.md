---
type: Playbook
title: E2E Fixtures
description: "Add or edit e2e fixtures: calls, inputs, mocks, assertions and schema."
paths:
  - "fixtures/**"
  - "e2e/**"
x-ai-rulez:
  kind: skill
  id: e2e-fixtures
  metadata:
    name: e2e-fixtures
---

# xberg E2E Fixtures

One JSON object per file under `fixtures/<dir>/`. The directory is the default category; an explicit `category` field
overrides it and names the generated test file (e.g. `fixtures/plugin_api/list_reranker_backends.json` sets
`reranker_backend_management`, so its Python test lands in `e2e/python/tests/test_reranker_backend_management.py`).

- Fields: `id` (unique snake_case, becomes the test name), `call`, `description`, `docs` (`topic`, `title`,
  `description`, `side_effects`), `assertions`; optional `category`, `tags`, `input`, `config`, `args`, `skip`.
- `call`: `extract` (most fixtures), `extract_batch`, `pdf_page_count`, `extract_with_external_redaction`, and the
  plugin-registry calls (`list_*`, `register_*`, `unregister_*`, `clear_*` for OCR, embedding, tokenizer, reranker
  backends, renderers, validators, post-processors; `ocr_backend_supports_language`).
- `input`: `extract_input` (`kind: "uri"` with `$mock_url/...`, or `bytes` + `filename` + `mime_type`) plus
  `mock_responses` (`path`, `status_code`, `headers`, optional `method`, and a body: `body_file` resolved against
  a `responses` subdirectory of `fixtures/` (none exists yet), then `fixtures/` itself, so `../test_documents/...` is
  the in-repo submodule; or `body_inline`).
  Batch calls take `inputs`. A bytes input can also sit at the top level of `input` (`kind: "bytes"`, `bytes`,
  `filename`, `mime_type`) without `extract_input`. Other calls take call-specific `input` fields
  (`pdf_bytes`/`password`, `findings_json`, registry `name`/`backend`); copy an existing fixture of that call.
- Assertion types:
  `not_error`, `error`, `equals`, `min_length`, `count_min`, `count_equals`, `contains`, `contains_any`,
  `contains_all`, `not_contains`, `not_empty`, `greater_than_or_equal`, `less_than_or_equal`. Example `field`:
  `results[0].mime_type`.
- Assertion shape and generation: the shared `e2e-generation` skill.
- `skip`: `{languages: [...], reason: "..."}`.
- Directories: `smoke`, `extract`, `batch`, `contract`, `error`, `format_specific`, `pdf`, `code`, `url`, `plugin_api`,
  `registry`, `summarization`. Field-only categories (all under `plugin_api/`): `ocr_backend_management`,
  `embedding_backend_management`, `tokenizer_backend_management`, `reranker_backend_management`,
  `renderer_management`, `validator_management`, `post_processor_management`.
- `fixtures/repros/` holds repro assets (PDFs, configs, READMEs) and `fixtures/images/` a test image; neither holds
  e2e fixtures.

Copy the nearest existing fixture of the same `call` rather than writing one from this list.
