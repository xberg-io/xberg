#!/usr/bin/env bash
set -euo pipefail

artifacts_dir="${1:-artifacts/java-natives}"
resources_root="${2:-packages/java/src/main/resources/natives}"

mkdir -p "$resources_root"

find "$artifacts_dir" -maxdepth 2 -type f -print

shopt -s nullglob

for artifact in "$artifacts_dir"/*; do
  if [ ! -d "$artifact" ]; then
    continue
  fi

  direct_files=()
  for f in "$artifact"/*; do
    if [ -f "$f" ]; then
      direct_files+=("$f")
    fi
  done

  if [ "${#direct_files[@]}" -gt 0 ]; then
    artifact_name="$(basename "$artifact")"
    rid="${artifact_name#java-natives-}"
    mkdir -p "$resources_root/$rid"
    for f in "${direct_files[@]}"; do
      case "$(basename "$f")" in
      .gitkeep) continue ;;
      esac
      cp -f "$f" "$resources_root/$rid/"
    done
    continue
  fi

  for rid_dir in "$artifact"/*; do
    if [ ! -d "$rid_dir" ]; then
      continue
    fi
    rid="$(basename "$rid_dir")"
    mkdir -p "$resources_root/$rid"
    for f in "$rid_dir"/*; do
      if [ -f "$f" ]; then
        case "$(basename "$f")" in
        .gitkeep) continue ;;
        esac
        cp -f "$f" "$resources_root/$rid/"
      fi
    done
  done
done

find "$resources_root" -maxdepth 2 -type f -print | sort
