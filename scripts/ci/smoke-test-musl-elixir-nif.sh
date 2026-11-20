#!/usr/bin/env bash
set -euo pipefail

die() {
  echo "smoke-test-musl-elixir-nif: $*" >&2
  exit 1
}

archive="${1:?usage: $0 <archive.tar.gz> <archive.tar.gz.sha256>}"
checksum="${2:?missing archive checksum}"
[ -f "$archive" ] || die "archive not found: $archive"
[ -f "$checksum" ] || die "checksum not found: $checksum"
archive="$(cd "$(dirname "$archive")" && pwd -P)/$(basename "$archive")"
checksum="$(cd "$(dirname "$checksum")" && pwd -P)/$(basename "$checksum")"

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
fixture="$repo/scripts/ci/fixtures/musl-python-smoke.pdf"
[ -f "$fixture" ] || die "smoke-test fixture missing: $fixture"
[ -f "$repo/packages/elixir/lib/xberg/native.ex" ] || die "generated Xberg.Native module missing"
# Use the package lock so the probe runs the consumer's exact loader. ~keep
read -r version digest < <(sed -nE 's/.*"rustler_precompiled": \{:hex, :rustler_precompiled, "([^"]+)".*"([0-9a-f]{64})"\},?$/\1 \2/p' "$repo/packages/elixir/mix.lock")
docker run --rm \
  -v "$archive:/staged/$(basename "$archive"):ro" \
  -v "$checksum:/staged/checksum:ro" \
  -v "$fixture:/fixture.pdf:ro" \
  -v "$repo/scripts/ci/smoke_test_elixir_nif.exs:/smoke/smoke_test_elixir_nif.exs:ro" \
  -v "$repo/packages/elixir:/package:ro" \
  -e "ARCHIVE_NAME=$(basename "$archive")" \
  -e RUSTLER_CACHE=/nif-cache \
  -e "LOADER_VERSION=$version" \
  -e "LOADER_DIGEST=$digest" \
  -e EXPECTED_TEXT="XBERG MUSL SMOKE 490" \
  alpine:3.24.3 sh -euc '
    set -o pipefail
    apk add --no-cache elixir curl >/dev/null
    # The vendored libonnxruntime.so.1 comes from Alpine edge (the builder stage of
    # docker/Dockerfile.musl-rustler upgrades libstdc++ to 15.2.0 before compiling)
    # and relocates against symbols GCC 15 added, so on 3.21 it dies with
    # "Error relocating: _ZNSt8__format25__locale_encoding_to_utf8...: symbol not found".
    # Bundling libstdc++.so.6 in the closure cannot fix this one: apk add elixir pulls
    # Erlang, which links libstdc++ itself, so the BEAM already has 3.21 14.2.0 mapped
    # under that soname before the NIF is dlopened, and the $ORIGIN copy beside the NIF
    # is never consulted. Upgrading here -- after elixir, since apk resolves Erlang
    # so:libstdc++.so.6 against the 3.21 package either way -- puts the runtime on the
    # same libstdc++ the artifact was linked against, which is also why the Java gate
    # installs its JDK from the edge repositories. ~keep
    apk add --no-cache --upgrade libstdc++ \
      --repository=https://dl-cdn.alpinelinux.org/alpine/edge/main >/dev/null
    mkdir -p /work /loader "$RUSTLER_CACHE"
    cd /work
    cp /package/mix.exs .
    curl --fail --silent --show-error --location --retry 3 \
      "https://repo.hex.pm/tarballs/rustler_precompiled-$LOADER_VERSION.tar" -o /loader/hex.tar
    printf "%s  %s\n" "$LOADER_DIGEST" /loader/hex.tar | sha256sum -c -
    tar -xf /loader/hex.tar -O contents.tar.gz | tar -xzf - -C /loader

    read -r hash name </staged/checksum
    test "$name" = "$ARCHIVE_NAME"
    write_checksum() {
      printf "%%{\n  \"%s\" => \"sha256:%s\"\n}\n" "$name" "$1" >checksum-Elixir.Xberg.Native.exs
    }
    run_loader() {
      MIX_ENV=prod RUSTLER_PRECOMPILED_GLOBAL_CACHE_PATH="$RUSTLER_CACHE" \
        elixir -r /loader/lib/rustler_precompiled/config/available_targets.ex \
          -r /loader/lib/rustler_precompiled/config.ex -r /loader/lib/rustler_precompiled.ex \
          /smoke/smoke_test_elixir_nif.exs /fixture.pdf "$EXPECTED_TEXT"
    }
    cp "/staged/$name" "$RUSTLER_CACHE/$name"
    write_checksum "$hash"
    run_loader 2>&1 | tee positive.log
    grep -F "Copying NIF from cache and extracting to " positive.log | grep -F "${name%.tar.gz}"

    # Keep the same cached archive and change only its expected digest. ~keep
    rm -r xberg
    bad_hash=$(printf "%064d" 0)
    test "$hash" != "$bad_hash"
    write_checksum "$bad_hash"
    if run_loader >negative.log 2>&1; then
      echo "RustlerPrecompiled accepted an incorrect checksum" >&2
      exit 1
    fi
    if ! grep -F "the integrity check failed because the checksum of files does not match" negative.log; then
      cat negative.log >&2
      exit 1
    fi
  '
