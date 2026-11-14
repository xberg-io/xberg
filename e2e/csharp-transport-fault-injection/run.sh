#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
case "$(uname -s)" in
Darwin)
  suffix=dylib
  link=-dynamiclib
  ;;
Linux)
  suffix=so
  link=-shared
  ;;
*)
  echo "This probe supports macOS and Linux" >&2
  exit 1
  ;;
esac
dotnet build TransportFaultInjection.csproj
output=bin/Debug/net10.0
"${CC:-cc}" "$link" -fPIC shim.c -o "$output/libxberg_ffi.$suffix"
dotnet "$output/TransportFaultInjection.dll"
