#!/usr/bin/env bash
# Vendor the shared Bats helper library from xberg-io/actions.
#
# The library is a committed file, not a fetch-at-test-time dependency, for two reasons:
# a local `bats --recursive scripts/tests/bats` run has to work with no network, and CI must load the same bytes the
# developer loaded. A helper resolved from one place locally and another in CI reintroduces the
# local-green/CI-red class the library exists to prevent.
#
# This script only refreshes that file (`sync`) or proves it has not drifted (`--check`). It is
# never invoked by the test path. ~keep
set -euo pipefail

readonly SOURCE_REPOSITORY="xberg-io/actions"
readonly SOURCE_DIRECTORY="bats-lib/xberg-bats"
readonly LIBRARY_FILES=(load.bash)

mode="${1:-sync}"
destination="${BATS_LIB_DEST:-scripts/tests/lib/xberg-bats}"

case "$mode" in
sync | --check) ;;
*)
  echo "usage: $0 [sync|--check]" >&2
  exit 2
  ;;
esac

ref="${BATS_LIB_REF:-}"
if [ -z "$ref" ]; then
  echo "BATS_LIB_REF must name an immutable ${SOURCE_REPOSITORY} tag or commit." >&2
  echo "The helper library ships from that repository; pin the revision you vendored." >&2
  exit 1
fi

# An immutable ref only. A branch name would make `--check` compare against whatever that branch
# points at today, which is the opposite of a drift gate. ~keep
if ! printf '%s' "$ref" | grep -qE '^(v[0-9]+(\.[0-9]+){2}|[0-9a-f]{40})$'; then
  echo "BATS_LIB_REF must be a vX.Y.Z tag or a full 40-character commit sha, not '${ref}'." >&2
  exit 1
fi

staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT

for file in "${LIBRARY_FILES[@]}"; do
  curl --proto '=https' --proto-redir '=https' --tlsv1.2 \
    --fail --silent --show-error --location \
    --connect-timeout 10 --max-time 60 --retry 3 --retry-delay 2 \
    --output "${staging}/${file}" \
    "https://raw.githubusercontent.com/${SOURCE_REPOSITORY}/${ref}/${SOURCE_DIRECTORY}/${file}"
done

case "$mode" in
sync)
  mkdir -p "$destination"
  cp "$staging"/* "$destination/"
  echo "Vendored ${SOURCE_DIRECTORY} from ${SOURCE_REPOSITORY}@${ref} into ${destination}"
  ;;
--check)
  if ! diff -ru "$destination" "$staging"; then
    echo "::error::${destination} has drifted from ${SOURCE_REPOSITORY}@${ref}." >&2
    echo "Run 'mise run test:scripts:lib:sync' to refresh it." >&2
    exit 1
  fi
  echo "${destination} matches ${SOURCE_REPOSITORY}@${ref}"
  ;;
esac
