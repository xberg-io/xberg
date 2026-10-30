#!/usr/bin/env bats

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	cd "$XBERG_WORK" || return
	mkdir -p scripts/benchmarks scripts/lib tools/benchmark-harness/scripts test_documents/ground_truth test_documents/.corpus-cache/pdf test_documents/.corpus-cache/ground_truth/pdf temp
	cp "$REPOSITORY_ROOT/scripts/benchmarks/publish-corpus-cache.sh" scripts/benchmarks/
	cp "$REPOSITORY_ROOT/scripts/lib/common.sh" scripts/lib/
	cp "$REPOSITORY_ROOT/tools/benchmark-harness/scripts/corpus_cache_manifest.py" tools/benchmark-harness/scripts/
	touch Cargo.toml
	export TMPDIR="$XBERG_WORK/temp"
	python3 - <<'PY'
import hashlib,json
from pathlib import Path
root=Path('test_documents/.corpus-cache')
fields={'pdf_sha256':'pdf/synthetic.pdf','out_gt_md_sha256':'ground_truth/pdf/synthetic.md','gt_txt_sha256':'ground_truth/pdf/synthetic.txt'}
doc={'id':'synthetic','redistribute':'reference','gate_verdict':'ACCEPT','size_tier':'smoke'}
for key,name in fields.items():
 data=f'synthetic {key}\n'.encode();(root/name).write_bytes(data);doc[key]=hashlib.sha256(data).hexdigest()
Path('test_documents/ground_truth/corpus_manifest.json').write_text(json.dumps({'documents':[doc]}))
PY
	xberg_stub git 'printf "git %s\n" "$*" >>"$XBERG_TRACE"; printf "0123456789abcdef\n"'
	xberg_stub zstd 'printf "zstd %s\n" "$*" >>"$XBERG_TRACE"' '[ "${ZSTD_STATUS:-0}" = 0 ] || exit "$ZSTD_STATUS"' 'while [ "$1" != -f ]; do shift; done; cp "$2" "$4"'
	xberg_stub gcloud 'printf "gcloud %s\n" "$*" >>"$XBERG_TRACE"' '[ "$5" = --if-generation-match=0 ] || exit 98' '[ "${UPLOAD_STATUS:-0}" = 0 ] || exit "$UPLOAD_STATUS"' 'cp "$3" "$XBERG_WORK/uploaded.tar"'
	unset ZSTD_STATUS UPLOAD_STATUS
}

@test "corpus publisher should verify exact three synthetic archive members and conditional content address" {
	run "$BASH" scripts/benchmarks/publish-corpus-cache.sh
	xberg_assert_status 0
	python3 - <<'PY'
import tarfile
from pathlib import Path
with tarfile.open('uploaded.tar') as t:
 files=[m for m in t if m.isfile()]
 assert len(files)==3
 for m in files: assert t.extractfile(m).read()==(Path('test_documents')/m.name).read_bytes()
PY
	digest="$(python3 tools/benchmark-harness/scripts/corpus_cache_manifest.py digest --manifest test_documents/ground_truth/corpus_manifest.json)"
	grep -F "gs://xberg-benchmark-corpus/corpus-cache/v2/$digest.tar.zst --if-generation-match=0" "$XBERG_TRACE"
	[ "$(grep -c '^gcloud ' "$XBERG_TRACE")" -eq 1 ]
	[ ! -d test_documents/.corpus-cache.lock ]
	[ "$(find temp -mindepth 1 | wc -l)" -eq 0 ]
}

@test "corpus publisher should reject checksum corruption before compression or upload and release lock" {
	printf corrupt >test_documents/.corpus-cache/pdf/synthetic.pdf
	run "$BASH" scripts/benchmarks/publish-corpus-cache.sh
	xberg_assert_status 1
	xberg_assert_output_contains 'digest mismatch'
	[ "$(grep -c '^git ' "$XBERG_TRACE")" -eq 1 ]
	[ "$(wc -l <"$XBERG_TRACE")" -eq 1 ]
	[ ! -d test_documents/.corpus-cache.lock ]
	[ "$(find temp -mindepth 1 | wc -l)" -eq 0 ]
}

@test "corpus publisher should preserve another operation lock and avoid all tools" {
	mkdir test_documents/.corpus-cache.lock
	run "$BASH" scripts/benchmarks/publish-corpus-cache.sh
	xberg_assert_status 1
	xberg_assert_output_contains 'locked by another'
	[ -d test_documents/.corpus-cache.lock ]
	xberg_assert_trace_empty
	[ "$(find temp -mindepth 1 | wc -l)" -eq 0 ]
}

@test "corpus publisher should propagate upload failure and clean owned temporary data" {
	export UPLOAD_STATUS=9
	run "$BASH" scripts/benchmarks/publish-corpus-cache.sh
	xberg_assert_status 9
	[ "$(grep -c '^gcloud ' "$XBERG_TRACE")" -eq 1 ]
	[ ! -f uploaded.tar ]
	[ ! -d test_documents/.corpus-cache.lock ]
	[ "$(find temp -mindepth 1 | wc -l)" -eq 0 ]
}

@test "corpus publisher should stop on compression failure before upload" {
	export ZSTD_STATUS=8
	run "$BASH" scripts/benchmarks/publish-corpus-cache.sh
	xberg_assert_status 8
	[ "$(grep -c '^zstd ' "$XBERG_TRACE")" -eq 1 ]
	[ ! -f uploaded.tar ]
	[ "$(wc -l <"$XBERG_TRACE")" -eq 2 ]
}

@test "model prefetch should substitute version and generate selected-model program before one cargo invocation" {
	xberg_stub cargo 'printf "cargo %s\n" "$*" >>"$XBERG_TRACE"; cp Cargo.toml "$XBERG_WORK/prefetch.toml"; cp src/main.rs "$XBERG_WORK/prefetch.rs"; printf "%s\n" "$MODELS" >"$XBERG_WORK/models"'
	run env FASTEMBED_VERSION=5.8.0 MODELS=default,BGEBaseENV15 "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/cache-hf-fastembed/prefetch.sh"
	xberg_assert_status 0
	xberg_assert_trace 'cargo run --release'
	grep -F 'version = "5.8.0"' prefetch.toml
	grep -F 'for raw in models.split' prefetch.rs
	grep -F '"BGEBaseENV15" => Some(EmbeddingModel::BGEBaseENV15)' prefetch.rs
	xberg_assert_file models default,BGEBaseENV15
}

@test "model prefetch should keep cache warming best effort on cargo failure" {
	xberg_stub cargo 'printf "cargo %s\n" "$*" >>"$XBERG_TRACE"; exit 7'
	run env FASTEMBED_VERSION=5.8.0 MODELS=default "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/cache-hf-fastembed/prefetch.sh"
	xberg_assert_status 0
	xberg_assert_trace 'cargo run --release'
}

@test "model prefetch should fail before cargo when version is missing" {
	xberg_stub_trace cargo
	run env -u FASTEMBED_VERSION "$BASH" "$REPOSITORY_ROOT/scripts/ci/actions/cache-hf-fastembed/prefetch.sh"
	[ "$status" -ne 0 ]
	xberg_assert_output_contains FASTEMBED_VERSION
	xberg_assert_trace_empty
}

@test "line filter AB should reject unsupported comparisons without executing any extraction" {
	xberg_stub_trace cargo
	xberg_stub_trace xberg
	run "$BASH" "$REPOSITORY_ROOT/tools/ocr-measure/ab_line_filter.sh" synthetic.pdf gt.txt output native false
	xberg_assert_status 2
	xberg_assert_output_contains 'no runtime configuration toggle'
	xberg_assert_output_contains 'distinct recorded source revision'
	xberg_assert_output_contains 'clear both the OCR cache and extraction cache'
	[ ! -d output ]
	xberg_assert_trace_empty
}

@test "line filter AB should return usage for missing arguments without any extraction" {
	run "$BASH" "$REPOSITORY_ROOT/tools/ocr-measure/ab_line_filter.sh" synthetic.pdf
	xberg_assert_status 2
	xberg_assert_output_contains 'usage: ab_line_filter.sh'
	xberg_assert_trace_empty
}
