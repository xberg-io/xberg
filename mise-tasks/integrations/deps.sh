#!/usr/bin/env bash
#MISE description="Install, update or upgrade the integration dependencies"
#MISE hide=true
#USAGE arg "<mode>" help="sync installs the locked set; update stays within major versions; upgrade takes the latest"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

# shellcheck source=mise-tasks/integrations/.lib/packages.sh
. "$MISE_CONFIG_ROOT/mise-tasks/integrations/.lib/packages.sh"
case "$usage_mode" in
sync) uv_flags=() maven_goal="" ;;
update) uv_flags=(--upgrade) maven_goal=versions:use-next-versions ;;
upgrade) uv_flags=(--upgrade) maven_goal=versions:use-latest-releases ;;
*)
  echo "mode must be sync, update or upgrade, got: $usage_mode" >&2
  exit 2
  ;;
esac
for dir in "${uv_dirs[@]}"; do
  (cd "$dir" && uv sync --all-extras ${uv_flags[@]+"${uv_flags[@]}"})
done
[ -n "$maven_goal" ] || exit 0
(cd "$spring_ai" && command -v mvn >/dev/null 2>&1 &&
  mvn -q "$maven_goal" -DgenerateBackupPoms=false ||
  echo "Maven not found, skipping spring-ai $usage_mode")
