#!/usr/bin/env bash
set -euo pipefail

uv sync --group doc --no-editable --no-install-workspace --no-install-project
