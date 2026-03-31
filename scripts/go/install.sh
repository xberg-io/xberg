#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-${GOLANGCI_LINT_VERSION:-latest}}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "Installing golangci-lint ${VERSION}..."
go install "github.com/golangci/golangci-lint/v2/cmd/golangci-lint@${VERSION}"

cd "${REPO_ROOT}/packages/go/v4"
go mod download
go mod tidy
