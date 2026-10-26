# xberg integrations

First-party integrations that connect xberg document extraction to popular AI and
data frameworks. Each is published to its language's registry and versioned in
lockstep with the core `xberg` release (including `-rc.N` pre-releases).

| Integration | Path | Package | Registry |
|-------------|------|---------|----------|
| LangChain | `python/langchain` | `langchain-xberg` | PyPI |
| LlamaIndex (readers) | `python/llama-index/readers/llama-index-readers-xberg` | `llama-index-readers-xberg` | PyPI |
| LlamaIndex (node parser) | `python/llama-index/node_parsers/llama-index-node-parser-xberg` | `llama-index-node-parser-xberg` | PyPI |
| CrewAI | `python/crewai` | `crewai-xberg` | PyPI |
| txtai | `python/txtai` | `txtai-xberg` | PyPI |
| SurrealDB | `python/surrealdb` | `surrealdb-xberg` | PyPI |
| Spring AI | `java/spring-ai` | `io.xberg:spring-ai-xberg` | Maven Central |
| n8n | `node/n8n-nodes-xberg` | `@xberg-io/n8n-nodes-xberg` | npm |

## Layout

- **Python** packages are standalone [uv](https://docs.astral.sh/uv/) projects — each
  owns its own `uv.lock` and is built/tested independently (they are intentionally
  *not* uv-workspace members, to avoid cross-framework resolver conflicts). Work on
  one with `cd integrations/python/<name> && uv sync --all-extras --prerelease=allow`.
- **Java** (`java/spring-ai`) is a Maven project: `mvn -f integrations/java/spring-ai/pom.xml test`.
- **Node** (`node/n8n-nodes-xberg`) is a TypeScript package: `npm ci && npm run build`.

## Versioning

Never hand-edit an integration's `version` or its `xberg` dependency pin. The root
`Cargo.toml` version is the single source of truth; `task version:sync` propagates it
to every integration via `scripts/sync_integration_versions.py`. See the
`xberg-release` skill under `.ai-rulez/skills/`.

## Dependencies

`task integrations:sync` installs all Python integration dependencies;
`task integrations:update` / `task integrations:upgrade` refresh their lock files.
These are wired into the repo-level `task setup` / `update` / `upgrade`.

## CI & publishing

- `.github/workflows/ci-integrations.yaml` — path-filtered tests (Python matrix, Maven, Node).
- Publishing rides the core `.github/workflows/publish.yaml` as a lockstep stage, so
  each integration ships with the core version it targets.

Docs live in the docs-site under `docs-site/src/content/docs/integrations/`.
