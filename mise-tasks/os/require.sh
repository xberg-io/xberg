#!/usr/bin/env bash
#MISE description="Fail unless this machine runs one of the named operating systems"
#MISE hide=true
#USAGE arg "<os>..." help="Accepted systems: linux, darwin or windows"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

# A platform-gated task depends on this, so off-platform it fails loudly instead of reporting a
# pass that ran nothing. ~keep
case "$(uname -s)" in
Linux*) current=linux ;;
Darwin*) current=darwin ;;
MINGW* | MSYS* | CYGWIN*) current=windows ;;
*) current=unknown ;;
esac
for wanted in $usage_os; do
  if [ "$wanted" = "$current" ]; then
    exit 0
  fi
done
echo "this task runs only on: $usage_os (this machine: $current)" >&2
exit 1
