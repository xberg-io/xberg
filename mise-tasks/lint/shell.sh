#!/usr/bin/env bash
#MISE description="Format-check shell sources (poly does not format shell; see the comment below)"
set -euo pipefail

# poly runs shellcheck on shell but does NOT run shfmt: a tracked .sh mangled on purpose still
# passes `poly fmt --check` while `shfmt -l` flags it, in this repo and in xberg-io/actions, which
# has had [tools.shfmt] enabled all along. So formatting needs an explicit invocation.
# Indentation comes from .editorconfig, which shfmt reads natively.
#
# Scoped to scripts/tests and mise-tasks for now. The other tracked .sh files are 27 files away
# from clean, and reformatting release-path scripts that still have no coverage is the exact
# ordering this project exists to avoid. Widen the scope as suites land. ~keep
#
# scripts/tests/lib is excluded: it holds the VENDORED helper library, which has to stay
# byte-identical to its source in xberg-io/actions or `mise run test:scripts:lib:check` fails.
# .editorconfig would otherwise reformat it to two-space.
#
# The file list is built with find and then asserted non-empty. `git ls-files` misses files that
# are not staged yet, and shfmt with zero arguments reads stdin and exits 0 -- a lint task that
# silently checks nothing is worse than one that is absent. ~keep
files=()
while IFS= read -r file; do
  files+=("$file")
done < <(find scripts/tests mise-tasks -type f \( -name '*.bats' -o -name '*.sh' \) -not -path 'scripts/tests/lib/*')
if [ "${#files[@]}" -eq 0 ]; then
  echo "lint:shell matched no files under scripts/tests or mise-tasks; the scope is stale" >&2
  exit 1
fi
shfmt -d "${files[@]}"
