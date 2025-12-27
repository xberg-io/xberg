#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

source "$REPO_ROOT/scripts/lib/common.sh"
source "$REPO_ROOT/scripts/lib/library-paths.sh"
source "$REPO_ROOT/scripts/lib/tessdata.sh"

validate_repo_root "$REPO_ROOT" || exit 1

setup_rust_ffi_paths "$REPO_ROOT"
setup_tessdata

case "${RUNNER_OS:-$(uname -s)}" in
Linux)
	PATH="/usr/bin:${PATH}"
	;;
macOS)
	PATH="/opt/homebrew/bin:/usr/local/bin:${PATH}"
	;;
Windows*)
	PATH="/c/Program Files/Tesseract-OCR:${PATH}"
	;;
esac

cd "${REPO_ROOT}/e2e/csharp"
results_dir="${REPO_ROOT}/target/test-results/csharp-e2e"
mkdir -p "$results_dir"

dotnet test Kreuzberg.E2E.csproj \
	-c Release \
	--logger "console;verbosity=diagnostic" \
	--logger "trx;LogFileName=csharp-e2e.trx" \
	--results-directory "$results_dir" \
	--diag "$results_dir/dotnet-test-diag.log" \
	--blame \
	--blame-crash \
	--blame-hang \
	--blame-hang-timeout 20m \
	--blame-hang-dump-type mini \
	--blame-crash-dump-type mini
