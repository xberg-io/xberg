#!/usr/bin/env bash

_macho_macos_floor() {
  local file="$1" arch="$2" commands
  if ! commands="$(otool -arch "$arch" -l "$file")"; then
    printf 'failed to inspect Mach-O slice %s in %s\n' "$arch" "$file" >&2
    return 1
  fi
  awk '
    $1 == "Load" && $2 == "command" {
      if ($3 !~ /^[0-9]+$/ || NF != 3) invalid = 1
      if (in_command && command_fields != 1) invalid = 1
      in_command = 1
      command = ""
      command_fields = 0
    }
    $1 == "cmd" {
      command_fields++
      if (!in_command || command_fields != 1 || NF != 2 || $2 !~ /^(LC_[A-Z0-9_]+|0x[0-9a-fA-F]+)$/) {
        invalid = 1
        next
      }
      command = $2
      if (command == "LC_BUILD_VERSION" || command ~ /^LC_VERSION_MIN_/) {
        declarations++
        if (command != "LC_BUILD_VERSION" && command != "LC_VERSION_MIN_MACOSX") invalid = 1
      }
    }
    command == "LC_BUILD_VERSION" && $1 == "platform" {
      platforms++
      if (NF != 2 || ($2 != "1" && tolower($2) != "macos")) invalid = 1
    }
    (command == "LC_BUILD_VERSION" && $1 == "minos") ||
      (command == "LC_VERSION_MIN_MACOSX" && $1 == "version") {
      floors++
      if (NF != 2 || $2 !~ /^[1-9][0-9]*\.[0-9]+(\.[0-9]+)?$/) invalid = 1
      floor = $2
      if (command == "LC_BUILD_VERSION") modern = 1
    }
    END {
      if (invalid || command_fields != 1 || declarations != 1 || floors != 1 || platforms != modern + 0) exit 1
      print floor
    }
  ' <<<"$commands" || {
    printf 'missing or invalid macOS deployment floor for slice %s in %s\n' "$arch" "$file" >&2
    return 1
  }
}

# Emit one "<arch> <floor>" row for every validated macOS slice.
# An optional second argument requires an exact slice count.
macho_minos_by_arch() {
  local file="$1" expected_count="${2:-}" declared arch floor seen="" rows=()
  local -a arches=()
  if ! declared="$(lipo -archs "$file")"; then
    printf 'failed to list Mach-O slices in %s\n' "$file" >&2
    return 1
  fi
  if [[ -z "$declared" || "$declared" == *$'\n'* ]]; then
    printf 'missing or invalid Mach-O slice list in %s\n' "$file" >&2
    return 1
  fi
  IFS=$' \t' read -r -a arches <<<"$declared"
  if [[ ${#arches[@]} == 0 || (-n "$expected_count" && "$expected_count" != "${#arches[@]}") ]]; then
    printf 'unexpected Mach-O slice count in %s: %s (expected %s)\n' "$file" "${#arches[@]}" "$expected_count" >&2
    return 1
  fi
  for arch in "${arches[@]}"; do
    if [[ ! "$arch" =~ ^[a-zA-Z0-9_]+$ || "$seen" == *" $arch "* ]]; then
      printf 'invalid or duplicate Mach-O slice %s in %s\n' "$arch" "$file" >&2
      return 1
    fi
    seen+=" $arch "
    floor="$(_macho_macos_floor "$file" "$arch")" || return 1
    rows+=("$arch $floor")
  done
  printf '%s\n' "${rows[@]}"
}

# Return a scalar only when all slices declare the same macOS floor.
minos_of() {
  local rows arch floor common=""
  rows="$(macho_minos_by_arch "$@")" || return 1
  while IFS=" " read -r arch floor; do
    if [[ -n "$common" && "$common" != "$floor" ]]; then
      printf 'different macOS deployment floors in %s (%s: %s, earlier: %s)\n' "$1" "$arch" "$floor" "$common" >&2
      return 1
    fi
    common="$floor"
  done <<<"$rows"
  printf '%s\n' "$common"
}
