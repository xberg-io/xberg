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
curl -fsSL --retry 5 --retry-delay 5 --retry-all-errors -o "$work/$package" \
  "https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/${ort_version}/${package}"
bash "$script_dir/verify-archive.sh" "$work/$package" "$checksums"
unzip -q "$work/$package" "runtimes/$rid/native/*" "build/native/include/*" -d "$work/package"

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
