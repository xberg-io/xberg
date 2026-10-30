#!/usr/bin/env bash

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

error() {
  echo -e "${RED}Error: $*${NC}" >&2
  exit 1
}

info() {
  echo -e "${GREEN}$*${NC}" >&2
}

warn() {
  echo -e "${YELLOW}$*${NC}" >&2
}

if command -v sha256sum &>/dev/null; then
  HASH_CMD=(sha256sum)
elif command -v shasum &>/dev/null; then
  HASH_CMD=(shasum -a 256)
else
  error "Neither sha256sum nor shasum found in PATH"
fi

MODE="glob"
if [[ "${1:-}" == "--files" ]]; then
  MODE="files"
  shift
elif [[ "${1:-}" == "--dirs" ]]; then
  MODE="dirs"
  shift
fi

if [[ $# -eq 0 ]]; then
  error "No input provided. Usage: $0 <pattern...> or $0 --files <file...> or $0 --dirs <dir...>"
fi

TEMP_HASHES=$(mktemp)
trap 'rm -f "$TEMP_HASHES"' EXIT

case "$MODE" in
files)
  for file in "$@"; do
    if [[ -f "$file" ]]; then
      "${HASH_CMD[@]}" "$file" >>"$TEMP_HASHES" 2>/dev/null || warn "Failed to hash: $file"
    else
      warn "File not found: $file"
    fi
  done
  ;;

dirs)
  for dir in "$@"; do
    if [[ -d "$dir" ]]; then
      find "$dir" -type f \
        ! -path "*/.*" \
        ! -path "*/target/*" \
        ! -path "*/node_modules/*" \
        ! -path "*/.venv/*" \
        ! -path "*/dist/*" \
        ! -path "*/build/*" \
        -exec "${HASH_CMD[@]}" {} \; >>"$TEMP_HASHES" 2>/dev/null ||
        warn "Failed to hash one or more files under: $dir (cache key may be stale)"
    else
      warn "Directory not found: $dir"
    fi
  done
  ;;

glob)
  for pattern in "$@"; do

    if [[ "$pattern" == *"**"* ]]; then
      base_dir=$(echo "$pattern" | cut -d'*' -f1 | sed 's|/$||')

      suffix="${pattern#*\*\*/}"

      if [[ "$suffix" == /* ]]; then
        name_pattern="${suffix#/}"
      else
        name_pattern="$suffix"
      fi

      if [[ -d "$base_dir" ]]; then
        find "$base_dir" -type f \
          ! -path "*/.*" \
          ! -path "*/target/*" \
          ! -path "*/node_modules/*" \
          ! -path "*/.venv/*" \
          -name "$name_pattern" \
          -exec "${HASH_CMD[@]}" {} \; 2>/dev/null >>"$TEMP_HASHES" ||
          warn "Failed to hash one or more files under: $base_dir (cache key may be stale)"
      else
        warn "Directory not found: $base_dir"
      fi
    else
      for file in $pattern; do
        if [[ -f "$file" ]]; then
          "${HASH_CMD[@]}" "$file" >>"$TEMP_HASHES" 2>/dev/null || warn "Failed to hash: $file"
        fi
      done
    fi
  done
  ;;
esac

if [[ ! -s "$TEMP_HASHES" ]]; then
  error "No files found matching the provided patterns"
fi

FINAL_HASH=$(sort "$TEMP_HASHES" | "${HASH_CMD[@]}" | cut -d' ' -f1)

SHORT_HASH="${FINAL_HASH:0:12}"

echo "$SHORT_HASH"

FILE_COUNT=$(wc -l <"$TEMP_HASHES")
info "Hashed $FILE_COUNT files → $SHORT_HASH" >&2
