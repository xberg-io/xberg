---
priority: high
---

# Package Structure

This repo contains **two independently-published PyPI packages**, not one.

## Packages

- `readers/llama-index-readers-xberg/` — PyPI: `llama-index-readers-xberg`
  - `XbergReader` at `llama_index.readers.xberg`
  - Depends on `xberg` and `llama-index-core>=0.13,<0.15`
- `node_parsers/llama-index-node-parser-xberg/` — PyPI: `llama-index-node-parser-xberg`
  - `XbergNodeParser` at `llama_index.node_parser.xberg`
  - Depends only on `llama-index-core>=0.13,<0.15`

## Root `pyproject.toml`

- Version `0.0.0` — workspace coordinator only, **never published**
- Defines shared tooling config: ruff, mypy, codespell
- mypy spans both sub-packages for cross-package type checking

## Running Tests

- Run from sub-package directory: `cd readers/llama-index-readers-xberg && pytest`
- Or: `cd node_parsers/llama-index-node-parser-xberg && pytest`
- Do NOT run `pytest` from repo root

## Publishing

- Each package versioned and published independently
- Build backend: hatchling
