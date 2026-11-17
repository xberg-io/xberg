#!/usr/bin/env bash
set -euo pipefail

workflow="${1:-.github/workflows/benchmarks.yaml}"
cache_action="${2:-.github/actions/cache-benchmark-harness/action.yml}"
workflow_content="$(<"$workflow")"
cache_action_content="$(<"$cache_action")"

extract_job() {
  local job_name="$1"
  awk -v header="  ${job_name}:" '
    $0 == header { in_job = 1; print; next }
    in_job && /^  [[:alnum:]_-]+:/ { exit }
    in_job { print }
  '
}

extract_named_step() {
  local step_name="$1"
  awk -v header="      - name: ${step_name}" '
    $0 == header { in_step = 1; print; next }
    in_step && /^      - / { exit }
    in_step { print }
  '
}

extract_with_inputs() {
  awk '
    $0 == "        with:" { in_with = 1; next }
    in_with && /^        [^ ]/ { exit }
    in_with { print }
  '
}

require_exact_input() {
  local inputs="$1"
  local key="$2"
  local value="$3"
  local description="$4"
  local key_count
  local expected_count

  key_count="$(grep -Ec "^[[:space:]]+${key}:" <<<"$inputs" || true)"
  expected_count="$(grep -Fxc "          ${key}: ${value}" <<<"$inputs" || true)"
  if [[ "$key_count" -ne 1 || "$expected_count" -ne 1 ]]; then
    echo "benchmark workflow validation failed: $description"
    exit 1
  fi
}

require_exact_cli_build() {
  local job="$1"
  local description="$2"
  local expected="cargo build --locked --release -p xberg-cli --features all,xberg/sceptre-ocr-tract"
  local commands
  local command_count

  commands="$(
    grep -E '^[[:space:]]+run:[[:space:]]+cargo build --locked --release -p xberg-cli' <<<"$job" |
      sed -E 's/^[[:space:]]*run:[[:space:]]*//' || true
  )"
  command_count="$(grep -c . <<<"$commands" || true)"

  if [[ "$command_count" -ne 1 || "$commands" != "$expected" ]]; then
    echo "benchmark workflow validation failed: $description"
    exit 1
  fi
}

require_no_cli_build() {
  local job="$1"
  local description="$2"
  local command_count

  command_count="$(
    grep -Ec '^[[:space:]]+run:[[:space:]]+cargo build --locked --release -p xberg-cli' <<<"$job" || true
  )"

  if [[ "$command_count" -ne 0 ]]; then
    echo "benchmark workflow validation failed: $description"
    exit 1
  fi
}

extract_action_runs() {
  awk '
    {
      indent = match($0, /[^ ]/) - 1
      if (in_run && ($0 ~ /^[[:space:]]*$/ || indent > run_indent)) {
        if (folded) printf "%s ", $0; else print
        next
      }
      if (in_run && folded) print ""
      in_run = 0
      if ($0 !~ /^[[:space:]]+run:/) next
      printf "%c", 30
      value = $0
      sub(/^[[:space:]]+run:[[:space:]]*/, "", value)
      if (value ~ /^[|>][0-9+-]*([[:space:]]*#.*)?$/) {
        in_run = 1; run_indent = indent; folded = substr(value, 1, 1) == ">"
        next
      }
      quote = substr(value, 1, 1)
      if (quote == sprintf("%c", 34) || quote == sprintf("%c", 39)) {
        sub(/[[:space:]]+#.*$/, "", value)
        if (substr(value, length(value), 1) != quote) exit 1
        value = substr(value, 2, length(value) - 2)
      }
      print value
    }
  '
}

require_exact_harness_build() {
  local scripts
  scripts="$(extract_action_runs <<<"$1")"
  python3 - "$scripts" <<'PYTHON'
import re
import shlex
import sys


def normalize(words):
    prefixes = {"if", "!", "then", "do", "else", "elif", "env", "exec", "command", "-i", "--ignore-environment", "--"}
    environment = False
    while words:
        if words[0] == "env":
            environment = True
            words = words[1:]
        elif environment and words[0].startswith("--unset="):
            words = words[1:]
        elif words[0] in {"-u", "--unset"}:
            words = words[2:]
        elif words[0] in prefixes or re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", words[0]):
            words = words[1:]
        elif environment and words[0].startswith("-"):
            raise ValueError("unsupported env option in benchmark build command")
        else:
            break
    return words


def cargo_builds(scripts):
    builds = []
    for script in scripts.split(chr(30)):
        lexer = shlex.shlex(script.replace("\\\n", ""), posix=True, punctuation_chars=";&|()\n")
        lexer.whitespace = " \t\r"
        lexer.whitespace_split = True
        words = []
        for token in [*lexer, "\n"]:
            if token.strip(";&|()\n"):
                words.append(token)
                continue
            words = normalize(words)
            if words and words[0] == "cargo" and "build" in words[1:]:
                builds.append(words)
            words = []
    return builds


expected = ["cargo", "build", "--locked", "--manifest-path", "tools/benchmark-harness/Cargo.toml",
            "--bin", "benchmark-harness", "$BUILD_ARG"]
try:
    valid = cargo_builds(sys.argv[1]) == [expected]
except ValueError:
    valid = False
if not valid:
    sys.exit(
        "benchmark workflow validation failed: harness cache must build only the uploaded benchmark-harness binary"
    )
PYTHON
}

require_step() {
  local job="$1"
  local pattern="$2"
  local description="$3"

  if ! grep -qE -- "$pattern" <<<"$job"; then
    echo "benchmark workflow validation failed: $description"
    exit 1
  fi
}

setup_job="$(extract_job setup <<<"$workflow_content")"
validate_harness_job="$(extract_job validate-harness <<<"$workflow_content")"
aggregate_job="$(extract_job aggregate-and-publish <<<"$workflow_content")"
setup_rust_step="$(extract_named_step "Setup Rust" <<<"$setup_job")"
setup_rust_inputs="$(extract_with_inputs <<<"$setup_rust_step")"
swap_step="$(extract_named_step "Provision swap for all-feature release link" <<<"$setup_job")"
cohort_validation_step="$(extract_named_step "Validate benchmark cohorts" <<<"$validate_harness_job")"
harness_contract_step="$(extract_named_step "Validate benchmark harness contracts" <<<"$validate_harness_job")"

require_exact_input "$setup_rust_inputs" use-sccache '"false"' \
  "setup Rust must disable per-object sccache uploads"
require_exact_input "$setup_rust_inputs" disable-cache '"false"' \
  "setup Rust must retain the coarse Cargo target cache"
require_exact_cli_build "$setup_job" \
  "setup must build exactly one all-feature CLI for benchmark size measurement"
require_step "$swap_step" '^[[:space:]]+run: mise run benchmark:setup:swap$' \
  "setup must provision swap through the benchmark task before the all-feature release link"
swap_line="$(grep -nF -- '- name: Provision swap for all-feature release link' <<<"$setup_job" | cut -d: -f1)"
cli_build_line="$(grep -nF -- '- name: Build xberg-cli (release, all features + Sceptre tract diagnostic)' <<<"$setup_job" | cut -d: -f1)"
if [[ -z "$swap_line" || -z "$cli_build_line" || "$swap_line" -ge "$cli_build_line" ]]; then
  echo "benchmark workflow validation failed: swap must be provisioned before the all-feature release link"
  exit 1
fi
if grep -Fq -- '- name: Validate benchmark cohorts' <<<"$setup_job" ||
  grep -Fq -- '- name: Validate benchmark harness contracts' <<<"$setup_job"; then
  echo "benchmark workflow validation failed: deterministic harness tests must not relink in the measured setup job"
  exit 1
fi
require_step "$validate_harness_job" '^  validate-harness:$' \
  "deterministic harness tests must run in a standalone job"
require_step "$validate_harness_job" '^[[:space:]]+runs-on: ubuntu-latest$' \
  "standalone harness validation must use a fresh x86 Linux runner"
require_step "$validate_harness_job" '^[[:space:]]+timeout-minutes: 360$' \
  "standalone harness validation must retain the long build timeout"
require_step "$validate_harness_job" '^[[:space:]]+- uses: actions/checkout@v7$' \
  "standalone harness validation must check out the benchmark source"
require_step "$validate_harness_job" '^[[:space:]]+ref: \$\{\{ github\.sha \}\}$' \
  "standalone harness validation must check out the immutable workflow revision"
require_step "$validate_harness_job" '^[[:space:]]+submodules: recursive$' \
  "standalone harness validation must initialize fixture submodules"
for required_step in \
  'Free disk space' \
  'Install system dependencies' \
  'Setup Rust' \
  'Setup ONNX Runtime' \
  'Fetch test_documents fixtures' \
  'Authenticate to Google Cloud' \
  'Set up Cloud SDK' \
  'Restore reference corpus cache'; do
  require_step "$validate_harness_job" "- name: ${required_step}$" \
    "standalone harness validation is missing required step: ${required_step}"
done
require_step "$validate_harness_job" '^[[:space:]]+- name: Install zstd \(corpus cache decompression\)$' \
  "standalone harness validation is missing the zstd installation step"
require_step "$validate_harness_job" '^[[:space:]]+id-token: write$' \
  "standalone harness validation must authenticate for the reference corpus cache"
require_step "$validate_harness_job" '^[[:space:]]+run: scripts/benchmarks/restore-corpus-cache\.sh$' \
  "standalone harness validation must restore ground truth before loading benchmark cohorts"
if grep -qE '^[[:space:]]+(needs:|continue-on-error:)' <<<"$validate_harness_job"; then
  echo "benchmark workflow validation failed: standalone harness validation must run in parallel and remain required"
  exit 1
fi
if grep -Fq -- 'uses: ./.github/actions/setup-layout-models' <<<"$validate_harness_job"; then
  echo "benchmark workflow validation failed: deterministic harness tests must not download runtime layout models"
  exit 1
fi
require_step "$cohort_validation_step" \
  "cargo test --locked -p benchmark-harness --lib 'cohort::tests::' 2>&1 \\| tee /tmp/cohort-tests\\.log" \
  "cohort validation must use the non-release library test target"
require_step "$harness_contract_step" \
  'cargo test --locked -p benchmark-harness --no-fail-fast$' \
  "harness contract validation must use the non-release test profile"
for contract_target in validate_artifacts lossless_aggregation fixture_validation aggregate_schema; do
  require_step "$harness_contract_step" "--test ${contract_target}" \
    "harness contract validation is missing target: ${contract_target}"
done
if grep -q -- '--release' <<<"${cohort_validation_step}${harness_contract_step}"; then
  echo "benchmark workflow validation failed: deterministic harness tests must not use the release profile"
  exit 1
fi
# aggregate reuses the exact xberg-cli binary `setup` built and uploaded
# (benchmarks-target) rather than cold-rebuilding it. This is what guarantees
# installation-size consistency -- the measured binary is byte-identical to the
# benchmarked one -- and avoids a redundant release compile in the release job.
require_no_cli_build "$aggregate_job" \
  "aggregate must reuse the setup-built xberg-cli artifact, not cold-rebuild it"
require_step "$aggregate_job" '^[[:space:]]+- name: Download build artifacts \(harness binary \+ xberg-cli\)' \
  "aggregate must download the benchmarks-target artifact for size measurement"
require_step "$aggregate_job" 'restore-binary-permissions\.sh' \
  "aggregate must restore benchmark binary permissions after downloading the artifact"

app_token_step="$(extract_named_step "Create GitHub App token (if credentials available)" <<<"$aggregate_job")"
if grep -qE '^[[:space:]]+owner:' <<<"$app_token_step"; then
  echo "benchmark workflow validation failed: release token must default to the current repository"
  exit 1
fi

for cache_input in 'BUILD_ENV_HASH=' 'RUSTFLAGS:-' 'CARGO_BUILD_TARGET:-' 'rustc -vV' 'uname -s'; do
  if ! grep -Fq "$cache_input" <<<"$cache_action_content"; then
    echo "benchmark workflow validation failed: harness cache key omits build input $cache_input"
    exit 1
  fi
done

require_exact_harness_build "$cache_action_content"

echo "benchmark workflow build configuration is valid"
