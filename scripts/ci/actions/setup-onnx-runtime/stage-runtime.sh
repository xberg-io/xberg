#!/usr/bin/env bash
# Stages Microsoft's ONNX Runtime in the release archive layout (lib/, plus include/ from NuGet)
# from the Microsoft.ML.OnnxRuntime NuGet package, else from the onnxruntime PyPI wheel. Both
# artifacts must be pinned before anything is downloaded, and only verified bytes are unpacked.
# github.com/microsoft/onnxruntime has returned 404 since 2026-11-18, so its archives are no source. ~keep
set -euo pipefail

ort_version="${1:?ort-version required}"
rid="${2:?NuGet runtime identifier required}"
ort_root="${3:?destination root required}"
checksums="${4:?checksum manifest required}"
script_dir="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

if [[ ! "$ort_version" =~ ^[A-Za-z0-9]([A-Za-z0-9._-]*[A-Za-z0-9])?$ ]]; then
  echo "Invalid ONNX Runtime version: $ort_version" >&2
  exit 1
fi
case "$rid" in
linux-x64 | linux-arm64 | osx-arm64 | osx-x64) ;;
*)
  echo "Unsupported ONNX Runtime runtime identifier: $rid" >&2
  exit 1
  ;;
esac

package="microsoft.ml.onnxruntime.${ort_version}.nupkg"
bash "$script_dir/verify-archive.sh" "$package" "$checksums" --known
# Wheel rows carry the runtime identifier and the PyPI file URL after the digest and name. ~keep
wheels="$(awk -v prefix="onnxruntime-${ort_version}-" -v rid="$rid" '
  { sub(/\r$/, "") }
  $3 == rid && index($2, prefix) == 1 && $2 ~ /\.whl$/ { print $2, $4 }
' "$checksums")"
if [ -z "$wheels" ] || [ "$(printf '%s\n' "$wheels" | wc -l | tr -d '[:space:]')" != 1 ]; then
  printf 'Expected exactly one pinned PyPI wheel for ONNX Runtime %s (%s)\n' "$ort_version" "$rid" >&2
  exit 1
fi
wheel="${wheels%% *}"
wheel_url="${wheels#* }"
case "$wheel_url" in
"https://files.pythonhosted.org/packages/"*"/$wheel") ;;
*)
  printf 'Pinned PyPI wheel %s has an unexpected URL: %s\n' "$wheel" "$wheel_url" >&2
  exit 1
  ;;
esac
bash "$script_dir/verify-archive.sh" "$wheel" "$checksums" --known

work="$(mktemp -d "${RUNNER_TEMP:?RUNNER_TEMP required}/onnxruntime-stage.XXXXXX")"
trap 'rm -rf "$work"' EXIT
# manylinux_2_28 ships curl 7.61, which predates --retry-all-errors. ~keep
curl_options=(--proto '=https' --proto-redir '=https' --retry 5 --retry-delay 5)
if curl --retry-all-errors --version >/dev/null 2>&1; then
  curl_options+=(--retry-all-errors)
fi

# Builder images are not guaranteed to ship unzip; python3 reads the same members. ~keep
extract() {
  local archive="$1" destination="$2"
  shift 2
  if command -v unzip >/dev/null 2>&1; then
    unzip -q "$archive" "${@/%/*}" -d "$destination"
  else
    python3 - "$archive" "$destination" "$@" <<'PYTHON'
import sys
import zipfile

package, destination, *prefixes = sys.argv[1:]
with zipfile.ZipFile(package) as archive:
    for member in archive.namelist():
        if member.startswith(tuple(prefixes)):
            archive.extract(member, destination)
PYTHON
  fi
}

fetch_nuget() {
  if ! curl "${curl_options[@]}" -fsSL -o "$work/$package" \
    "https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/${ort_version}/${package}"; then
    echo "::warning::NuGet download of $package failed; using the PyPI wheel" >&2
    return 1
  fi
  if ! bash "$script_dir/verify-archive.sh" "$work/$package" "$checksums"; then
    echo "::warning::NuGet package $package failed verification; using the PyPI wheel" >&2
    return 1
  fi
}

tree="$work/tree"
mkdir -p "$tree/lib"
if fetch_nuget; then
  origin="NuGet"
  extract "$work/$package" "$work/package" "runtimes/$rid/native/" "build/native/include/"
  native="$work/package/runtimes/$rid/native"
  mkdir -p "$tree/include"
  cp "$work/package/build/native/include/"* "$tree/include/"
  case "$rid" in
  linux-*)
    cp "$native/"*.so "$tree/lib/"
    mv "$tree/lib/libonnxruntime.so" "$tree/lib/libonnxruntime.so.$ort_version"
    ;;
  osx-*) cp "$native/libonnxruntime.dylib" "$tree/lib/libonnxruntime.$ort_version.dylib" ;;
  esac
else
  origin="PyPI"
  curl "${curl_options[@]}" -fsSL -o "$work/$wheel" "$wheel_url"
  bash "$script_dir/verify-archive.sh" "$work/$wheel" "$checksums"
  extract "$work/$wheel" "$work/wheel" "onnxruntime/capi/libonnxruntime"
  capi="$work/wheel/onnxruntime/capi"
  case "$rid" in
  linux-*)
    cp "$capi/libonnxruntime.so.$ort_version" "$tree/lib/"
    for library in "$capi"/libonnxruntime_providers_*.so; do
      if [ -e "$library" ]; then cp "$library" "$tree/lib/"; fi
    done
    ;;
  osx-*) cp "$capi/libonnxruntime.$ort_version.dylib" "$tree/lib/" ;;
  esac
fi

# The release archives name the real library by version and point the short names at it.
# Linking records the Linux SONAME libonnxruntime.so.1 and the macOS install name
# @rpath/libonnxruntime.1.dylib, so both must resolve. ~keep
case "$rid" in
linux-*)
  ln -s "libonnxruntime.so.$ort_version" "$tree/lib/libonnxruntime.so.1"
  ln -s libonnxruntime.so.1 "$tree/lib/libonnxruntime.so"
  ;;
osx-*)
  ln -s "libonnxruntime.$ort_version.dylib" "$tree/lib/libonnxruntime.1.dylib"
  ln -s "libonnxruntime.$ort_version.dylib" "$tree/lib/libonnxruntime.dylib"
  ;;
esac

mkdir -p "$(dirname "$ort_root")"
rm -rf "$ort_root"
mv "$tree" "$ort_root"
echo "Staged ONNX Runtime ${ort_version} (${rid}) from ${origin}"
