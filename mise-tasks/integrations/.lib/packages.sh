# shellcheck shell=bash
# shellcheck disable=SC2034 # each array is read by the task that sources this file
# The first-party integrations under integrations/. Python integrations are standalone uv projects
# (each owns its uv.lock); the spring-ai integration is a Maven project. ~keep

# Standalone uv projects. The two llama-index sub-packages are developed and locked through the
# llama-index dev aggregator. ~keep
uv_dirs=(
  integrations/python/crewai
  integrations/python/txtai
  integrations/python/surrealdb
  integrations/python/langchain
  integrations/python/llama-index
)
spring_ai=integrations/java/spring-ai
# Publishable Python distributions, mirroring the publish-integrations-pypi job. The llama-index
# dev aggregator (integrations/python/llama-index) is version 0.0.0 and is NOT published -- only
# its two sub-packages are. `uv build` needs no network (builds sdist/wheel from source), so this
# is safe to run before the core is published. ~keep
py_build_dirs=(
  integrations/python/crewai
  integrations/python/txtai
  integrations/python/surrealdb
  integrations/python/langchain
  integrations/python/llama-index/readers/llama-index-readers-xberg
  integrations/python/llama-index/node_parsers/llama-index-node-parser-xberg
)
node_dirs=(
  integrations/node/n8n-nodes-xberg
  integrations/node/langchain-xberg
  integrations/node/llamaindex-xberg
)
py_test_packages=(
  python/crewai
  python/txtai
  python/surrealdb
  python/langchain
  python/llama-index/readers/llama-index-readers-xberg
  python/llama-index/node_parsers/llama-index-node-parser-xberg
)
node_test_packages=(
  n8n-nodes-xberg
  langchain-xberg
  llamaindex-xberg
)

# Test one Python integration against the LOCALLY-BUILT xberg wheel, never the released artifact.
# The integration pins xberg>=<this rc>, which is not on PyPI until the same release publishes
# (bootstrap), so it must not resolve from an index. UV_FIND_LINKS makes the local wheel the source
# that satisfies the pin; --no-install-package + the explicit wheel install then pin the exact
# bytes just built. No --locked: the pin was just version-bumped, so the committed lock
# intentionally lags the unpublished rc. ~keep
test_python_integration() {
  local package="$1" wheel_dir="$2"
  (
    cd "integrations/$package" || exit
    export UV_FIND_LINKS="$wheel_dir"
    uv sync --all-extras --prerelease=allow --no-install-package xberg
    uv pip install --python .venv/bin/python --reinstall "$wheel_dir"/*.whl
    uv run --no-sync pytest -q
  )
}
