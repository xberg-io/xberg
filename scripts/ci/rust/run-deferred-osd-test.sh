#!/usr/bin/env bash
set -euo pipefail

export XBERG_OSD_TEST_MODEL="${XBERG_OSD_TEST_MODEL:?Pinned cached OSD model required}"
export XBERG_OSD_FIXTURE="$PWD/test_documents/images/chi_sim_image.jpeg"
printf '%s  %s\n' \
  9cf5d576fcc47564f11265841e5ca839001e7e6f38ff7f7aacf46d15a96b00ff "$XBERG_OSD_TEST_MODEL" \
  8bd2f7f94402a1429bf04d53de13ed93a2983da2e4183288bd59097cdbd4c0e2 "$XBERG_OSD_FIXTURE" |
  shasum -a 256 -c -

name=ocr::processor::api_pool::retry::tests::warm_retry_loads_deferred_osd_from_its_private_models
log=$(mktemp)
trap 'rm -f "$log"' EXIT
cargo test --locked -p xberg --no-default-features --features pdf,ocr,bundle-tessdata-eng \
  --lib -- --exact "$name" --ignored --color never 2>&1 | tee "$log"
if ! grep -qF "test $name ... ok" "$log" ||
  ! grep -qF 'test result: ok. 1 passed; 0 failed; 0 ignored;' "$log"; then
  echo "Expected the deferred native OSD control to execute and pass exactly once" >&2
  exit 1
fi
