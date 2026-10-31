#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"
log_dir="$(mktemp -d)"
[ -n "$log_dir" ] && [ -d "$log_dir" ] || exit 1
trap 'rm -rf "$log_dir"' EXIT
export CARGO_BUILD_JOBS=1
export PKG_CONFIG_PATH="/usr/local/lib/pkgconfig:${PKG_CONFIG_PATH:-}"
expected_version="${EXPECTED_HEIF_VERSION:?selected runtime version is required}"
[ "$(pkg-config --modversion libheif)" = "$expected_version" ] || exit 1
cat >"$log_dir/heif-abi.c" <<'C'
#include <libheif/heif.h>
#include <stdio.h>
#include <string.h>
int main(int argc, char **argv) {
    if (argc != 2 || strcmp(heif_get_version(), argv[1])) return 1;
    struct heif_context *context = heif_context_alloc();
    if (!context) return 2;
    printf("linked libheif %s\n", heif_get_version());
    heif_context_free(context);
    return 0;
}
C
read -r -a heif_flags <<<"$(pkg-config --cflags --libs libheif)"
cc "$log_dir/heif-abi.c" "${heif_flags[@]}" -Wl,-rpath,/usr/local/lib -o "$log_dir/heif-abi"
if [ ! -x "$log_dir/heif-abi" ]; then
  echo 'C runtime probe was not produced as an executable' >&2
  exit 1
fi
ldd "$log_dir/heif-abi" | tee "$log_dir/heif-abi-linkage.txt"
grep -Eq 'libheif.so.1 => /usr/local/lib/libheif.so.1' "$log_dir/heif-abi-linkage.txt"
"$log_dir/heif-abi" "$expected_version"
if "$log_dir/heif-abi" 0.0.0; then
  echo 'wrong-runtime negative control passed'
  exit 1
fi
python3 scripts/ci/verify_heif_runtime.py --library /usr/local/lib/libheif.so.1 --version "$expected_version"
python3 - <<'PY'
import hashlib
import json
from pathlib import Path
root = Path('test_documents')
objects = json.loads((root / 'corpus.lock.json').read_text())['objects']
paths = ['images/test.heic', 'images/test.heif', 'images/test.avif', 'images/alpha.heif']
for name in paths:
    data = (root / name).read_bytes()
    record = objects[name]
    if len(data) != record['size'] or hashlib.sha256(data).hexdigest() != record['sha256']:
        raise RuntimeError(f'fixture identity mismatch: {name}')
if len(paths) != 4:
    raise RuntimeError('expected exactly four fixture inputs')
print('libheif fixture identities expected 4, verified 4')
PY
cargo test --locked -p xberg-libheif --no-default-features --features v1_19 \
  --test decode_smoke --no-run --message-format=json >"$log_dir/build.json"
python3 - "$log_dir/build.json" >"$log_dir/executable" <<'PY'
import json
import sys
rows = [json.loads(line) for line in open(sys.argv[1])]
paths = {row['executable'] for row in rows if row.get('reason') == 'compiler-artifact'
         and row.get('target', {}).get('name') == 'decode_smoke' and row.get('executable')}
if len(paths) != 1:
    raise RuntimeError(f'expected one actual libheif test artifact: {paths}')
print(paths.pop())
PY
artifact="$(cat "$log_dir/executable")"
[ -n "$artifact" ] && [ -x "$artifact" ] || exit 1
ldd "$artifact" | tee "$log_dir/linkage"
grep -Eq 'libheif.so.1 => /usr/local/lib/libheif.so.1' "$log_dir/linkage"
nm -D "$artifact" >"$log_dir/symbols"
grep -q ' U heif_image_get_plane_readonly$' "$log_dir/symbols"
if grep -q ' U heif_image_get_plane_readonly2$' "$log_dir/symbols"; then
  echo 'Rust v1_19 wrapper references a post-floor plane API' >&2
  exit 1
fi
"$artifact" --list | tee "$log_dir/list"
observed="$(awk '/: test$/ { n++ } END { print n + 0 }' "$log_dir/list")"
[ "$observed" -eq 5 ] || {
  echo "expected 5 tests, collected $observed" >&2
  exit 1
}
"$artifact" --nocapture --test-threads=1 2>&1 | tee "$log_dir/run"
if grep -q 'SKIP: fixture' "$log_dir/run"; then
  echo 'A runtime fixture test skipped' >&2
  exit 1
fi
grep -q '^test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;' "$log_dir/run"
echo "Rust libheif v1_19 runtime $expected_version: expected 5, passed 5; fixtures 4"
