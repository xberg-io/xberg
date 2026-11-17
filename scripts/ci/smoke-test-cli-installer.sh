#!/usr/bin/env bash
# Run inside a clean Linux host with a locally supplied CLI release archive.
set -euo pipefail

archive="$(realpath "${1:?usage: smoke-test-cli-installer.sh ARCHIVE VERSION [musl|gnu|auto] [GNU_ARCHIVE]}")"
version="${2:?expected release version}"
libc="${3:-musl}"
version="${version#v}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

mkdir "$work/bin" "$work/archive"
export SMOKE_GNU_ARCHIVE="${4:-}" SMOKE_ARCHIVE="$archive" XBERG_VERSION="$version" XBERG_INSTALL="$work/installed"
unset XBERG_LINUX_LIBC GITHUB_TOKEN
if [ "$libc" = gnu ]; then
  export XBERG_LINUX_LIBC="$libc"
fi

# Keep target selection observable without requiring network or package installation. ~keep
cat >"$work/bin/curl" <<'SH'
#!/bin/sh
set -eu
[ "$#" -ge 6 ] && [ "$1" = -q ] && [ "$3" = --proto ] && [ "$4" = '=https' ] &&
  [ "$5" = --proto-redir ] && [ "$6" = '=https' ] || exit 91
mode="$2"
shift 6
metadata="https://api.github.com/repos/xberg-io/xberg/releases/tags/v${XBERG_VERSION}"
if [ "$1" = "$metadata" ]; then
  [ "$mode" = -fsS ] && [ "$#" = 3 ] && [ "$2" = -o ] || exit 91
  {
    printf '{"assets":['
    separator=''
    for artifact in "$SMOKE_ARCHIVE" "$SMOKE_GNU_ARCHIVE"; do
      [ -n "$artifact" ] || continue
      name="${artifact##*/}"
      case "$name" in *[!A-Za-z0-9_.-]*) exit 91 ;; esac
      digest="$(sha256sum <"$artifact")"
      printf '%s{"name":"%s","digest":"sha256:%s"}' "$separator" "$name" "${digest%% *}"
      separator=,
    done
    printf ']}\n'
  } >"$3"
  exit 0
fi

[ "$mode" = -fsSL ] && [ "$#" = 5 ] && [ "$1" = -w ] && [ "$2" = '%{http_code}' ] && [ "$4" = -o ] || exit 91
base="https://github.com/xberg-io/xberg/releases/download/v${XBERG_VERSION}"
case "$3" in
  "$base/${SMOKE_ARCHIVE##*/}") cp "$SMOKE_ARCHIVE" "$5" ;;
  "$base/${SMOKE_GNU_ARCHIVE##*/}")
    [ -n "$SMOKE_GNU_ARCHIVE" ]
    cp "$SMOKE_GNU_ARCHIVE" "$5"
    ;;
  *)
    case "${SMOKE_ARCHIVE##*/}:$3" in
      *-linux-musl.tar.gz:"$base/"*-linux-gnu.tar.gz) printf '404'; exit 22 ;;
      *) exit 91 ;;
    esac
    ;;
esac
printf '200'
SH
chmod +x "$work/bin/curl"
PATH="$work/bin:$PATH" bash "$root/scripts/install.sh"

tar -xzf "$archive" -C "$work/archive"
(
  cd "$work/archive/$(basename "$archive" .tar.gz)"
  for file in xberg xberg.bin lib *.so *.so.*; do
    [ -e "$file" ] || continue
    diff -r "$file" "$XBERG_INSTALL/$file"
  done
)
rm -rf "$work/archive"

cd "$work"
cli="$XBERG_INSTALL/xberg"
[ "$("$cli" --version)" = "xberg $version" ]
printf '%s\n' 'Xberg installed layout smoke test' >document.txt
"$cli" extract document.txt | grep -F 'Xberg installed layout smoke test'
"$cli" extract "$root/scripts/ci/fixtures/musl-python-smoke.pdf" | grep -F 'XBERG MUSL SMOKE 490'
"$cli" extract "$root/crates/xberg/tests/fixtures/office/merged_cells.docx" | grep -F 'Blower'
