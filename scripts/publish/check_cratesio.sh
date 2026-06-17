#!/usr/bin/env bash
#   $1: Package version (required)

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 1
fi

version="${1#v}"
export VERSION="$version"

python3 - <<'PY'
import json
import os
import sys
import time
import urllib.request

version = os.environ["VERSION"]

crates = [
    ("kreuzberg-pdfium-render", "pdfium_exists"),
    ("kreuzberg-tesseract", "tesseract_exists"),
    ("kreuzberg-paddle-ocr", "paddle_exists"),
    ("kreuzberg", "kreuzberg_exists"),
    ("kreuzberg-cli", "cli_exists"),
]

max_attempts = 3
sleep_base = 5

results: dict[str, bool] = {}

for crate, output_key in crates:
    exists = False
    for attempt in range(1, max_attempts + 1):
        try:
            url = f"https://crates.io/api/v1/crates/{crate}"
            with urllib.request.urlopen(url, timeout=20) as resp:
                data = json.load(resp)
            versions = [item.get("num") for item in data.get("versions", [])]
            exists = version in versions
            break
        except Exception as exc:
            if attempt >= max_attempts:
                print(
                    f"::warning::crates.io check failed for {crate} {version} ({exc})",
                    file=sys.stderr,
                )
            else:
                sleep_time = attempt * sleep_base
                print(
                    f"::warning::crates.io check failed for {crate} {version} (attempt {attempt}/{max_attempts}), retrying in {sleep_time}s...",
                    file=sys.stderr,
                )
                time.sleep(sleep_time)

    results[output_key] = exists
    status = "already exists" if exists else "not found"
    print(f"::notice::Rust crate {crate} {version} {status} on crates.io", file=sys.stderr)

for key, value in results.items():
    print(f"{key}={'true' if value else 'false'}")

all_exist = all(results.values())
print(f"all_exist={'true' if all_exist else 'false'}")
PY
