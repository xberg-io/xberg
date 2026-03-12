#!/usr/bin/env bash
set -euo pipefail

mode="${1:-check}"
root="$(git rev-parse --show-toplevel)"

# C# directories with .csproj or .slnx files
# Each entry is "dir:target" where target is the solution/project file
csharp_targets=(
  "packages/csharp:Kreuzberg.slnx"
  "e2e/csharp:Kreuzberg.E2E.csproj"
)

failed=0

for entry in "${csharp_targets[@]}"; do
  dir="${entry%%:*}"
  target="${entry##*:}"
  full="$root/$dir"

  if [ ! -f "$full/$target" ]; then
    continue
  fi

  echo "==> Linting $dir ($target)"
  cd "$full"
  dotnet restore "$target" --verbosity quiet 2>/dev/null || true

  case "$mode" in
  fix)
    dotnet format "$target" || failed=1
    ;;
  check)
    dotnet format "$target" --verify-no-changes || failed=1
    ;;
  esac
done

exit $failed
