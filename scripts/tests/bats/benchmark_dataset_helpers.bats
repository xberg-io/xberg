#!/usr/bin/env bats

setup_file() {
	bats_load_library xberg-bats
	xberg_shadow_system_path_without huggingface-cli git-lfs
}

setup() {
	bats_load_library xberg-bats
	xberg_setup
	REPOSITORY_ROOT="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd -P)"
	export PATH="$XBERG_STUB_BIN:$XBERG_SYS_BIN"
	cd "$XBERG_WORK" || return
	mkdir -p tools/benchmark-harness/scripts test_documents/pdf
	cp "$REPOSITORY_ROOT/tools/benchmark-harness/scripts/gen_split_fixtures.sh" tools/benchmark-harness/scripts/
	for name in fake_memo flattened_form interactive_form google_doc_document nougat_002 multipage_marketing code_and_formula; do printf '%s\n' "$name" >"test_documents/pdf/$name.pdf"; done
	xberg_stub pdfinfo 'printf "pdfinfo %s\n" "$*" >>"$XBERG_TRACE"' 'printf "Pages: %s\n" "${PAGE_COUNT:-2}"'
	xberg_stub pdfunite 'printf "pdfunite %s\n" "$*" >>"$XBERG_TRACE"' '[ "${UNITE_STATUS:-0}" = 0 ] || exit "$UNITE_STATUS"' 'for arg; do target="$arg"; done; printf "synthetic merged bytes\n" >"$target"'
	xberg_stub git 'exit 1'
	xberg_stub curl 'printf "curl %s\n" "$*" >>"$XBERG_TRACE"' 'if [ "${HTTP_ERROR:-false}" = true ]; then case " $* " in *" -f "*) exit 22;; esac; fi' 'while [ "$1" != -o ]; do shift; done; printf "annotation bytes\n" >"$2"'
	unset PAGE_COUNT UNITE_STATUS HTTP_ERROR
}

@test "split generator should write eight merged artifacts and exact cumulative page boundaries" {
	run "$BASH" tools/benchmark-harness/scripts/gen_split_fixtures.sh
	xberg_assert_status 0
	[ "$(grep -c '^pdfunite ' "$XBERG_TRACE")" -eq 8 ]
	[ "$(grep -c '^pdfinfo ' "$XBERG_TRACE")" -eq 19 ]
	[ "$(find tools/benchmark-harness/fixtures/split -type f | wc -l)" -eq 16 ]
	xberg_assert_file tools/benchmark-harness/fixtures/split/memo_form_memo.pdf 'synthetic merged bytes'
	python3 - <<'PY'
import json
from pathlib import Path
root=Path('tools/benchmark-harness/fixtures/split')
counts={'memo_form_memo':3,'forms_batch':3,'memo_paper':2,'marketing_memo':2,'memo_marketing_form':3,'paper_memo_form':3,'two_papers':2,'single_paper':1}
for name,count in counts.items():
 assert json.loads((root/f'{name}.split.json').read_text())=={'document':f'{name}.pdf','boundaries':[{'start_page':i*2+1,'end_page':i*2+2} for i in range(count)]}
PY
}

@test "split generator should reject zero page metadata before merging" {
	export PAGE_COUNT=0
	run "$BASH" tools/benchmark-harness/scripts/gen_split_fixtures.sh
	xberg_assert_status 1
	xberg_assert_output_contains 'invalid page count'
	[ "$(grep -c '^pdfinfo ' "$XBERG_TRACE")" -eq 1 ]
	run grep -q '^pdfunite ' "$XBERG_TRACE"
	xberg_assert_status 1
}

@test "split generator should propagate merge failures without a success sidecar" {
	export UNITE_STATUS=8
	run "$BASH" tools/benchmark-harness/scripts/gen_split_fixtures.sh
	xberg_assert_status 8
	[ "$(grep -c '^pdfunite ' "$XBERG_TRACE")" -eq 1 ]
	[ ! -f tools/benchmark-harness/fixtures/split/memo_form_memo.split.json ]
}

@test "split generator should reject a missing first source before inspecting or merging" {
	rm test_documents/pdf/fake_memo.pdf
	run "$BASH" tools/benchmark-harness/scripts/gen_split_fixtures.sh
	xberg_assert_status 1
	xberg_assert_output_contains 'missing source:'
	xberg_assert_trace_empty
}

@test "dataset download should reuse populated images and annotations without external calls" {
	mkdir -p dataset/images dataset/ori_pdfs
	printf 'cached annotations\n' >dataset/OmniDocBench.json
	for ((i = 0; i < 101; i++)); do printf image >"dataset/images/$i"; done
	run "$BASH" "$REPOSITORY_ROOT/tools/benchmark-harness/scripts/download_omnidocbench.sh" "$XBERG_WORK/dataset"
	xberg_assert_status 0
	xberg_assert_trace_empty
	xberg_assert_file dataset/OmniDocBench.json 'cached annotations'
}

@test "dataset download should request the exact dataset subsets and preserve downloaded bytes" {
	xberg_stub huggingface-cli 'printf "hf %s\n" "$*" >>"$XBERG_TRACE"' 'mkdir -p "$XBERG_WORK/dataset/images" "$XBERG_WORK/dataset/ori_pdfs"; printf "image bytes\n" >"$XBERG_WORK/dataset/images/one"; printf "pdf bytes\n" >"$XBERG_WORK/dataset/ori_pdfs/one"'
	run "$BASH" "$REPOSITORY_ROOT/tools/benchmark-harness/scripts/download_omnidocbench.sh" "$XBERG_WORK/dataset"
	xberg_assert_status 0
	xberg_assert_file dataset/OmniDocBench.json 'annotation bytes'
	xberg_assert_file dataset/images/one 'image bytes'
	xberg_assert_file dataset/ori_pdfs/one 'pdf bytes'
	[ "$(grep -c '^curl ' "$XBERG_TRACE")" -eq 1 ]
	[ "$(grep -c '^hf ' "$XBERG_TRACE")" -eq 1 ]
	grep -F -- "--include images/* ori_pdfs/* OmniDocBench.json" "$XBERG_TRACE"
}

@test "dataset download should reject HTTP errors before downloading dataset subsets" {
	export HTTP_ERROR=true
	xberg_stub huggingface-cli 'printf "hf %s\n" "$*" >>"$XBERG_TRACE"' 'mkdir -p "$XBERG_WORK/dataset/images" "$XBERG_WORK/dataset/ori_pdfs"'
	run "$BASH" "$REPOSITORY_ROOT/tools/benchmark-harness/scripts/download_omnidocbench.sh" "$XBERG_WORK/dataset"
	xberg_assert_status 22
	run grep -q '^hf ' "$XBERG_TRACE"
	xberg_assert_status 1
}

@test "dataset download should propagate client failure" {
	xberg_stub_exit huggingface-cli 7
	run "$BASH" "$REPOSITORY_ROOT/tools/benchmark-harness/scripts/download_omnidocbench.sh" "$XBERG_WORK/dataset"
	xberg_assert_status 7
	[ "$(grep -c '^curl ' "$XBERG_TRACE")" -eq 1 ]
}

@test "dataset download should reject missing download clients" {
	run "$BASH" "$REPOSITORY_ROOT/tools/benchmark-harness/scripts/download_omnidocbench.sh" "$XBERG_WORK/dataset"
	xberg_assert_status 1
	xberg_assert_output_contains 'Need either huggingface-cli or git-lfs'
}

@test "dataset git fallback should reject absent required image payload" {
	xberg_stub_exit git-lfs 0
	xberg_stub git 'printf "git %s\n" "$*" >>"$XBERG_TRACE"' 'if [ "$1" = clone ]; then mkdir -p "$5/ori_pdfs"; fi'
	run "$BASH" "$REPOSITORY_ROOT/tools/benchmark-harness/scripts/download_omnidocbench.sh" "$XBERG_WORK/dataset"
	xberg_assert_status 1
	[ "$(grep -c '^git ' "$XBERG_TRACE")" -eq 2 ]
}

@test "dataset git fallback should copy both required directories with exact bytes" {
	xberg_stub_exit git-lfs 0
	xberg_stub git 'printf "git %s\n" "$*" >>"$XBERG_TRACE"' 'if [ "$1" = clone ]; then mkdir -p "$5/images" "$5/ori_pdfs"; printf "lfs image\n" >"$5/images/one"; printf "lfs pdf\n" >"$5/ori_pdfs/one"; fi'
	run "$BASH" "$REPOSITORY_ROOT/tools/benchmark-harness/scripts/download_omnidocbench.sh" "$XBERG_WORK/dataset"
	xberg_assert_status 0
	xberg_assert_file dataset/images/one 'lfs image'
	xberg_assert_file dataset/ori_pdfs/one 'lfs pdf'
	[ "$(grep -c '^git ' "$XBERG_TRACE")" -eq 2 ]
}
