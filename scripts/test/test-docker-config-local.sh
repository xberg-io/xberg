#!/bin/bash

################################################################################
################################################################################

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$(cd "$SCRIPT_DIR/../../docker" && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

TEST_VARIANT="${TEST_VARIANT:-all}"
IMAGE_NAME="${IMAGE_NAME:-}"
VERBOSE="${VERBOSE:-false}"
KEEP_CONTAINERS="${KEEP_CONTAINERS:-false}"
TIMEOUT_SECONDS=30
PORT_BASE=18100
TEST_TEMP_DIR="/tmp/xberg-config-test-$$"

TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
declare -a FAILED_TEST_NAMES=()
declare -a TESTED_VARIANTS=()

################################################################################
################################################################################

log_header() {
  echo -e "\n${CYAN}╔════════════════════════════════════════════════════════╗${NC}"
  echo -e "${CYAN}║ $1${NC}"
  echo -e "${CYAN}╚════════════════════════════════════════════════════════╝${NC}\n"
}

log_info() {
  echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
  echo -e "${GREEN}[PASS]${NC} $*"
}

log_warning() {
  echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
  echo -e "${RED}[FAIL]${NC} $*"
}

log_debug() {
  if [ "$VERBOSE" = "true" ]; then
    echo -e "${YELLOW}[DEBUG]${NC} $*"
  fi
}

start_test() {
  TOTAL_TESTS=$((TOTAL_TESTS + 1))
  local test_num
  test_num=$(printf "%02d" $TOTAL_TESTS)
  echo ""
  echo -e "${CYAN}Test $test_num:${NC} $*"
}

pass_test() {
  PASSED_TESTS=$((PASSED_TESTS + 1))
  log_success "Test passed"
}

fail_test() {
  FAILED_TESTS=$((FAILED_TESTS + 1))
  FAILED_TEST_NAMES+=("$1")
  log_error "Test failed: $1"
  if [ -n "${2:-}" ]; then
    log_error "  Details: $2"
  fi
}

# shellcheck disable=SC2317,SC2329  # Function is invoked via trap EXIT
cleanup() {
  log_info "Cleaning up test environment..."

  if [ "$KEEP_CONTAINERS" != "true" ]; then
    docker ps -a --filter "name=xberg-config-test-" --format "{{.Names}}" | while read -r container; do
      log_debug "Stopping container: $container"
      docker stop "$container" 2>/dev/null || true
      docker rm "$container" 2>/dev/null || true
    done
  else
    log_warning "Keeping containers for inspection (use 'docker ps -a' to view)"
  fi

  if [ -d "$TEST_TEMP_DIR" ]; then
    log_debug "Removing temporary directory: $TEST_TEMP_DIR"
    rm -rf "$TEST_TEMP_DIR"
  fi
}

trap cleanup EXIT

################################################################################
################################################################################

setup_test_environment() {
  log_info "Setting up test environment..."

  if ! mkdir -p "$TEST_TEMP_DIR"; then
    log_error "Failed to create temporary directory"
    exit 1
  fi

  log_debug "Test temp directory: $TEST_TEMP_DIR"
}

verify_docker_available() {
  if ! command -v docker &>/dev/null; then
    log_error "Docker is not installed or not in PATH"
    exit 1
  fi

  if ! docker ps &>/dev/null; then
    log_error "Docker daemon is not running or you don't have permissions"
    exit 1
  fi

  log_info "Docker is available"
}

check_image_exists() {
  local image="$1"

  if ! docker image inspect "$image" &>/dev/null; then
    log_error "Docker image does not exist: $image"
    log_error "Please build the image first with: docker build -f $DOCKER_DIR/Dockerfile.${image##*:} -t $image ."
    return 1
  fi

  return 0
}

get_image_name() {
  local variant="$1"

  if [ -n "$IMAGE_NAME" ]; then
    echo "$IMAGE_NAME"
  else
    echo "xberg:$variant"
  fi
}

################################################################################
################################################################################

create_toml_config() {
  local file_path="$1"
  local port="${2:-8000}"

  cat >"$file_path" <<EOF
use_cache = true
enable_quality_processing = true

[ocr]
backend = "tesseract"
language = "eng"
EOF

  log_debug "Created TOML config: $file_path"
}

create_yaml_config() {
  local file_path="$1"
  local port="${2:-8000}"

  cat >"$file_path" <<EOF
use_cache: true
enable_quality_processing: true

ocr:
  backend: "tesseract"
  language: "eng"
EOF

  log_debug "Created YAML config: $file_path"
}

create_json_config() {
  local file_path="$1"
  local port="${2:-8000}"

  cat >"$file_path" <<EOF
{
  "use_cache": true,
  "enable_quality_processing": true,
  "ocr": {
    "backend": "tesseract",
    "language": "eng"
  }
}
EOF

  log_debug "Created JSON config: $file_path"
}

################################################################################
################################################################################

run_container() {
  local container_name="$1"
  local image="$2"
  local port="$3"
  shift 3

  local docker_opts=()
  local cmd_args=()
  local after_separator=false

  while [ $# -gt 0 ]; do
    if [ "$1" = "--" ]; then
      after_separator=true
      shift
      continue
    fi

    if [ "$after_separator" = true ]; then
      cmd_args+=("$1")
    else
      docker_opts+=("$1")
    fi
    shift
  done

  log_debug "Running container: $container_name"
  log_debug "Docker opts: ${docker_opts[*]}"
  log_debug "Command args: ${cmd_args[*]}"

  if ! docker run -d \
    --name "$container_name" \
    -p "$port:8000" \
    "${docker_opts[@]}" \
    "$image" \
    "${cmd_args[@]}" >/dev/null 2>&1; then
    return 1
  fi

  return 0
}

wait_for_health() {
  local port="$1"
  local max_wait="${2:-$TIMEOUT_SECONDS}"
  local elapsed=0
  local interval=1

  log_debug "Waiting for service on port $port (timeout: ${max_wait}s)"

  while [ "$elapsed" -lt "$max_wait" ]; do
    if curl -sf "http://localhost:$port/health" &>/dev/null; then
      log_debug "Service became healthy after ${elapsed}s"
      return 0
    fi

    sleep $interval
    elapsed=$((elapsed + interval))
  done

  log_debug "Service did not become healthy within ${max_wait}s"
  return 1
}

check_container_running() {
  local container_name="$1"

  if docker inspect "$container_name" --format='{{.State.Running}}' 2>/dev/null | grep -q "true"; then
    return 0
  fi

  return 1
}

get_container_logs() {
  local container_name="$1"
  docker logs "$container_name" 2>&1 | tail -20
}

################################################################################
################################################################################

test_etc_xberg_mount() {
  local variant="$1"
  start_test "Volume mount to /etc/xberg/xberg.toml (variant: $variant)"

  local image
  image="$(get_image_name "$variant")"
  local port=$((PORT_BASE + TOTAL_TESTS))
  local container_name="xberg-config-test-etc-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/xberg.toml"

  create_toml_config "$config_file" "$port"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/xberg/xberg.toml:ro"; then
    fail_test "Failed to start container with /etc/xberg mount"
    log_error "  Container logs:\n$(get_container_logs "$container_name" 2>/dev/null || echo 'N/A')"
    return 1
  fi

  sleep 2

  if ! check_container_running "$container_name"; then
    fail_test "Container exited unexpectedly"
    log_error "  Container logs:\n$(get_container_logs "$container_name")"
    return 1
  fi

  if ! wait_for_health "$port"; then
    fail_test "Service failed to start (health check timeout)"
    log_error "  Container logs:\n$(get_container_logs "$container_name")"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  if ! curl -sf "http://localhost:$port/health" >/dev/null; then
    fail_test "Health endpoint returned non-success status"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  log_success "Service is running and healthy"
  docker stop "$container_name" 2>/dev/null || true
  pass_test
}

test_app_config_mount() {
  local variant="$1"
  start_test "Volume mount to /app/.config/xberg/config.toml (variant: $variant)"

  local image
  image="$(get_image_name "$variant")"
  local port=$((PORT_BASE + TOTAL_TESTS))
  local container_name="xberg-config-test-app-config-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/config.toml"

  create_toml_config "$config_file" "$port"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/app/.config/xberg/config.toml:ro"; then
    fail_test "Failed to start container with /app/.config mount"
    log_error "  Container logs:\n$(get_container_logs "$container_name" 2>/dev/null || echo 'N/A')"
    return 1
  fi

  sleep 2

  if ! check_container_running "$container_name"; then
    fail_test "Container exited unexpectedly"
    log_error "  Container logs:\n$(get_container_logs "$container_name")"
    return 1
  fi

  if ! wait_for_health "$port"; then
    fail_test "Service failed to start (health check timeout)"
    log_error "  Container logs:\n$(get_container_logs "$container_name")"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  if ! curl -sf "http://localhost:$port/health" >/dev/null; then
    fail_test "Health endpoint returned non-success status"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  log_success "Service is running and healthy"
  docker stop "$container_name" 2>/dev/null || true
  pass_test
}

test_custom_path_with_flag() {
  local variant="$1"
  start_test "Custom path with --config flag (variant: $variant)"

  local image
  image="$(get_image_name "$variant")"
  local port=$((PORT_BASE + TOTAL_TESTS))
  local container_name="xberg-config-test-custom-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/custom-config.toml"
  local container_path="/app/custom-config.toml"

  create_toml_config "$config_file" "$port"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:$container_path:ro" \
    --entrypoint "/usr/local/bin/xberg" \
    -- "serve" "--config" "$container_path" "--host" "0.0.0.0"; then
    fail_test "Failed to start container with custom --config flag"
    log_error "  Container logs:\n$(get_container_logs "$container_name" 2>/dev/null || echo 'N/A')"
    return 1
  fi

  sleep 2

  if ! check_container_running "$container_name"; then
    fail_test "Container exited unexpectedly"
    log_error "  Container logs:\n$(get_container_logs "$container_name")"
    return 1
  fi

  if ! wait_for_health "$port"; then
    fail_test "Service failed to start (health check timeout)"
    log_error "  Container logs:\n$(get_container_logs "$container_name")"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  if ! curl -sf "http://localhost:$port/health" >/dev/null; then
    fail_test "Health endpoint returned non-success status"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  log_success "Service is running and healthy with custom config path"
  docker stop "$container_name" 2>/dev/null || true
  pass_test
}

test_env_var_overrides() {
  local variant="$1"
  start_test "Environment variable overrides with config file (variant: $variant)"

  local image
  image="$(get_image_name "$variant")"
  local port=$((PORT_BASE + TOTAL_TESTS))
  local container_name="xberg-config-test-env-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/env-config.toml"

  create_toml_config "$config_file" "8000"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/xberg/xberg.toml:ro" \
    --env "XBERG_SERVER_PORT=$port"; then
    fail_test "Failed to start container with env var override"
    log_error "  Container logs:\n$(get_container_logs "$container_name" 2>/dev/null || echo 'N/A')"
    return 1
  fi

  sleep 2

  if ! check_container_running "$container_name"; then
    fail_test "Container exited unexpectedly"
    log_error "  Container logs:\n$(get_container_logs "$container_name")"
    return 1
  fi

  if ! wait_for_health "$port"; then
    fail_test "Service failed to start (health check timeout)"
    log_error "  Container logs:\n$(get_container_logs "$container_name")"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  if ! curl -sf "http://localhost:$port/health" >/dev/null; then
    fail_test "Health endpoint returned non-success status"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  log_success "Service is running with environment variable overrides"
  docker stop "$container_name" 2>/dev/null || true
  pass_test
}

test_toml_format() {
  local variant="$1"
  start_test "TOML config format (variant: $variant)"

  local image
  image="$(get_image_name "$variant")"
  local port=$((PORT_BASE + TOTAL_TESTS))
  local container_name="xberg-config-test-toml-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/config.toml"

  create_toml_config "$config_file" "$port"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/xberg/xberg.toml:ro"; then
    fail_test "Failed to start container with TOML config"
    return 1
  fi

  sleep 2

  if ! wait_for_health "$port"; then
    fail_test "Service failed to start with TOML config"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  log_success "TOML config format works correctly"
  docker stop "$container_name" 2>/dev/null || true
  pass_test
}

test_yaml_format() {
  local variant="$1"
  start_test "YAML config format (variant: $variant)"

  local image
  image="$(get_image_name "$variant")"
  local port=$((PORT_BASE + TOTAL_TESTS))
  local container_name="xberg-config-test-yaml-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/config.yaml"

  create_yaml_config "$config_file" "$port"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/xberg/xberg.yaml:ro"; then
    fail_test "Failed to start container with YAML config"
    return 1
  fi

  sleep 2

  if ! wait_for_health "$port"; then
    fail_test "Service failed to start with YAML config"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  log_success "YAML config format works correctly"
  docker stop "$container_name" 2>/dev/null || true
  pass_test
}

test_json_format() {
  local variant="$1"
  start_test "JSON config format (variant: $variant)"

  local image
  image="$(get_image_name "$variant")"
  local port=$((PORT_BASE + TOTAL_TESTS))
  local container_name="xberg-config-test-json-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/config.json"

  create_json_config "$config_file" "$port"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/xberg/xberg.json:ro"; then
    fail_test "Failed to start container with JSON config"
    return 1
  fi

  sleep 2

  if ! wait_for_health "$port"; then
    fail_test "Service failed to start with JSON config"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  log_success "JSON config format works correctly"
  docker stop "$container_name" 2>/dev/null || true
  pass_test
}

test_readonly_mount() {
  local variant="$1"
  start_test "Read-only mount (variant: $variant)"

  local image
  image="$(get_image_name "$variant")"
  local port=$((PORT_BASE + TOTAL_TESTS))
  local container_name="xberg-config-test-readonly-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/readonly-config.toml"

  create_toml_config "$config_file" "$port"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/xberg/xberg.toml:ro"; then
    fail_test "Failed to start container with read-only mount"
    return 1
  fi

  sleep 2

  if ! check_container_running "$container_name"; then
    fail_test "Container exited unexpectedly with read-only mount"
    return 1
  fi

  if ! wait_for_health "$port"; then
    fail_test "Service failed to start with read-only mount"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  log_success "Read-only mount works correctly"
  docker stop "$container_name" 2>/dev/null || true
  pass_test
}

################################################################################
################################################################################

test_runtime_notices() {
  local variant="$1"
  start_test "Canonical license and third-party notices (variant: $variant)"

  local image notice packaged
  image="$(get_image_name "$variant")"
  for notice in LICENSE THIRD_PARTY_LICENSES.md; do
    packaged="$TEST_TEMP_DIR/${variant}-${notice}"
    if ! docker run --rm --entrypoint cat "$image" \
      "/usr/local/share/licenses/xberg/$notice" >"$packaged"; then
      fail_test "Could not read runtime notice: $notice"
      return 1
    fi
    if ! cmp -s "$DOCKER_DIR/../$notice" "$packaged"; then
      fail_test "Runtime notice differs from canonical source: $notice"
      return 1
    fi
  done
  pass_test
}

run_test_suite() {
  local variant="$1"

  log_header "Testing variant: $(get_image_name "$variant")"

  if ! check_image_exists "$(get_image_name "$variant")"; then
    log_warning "Skipping tests for variant: $variant (image not found)"
    return
  fi

  TESTED_VARIANTS+=("$variant")

  test_runtime_notices "$variant"
  test_etc_xberg_mount "$variant"
  test_app_config_mount "$variant"
  test_custom_path_with_flag "$variant"
  test_env_var_overrides "$variant"
  test_toml_format "$variant"
  test_yaml_format "$variant"
  test_json_format "$variant"
  test_readonly_mount "$variant"
}

print_summary() {
  log_header "Test Summary"

  local pass_rate=0
  if [ $TOTAL_TESTS -gt 0 ]; then
    pass_rate=$((PASSED_TESTS * 100 / TOTAL_TESTS))
  fi

  echo -e "Total Tests:   ${CYAN}$TOTAL_TESTS${NC}"
  echo -e "Passed Tests:  ${GREEN}$PASSED_TESTS${NC}"
  echo -e "Failed Tests:  ${RED}$FAILED_TESTS${NC}"
  echo -e "Pass Rate:     ${BLUE}${pass_rate}%${NC}"
  echo ""

  if [ $FAILED_TESTS -gt 0 ]; then
    echo -e "${RED}Failed Tests:${NC}"
    for test_name in "${FAILED_TEST_NAMES[@]}"; do
      echo "  - $test_name"
    done
    echo ""
  fi

  if [ ${#TESTED_VARIANTS[@]} -gt 0 ]; then
    echo -e "${CYAN}Tested Variants:${NC}"
    for variant in "${TESTED_VARIANTS[@]}"; do
      echo "  - $(get_image_name "$variant")"
    done
    echo ""
  fi
}

################################################################################
################################################################################

main() {
  while [[ $# -gt 0 ]]; do
    case $1 in
    --variant)
      TEST_VARIANT="$2"
      shift 2
      ;;
    --image)
      IMAGE_NAME="$2"
      shift 2
      ;;
    --verbose)
      VERBOSE=true
      shift
      ;;
    --keep-containers)
      KEEP_CONTAINERS=true
      shift
      ;;
    --help)
      echo "Usage: $0 [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --variant VARIANT       Test specific variant (core, full, or all) [default: all]"
      echo "  --image IMAGE          Use pre-built image instead of building [default: build from Dockerfile]"
      echo "  --verbose              Enable verbose output"
      echo "  --keep-containers      Don't cleanup containers after tests"
      echo "  --help                 Show this help message"
      exit 0
      ;;
    *)
      log_error "Unknown option: $1"
      exit 1
      ;;
    esac
  done

  log_header "Docker Configuration Volume Mount Test Suite"

  log_info "Configuration:"
  log_info "  Variant:         $TEST_VARIANT"
  log_info "  Verbose:         $VERBOSE"
  log_info "  Keep Containers: $KEEP_CONTAINERS"
  log_info "  Port Range:      $PORT_BASE-$((PORT_BASE + 99))"
  log_info ""

  verify_docker_available

  setup_test_environment

  case "$TEST_VARIANT" in
  core)
    run_test_suite "core"
    ;;
  full)
    run_test_suite "full"
    ;;
  all)
    run_test_suite "core"
    run_test_suite "full"
    ;;
  *)
    log_error "Invalid variant: $TEST_VARIANT (must be 'core', 'full', or 'all')"
    exit 1
    ;;
  esac

  print_summary

  if [ $FAILED_TESTS -eq 0 ]; then
    log_success "All tests passed!"
    exit 0
  else
    log_error "Some tests failed"
    exit 1
  fi
}

main "$@"
