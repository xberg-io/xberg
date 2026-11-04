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
- `fixtures/` — hand-written e2e fixtures; `e2e/<lang>/` — suites generated from them
- `integrations/` — framework integrations
- `tools/` — benchmark, OCR measurement, perf tooling
- `test_documents/` — corpus submodule (`test-corpus` skill)
- `docs-site/`, `templates/` — docs site, doc templates
- `skills/`, `plugin/` — published skills, agent plugin; generated from `plugin/.ai-rulez/` (`plugin-bundles` rule)
- `charts/`, `docker/`, `scripts/` — deploy, images, CI scripts

Layout and where code belongs: `crate-structure` skill.

CI and governance validation run only on `main`, including manual dispatches. PRs do not run CI. Before merging, run
the drift gates for what you touched (`mise run alef:verify`, `e2e:verify`, `plugin:verify`, `docs:verify:changelog`,
`supported-counts:verify`, `verify:feature-parity`, `verify:docker-crates`) locally. After merging, check main CI or dispatch
`gh workflow run ci-lint.yaml --ref main`.
