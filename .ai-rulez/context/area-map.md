---
type: Concept
title: Area Map
x-ai-rulez:
  kind: context
  id: area-map
  metadata:
    priority: high
---

# Area Map

Xberg is a Rust document intelligence library with polyglot bindings.

- `crates/xberg/` — core library; scoped skills load per `src/` area
- `crates/xberg-cli/` — command-line binary
- `crates/xberg-{py,node,php,wasm,ffi,jni}/` — binding crates (`alef-generated-bindings` skill)
- `crates/xberg-*/` — other crates: OCR, PDF, NER, format backends
- `packages/<lang>/` — generated language packages
- `e2e/<lang>/`, `fixtures/` — generated e2e suites and fixtures
- `integrations/` — framework integrations
- `tools/` — benchmark, OCR measurement, perf tooling
- `test_documents/` — corpus submodule (`test-corpus` skill)
- `docs-site/`, `templates/` — docs site, doc templates
- `skills/`, `plugin/` — published skills, agent plugin
- `charts/`, `docker/`, `scripts/` — deploy, images, CI scripts

Layout and where code belongs: `crate-structure` skill.
