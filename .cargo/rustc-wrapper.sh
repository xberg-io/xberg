#!/usr/bin/env bash
set -euo pipefail

if command -v sccache >/dev/null 2>&1; then
  exec sccache "$@"
fi

exec "$@"
