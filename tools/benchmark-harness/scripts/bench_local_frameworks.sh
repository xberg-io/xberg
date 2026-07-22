#!/usr/bin/env bash

is_native_batch_framework() {
  case "$1" in
  xberg-markdown-baseline | xberg-markdown-baseline-batch | xberg-markdown-layout | xberg-markdown-layout-batch | xberg-markdown-paddle-ocr | xberg-markdown-paddle-ocr-batch | docling | liteparse) return 0 ;;
  *) return 1 ;;
  esac
}

append_unique_framework() {
  local candidate="$1"
  local selected="$2"
  case ",$selected," in
  *",$candidate,"*) printf '%s' "$selected" ;;
  ,,) printf '%s' "$candidate" ;;
  *) printf '%s,%s' "$selected" "$candidate" ;;
  esac
}

native_batch_frameworks() {
  local remaining="$1"
  local candidate
  local selected=""
  while [ -n "$remaining" ]; do
    case "$remaining" in
    *,*)
      candidate="${remaining%%,*}"
      remaining="${remaining#*,}"
      ;;
    *)
      candidate="$remaining"
      remaining=""
      ;;
    esac
    if is_native_batch_framework "$candidate"; then
      selected="$(append_unique_framework "$candidate" "$selected")"
    fi
  done
  printf '%s' "$selected"
}

validate_native_batch_frameworks() {
  local remaining="$1"
  local candidate
  local selected=""
  if [ -z "$remaining" ]; then
    return 0
  fi
  case "$remaining" in
  ,* | *, | *,,*)
    echo "malformed native-batch framework list: $remaining" >&2
    return 1
    ;;
  esac
  while [ -n "$remaining" ]; do
    case "$remaining" in
    *,*)
      candidate="${remaining%%,*}"
      remaining="${remaining#*,}"
      ;;
    *)
      candidate="$remaining"
      remaining=""
      ;;
    esac
    if ! is_native_batch_framework "$candidate"; then
      echo "unsupported native-batch framework: $candidate" >&2
      return 1
    fi
    selected="$(append_unique_framework "$candidate" "$selected")"
  done
  printf '%s' "$selected"
}

framework_list_contains() {
  case ",$1," in
  *",$2,"*) return 0 ;;
  *) return 1 ;;
  esac
}

docling_is_explicitly_requested() {
  framework_list_contains "$1" docling || framework_list_contains "$2" docling
}
