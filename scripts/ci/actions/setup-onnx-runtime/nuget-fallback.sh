#!/usr/bin/env bash
# Rebuilds the GitHub release archive layout (lib/, include/) from Microsoft's
# Microsoft.ML.OnnxRuntime NuGet package, for when the GitHub release download fails.
set -euo pipefail

ort_version="${1:?ort-version required}"
rid="${2:?NuGet runtime identifier required}"
ort_root="${3:?destination root required}"
checksums="${4:?checksum manifest required}"
script_dir="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

package="microsoft.ml.onnxruntime.${ort_version}.nupkg"
bash "$script_dir/verify-archive.sh" "$package" "$checksums" --known
work="$(mktemp -d "${RUNNER_TEMP:?RUNNER_TEMP required}/onnxruntime-nuget.XXXXXX")"
trap 'rm -rf "$work"' EXIT
# manylinux_2_28 ships curl 7.61, which predates --retry-all-errors. ~keep
curl_options=(--proto '=https' --proto-redir '=https' --retry 5 --retry-delay 5)
if curl --retry-all-errors --version >/dev/null 2>&1; then
  curl_options+=(--retry-all-errors)
fi
curl "${curl_options[@]}" -fsSL -o "$work/$package" \
  "https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/${ort_version}/${package}"
bash "$script_dir/verify-archive.sh" "$work/$package" "$checksums"
# Builder images are not guaranteed to ship unzip; python3 reads the same members. ~keep
if command -v unzip >/dev/null 2>&1; then
  unzip -q "$work/$package" "runtimes/$rid/native/*" "build/native/include/*" -d "$work/package"
else
  python3 - "$work/$package" "$work/package" "runtimes/$rid/native/" "build/native/include/" <<'PYTHON'
import sys
import zipfile

package, destination, *prefixes = sys.argv[1:]
with zipfile.ZipFile(package) as archive:
    for member in archive.namelist():
        if member.startswith(tuple(prefixes)):
            archive.extract(member, destination)
PYTHON
fi

native="$work/package/runtimes/$rid/native"
tree="$work/tree"
mkdir -p "$tree/lib" "$tree/include"
cp "$work/package/build/native/include/"* "$tree/include/"
# The release archives name the real library by version and point the short names at it;
# the binaries record that versioned name (Linux SONAME, macOS install name). ~keep
case "$rid" in
linux-*)
  cp "$native/"*.so "$tree/lib/"
  mv "$tree/lib/libonnxruntime.so" "$tree/lib/libonnxruntime.so.$ort_version"
  ln -s "libonnxruntime.so.$ort_version" "$tree/lib/libonnxruntime.so.1"
  ln -s libonnxruntime.so.1 "$tree/lib/libonnxruntime.so"
  ;;
osx-*)
  cp "$native/libonnxruntime.dylib" "$tree/lib/libonnxruntime.$ort_version.dylib"
  ln -s "libonnxruntime.$ort_version.dylib" "$tree/lib/libonnxruntime.dylib"
  ;;
*)
  echo "Unsupported NuGet runtime identifier: $rid" >&2
  exit 1
  ;;
esac

mkdir -p "$(dirname "$ort_root")"
rm -rf "$ort_root"
mv "$tree" "$ort_root"
echo "Staged ONNX Runtime ${ort_version} (${rid}) from NuGet"
