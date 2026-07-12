#!/usr/bin/env bash
# Mach-O inspection helpers.

# Minimum macOS version a Mach-O binary declares (LC_BUILD_VERSION or the
# older LC_VERSION_MIN_MACOSX).
minos_of() {
  otool -l "$1" | awk '
    /LC_BUILD_VERSION/ {b = 1}
    b && /minos/ {print $2; exit}
    /LC_VERSION_MIN_MACOSX/ {v = 1}
    v && /version / {print $2; exit}'
}
