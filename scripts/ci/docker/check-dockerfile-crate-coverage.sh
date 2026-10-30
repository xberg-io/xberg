#!/usr/bin/env bash
# Fails fast (no docker build required) when a Cargo workspace member under
# crates/ is neither COPYd into a docker/Dockerfile.* build stage nor
# explicitly stripped from that Dockerfile's copy of Cargo.toml via the
# `sed -i '/<crate>/d; ...' Cargo.toml` exclusion line.
#
# Regression guard for #325: `crates/ttf-parser-compat` was added as a
# workspace member but never added to any Dockerfile's COPY list, so cargo
# failed to load its manifest inside the Docker build context (which only
# contains the crates each Dockerfile explicitly COPYs) while every
# non-Docker CI leg — which checks out the full repo — stayed green.
#
# It also checks a THIRD dimension, whose absence is why this script reported
# "full coverage" on 2026-08-21 while every Docker publish job was failing:
# a COPY can name a real workspace member and still fail, because
# `.dockerignore` starts with `*` and re-includes a hand-written allowlist. A
# path missing from that allowlist is simply not in the build context, and
# buildx dies with `"/crates/<name>": not found`. Being COPYd and being
# reachable are different properties; this script used to check only the first.
#
# It also checks the REVERSE direction, which the member-driven pass above is
# structurally incapable of seeing: a COPY line naming a crate that no longer
# exists. That happened to this very crate — `crates/ttf-parser-compat` was
# deleted when the PDF engine became a path dependency and all ten COPY lines were
# left behind, so every Docker publish job died on
#   "/crates/ttf-parser-compat": not found
# before compiling anything, while this script reported full coverage. Iterating
# over members alone can never catch a dangling COPY; you have to iterate over
# the COPY lines too.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

members=()
while IFS= read -r member; do
  members+=("$member")
done < <(grep -oE '"crates/[a-zA-Z0-9_-]+"' Cargo.toml | tr -d '"' | sed 's#^crates/##' | sort -u)
if [ "${#members[@]}" -eq 0 ]; then
  echo "::error::no workspace crates found; refusing an empty coverage check" >&2
  exit 1
fi

status=0
dockerfile_count=0

for dockerfile in docker/Dockerfile.*; do
  [ -f "$dockerfile" ] || continue
  dockerfile_count=$((dockerfile_count + 1))

  for crate in "${members[@]}"; do
    if grep -q "COPY crates/${crate}/ " "$dockerfile"; then
      continue
    fi
    # Match a real sed delete ADDRESS for this exact crate, not a bare
    # substring. `grep -q -- "$crate"` made the core member `xberg` a
    # substring of every `/xberg-py/d`, `/xberg-node/d`, ... directive, so
    # `xberg` counted as sed-excluded in all ten Dockerfiles and a missing
    # `COPY crates/xberg/` could never be reported -- the #325 outage this
    # script exists to prevent. The two address shapes in use are
    # `/<crate>/d`, `/"crates\/<crate>"/d`, and -- in Dockerfile.cli, whose
    # sed runs inside a double-quoted shell string -- the backslash-escaped
    # `/\"crates\/<crate>\"/d`. The optional `\\?` covers that dialect; without
    # it xberg-wasm reads as un-excluded and the check false-positives. ~keep
    if grep "sed -i" "$dockerfile" | grep -qE "[/\"]${crate}\\\\?\"?/d"; then
      continue
    fi
    echo "::error file=${dockerfile}::workspace member 'crates/${crate}' is neither COPYd nor sed-excluded"
    status=1
  done
done

if [ "$dockerfile_count" -eq 0 ]; then
  echo "::error::no Dockerfiles found; refusing an empty coverage check" >&2
  exit 1
fi

# Reverse direction: every `COPY crates/<x>/` must name a real workspace member.
# Guards against a deleted crate leaving a dangling COPY behind.
for dockerfile in docker/Dockerfile.*; do
  [ -f "$dockerfile" ] || continue

  while read -r copied; do
    [ -n "$copied" ] || continue
    for member in "${members[@]}"; do
      if [ "$member" = "$copied" ]; then
        copied=""
        break
      fi
    done
    [ -n "$copied" ] || continue
    echo "::error file=${dockerfile}::COPY names 'crates/${copied}', which is not a Cargo workspace member (deleted crate?)"
    status=1
  done < <(grep -oE '^COPY crates/[a-zA-Z0-9_-]+/' "$dockerfile" |
    sed 's#^COPY crates/##; s#/$##' | sort -u)
done

# Third direction: every COPYd crates/ path must survive .dockerignore, and every
# crates/ entry in the allowlist must name a directory that still exists.
allowed=()
while IFS= read -r entry; do
  allowed+=("$entry")
done < <(grep -oE '^!crates/[a-zA-Z0-9_-]+' .dockerignore | sed 's#^!crates/##')

for dockerfile in docker/Dockerfile.*; do
  [ -f "$dockerfile" ] || continue
  while read -r copied; do
    [ -n "$copied" ] || continue
    for entry in ${allowed[@]+"${allowed[@]}"}; do
      if [ "$entry" = "$copied" ]; then
        copied=""
        break
      fi
    done
    [ -n "$copied" ] || continue
    echo "::error file=${dockerfile}::COPY names 'crates/${copied}', which .dockerignore excludes from the build context; buildx will fail with '\"/crates/${copied}\": not found'. Add '!crates/${copied}/' to .dockerignore"
    status=1
  done < <(grep -oE '^COPY crates/[a-zA-Z0-9_-]+/' "$dockerfile" |
    sed 's#^COPY crates/##; s#/$##' | sort -u)
done

for entry in ${allowed[@]+"${allowed[@]}"}; do
  if [ ! -d "crates/${entry}" ]; then
    echo "::error file=.dockerignore::allowlists 'crates/${entry}', which no longer exists. Stale entries hide real omissions"
    status=1
  fi
done

if [ "$status" -eq 0 ]; then
  echo "Checked ${#members[@]} workspace crates across $dockerfile_count Dockerfiles."
  echo "All Cargo workspace members are covered by every docker/Dockerfile.*,"
  echo "every docker COPY of a crates/ path names a real workspace member,"
  echo "and every COPYd path survives .dockerignore."
fi

exit "$status"
