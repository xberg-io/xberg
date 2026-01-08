#!/bin/bash

################################################################################
# Docker Configuration Volume Mount Testing Script
#
# This script validates all Docker configuration scenarios locally:
# - Volume mounts to /etc/kreuzberg/kreuzberg.toml (recommended)
# - Volume mounts to /app/.config/kreuzberg/config.toml (user path)
# - Custom paths with --config flag
# - Environment variable overrides with config files
# - All config formats (TOML, YAML, JSON)
# - Read-only mounts
#
# Usage: ./test-docker-config-local.sh [OPTIONS]
# Options:
#   --variant core|full|all   Test specific variant (default: all)
#   --verbose                 Enable verbose output
#   --keep-containers         Don't cleanup containers after tests
################################################################################

set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$(cd "$SCRIPT_DIR/../../docker" && pwd)"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Test configuration
TEST_VARIANT="${TEST_VARIANT:-all}"
IMAGE_NAME="${IMAGE_NAME:-}" # Empty means build from Dockerfile
VERBOSE="${VERBOSE:-false}"
KEEP_CONTAINERS="${KEEP_CONTAINERS:-false}"
TIMEOUT_SECONDS=30
PORT_BASE=18100
TEST_TEMP_DIR="/tmp/kreuzberg-config-test-$$"

# Test tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
declare -a FAILED_TEST_NAMES=()
declare -a TESTED_VARIANTS=()

################################################################################
# Helper Functions
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

# shellcheck disable=SC2329  # Function is invoked via trap EXIT
cleanup() {
  log_info "Cleaning up test environment..."

  if [ "$KEEP_CONTAINERS" != "true" ]; then
    # Stop and remove test containers
    docker ps -a --filter "name=kreuzberg-config-test-" --format "{{.Names}}" | while read -r container; do
      log_debug "Stopping container: $container"
      docker stop "$container" 2>/dev/null || true
      docker rm "$container" 2>/dev/null || true
    done
  else
    log_warning "Keeping containers for inspection (use 'docker ps -a' to view)"
  fi

  # Remove temporary test files
  if [ -d "$TEST_TEMP_DIR" ]; then
    log_debug "Removing temporary directory: $TEST_TEMP_DIR"
    rm -rf "$TEST_TEMP_DIR"
  fi
}

trap cleanup EXIT

################################################################################
# Setup Functions
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
    # Use provided image name (CI mode)
    echo "$IMAGE_NAME"
  else
    # Use default naming convention (local mode)
    echo "kreuzberg:$variant"
  fi
}

################################################################################
# Config File Creation Functions
################################################################################

create_toml_config() {
  local file_path="$1"
  local port="${2:-8000}"

  # Always use port 8000 inside container (mapped from host port via -p flag)
  cat >"$file_path" <<EOF
[server]
host = "0.0.0.0"
port = 8000
max_upload_mb = 100
request_timeout_secs = 30

[ocr]
backend = "tesseract"
language = "eng"
enabled = true

[extraction]
enabled = true

[logging]
level = "info"
EOF

  log_debug "Created TOML config: $file_path"
}

create_yaml_config() {
  local file_path="$1"
  local port="${2:-8000}"

  # Always use port 8000 inside container (mapped from host port via -p flag)
  cat >"$file_path" <<EOF
server:
  host: "0.0.0.0"
  port: 8000
  max_upload_mb: 100
  request_timeout_secs: 30

ocr:
  backend: "tesseract"
  language: "eng"
  enabled: true

extraction:
  enabled: true

logging:
  level: "info"
EOF

  log_debug "Created YAML config: $file_path"
}

create_json_config() {
  local file_path="$1"
  local port="${2:-8000}"

  # Always use port 8000 inside container (mapped from host port via -p flag)
  cat >"$file_path" <<EOF
{
  "server": {
    "host": "0.0.0.0",
    "port": 8000,
    "max_upload_mb": 100,
    "request_timeout_secs": 30
  },
  "ocr": {
    "backend": "tesseract",
    "language": "eng",
    "enabled": true
  },
  "extraction": {
    "enabled": true
  },
  "logging": {
    "level": "info"
  }
}
EOF

  log_debug "Created JSON config: $file_path"
}

################################################################################
# Container Testing Functions
################################################################################

run_container() {
  local container_name="$1"
  local image="$2"
  local port="$3"
  shift 3

  # Separate docker options from command arguments
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
# Test Cases
################################################################################

test_etc_kreuzberg_mount() {
  local variant="$1"
  start_test "Volume mount to /etc/kreuzberg/kreuzberg.toml (variant: $variant)"

  local image
  image="$(get_image_name "$variant")"
  local port=$((PORT_BASE + TOTAL_TESTS))
  local container_name="kreuzberg-config-test-etc-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/kreuzberg.toml"

  # Create config file
  create_toml_config "$config_file" "$port"

  # Run container with mount
  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/kreuzberg/kreuzberg.toml:ro"; then
    fail_test "Failed to start container with /etc/kreuzberg mount"
    log_error "  Container logs:\n$(get_container_logs "$container_name" 2>/dev/null || echo 'N/A')"
    return 1
  fi

  sleep 2

  # Check if container is still running
  if ! check_container_running "$container_name"; then
    fail_test "Container exited unexpectedly"
    log_error "  Container logs:\n$(get_container_logs "$container_name")"
    return 1
  fi

  # Wait for service to be healthy
  if ! wait_for_health "$port"; then
    fail_test "Service failed to start (health check timeout)"
    log_error "  Container logs:\n$(get_container_logs "$container_name")"
    docker stop "$container_name" 2>/dev/null || true
    return 1
  fi

  # Test the health endpoint
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
  start_test "Volume mount to /app/.config/kreuzberg/config.toml (variant: $variant)"

  local image
  image="$(get_image_name "$variant")"
  local port=$((PORT_BASE + TOTAL_TESTS))
  local container_name="kreuzberg-config-test-app-config-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/config.toml"

  # Create config file
  create_toml_config "$config_file" "$port"

  # Run container with mount
  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/app/.config/kreuzberg/config.toml:ro"; then
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
  local container_name="kreuzberg-config-test-custom-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/custom-config.toml"
  local container_path="/app/custom-config.toml"

  # Create config file
  create_toml_config "$config_file" "$port"

  # Run container with custom config path
  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:$container_path:ro" \
    --entrypoint "/usr/local/bin/kreuzberg" \
    -- "serve" "--config" "$container_path"; then
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
  local container_name="kreuzberg-config-test-env-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/env-config.toml"

  # Create config file with port 8000
  create_toml_config "$config_file" "8000"

  # Run container with config mount and environment variable override
  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/kreuzberg/kreuzberg.toml:ro" \
    --env "KREUZBERG_SERVER_PORT=$port"; then
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
  local container_name="kreuzberg-config-test-toml-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/config.toml"

  create_toml_config "$config_file" "$port"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/kreuzberg/kreuzberg.toml:ro"; then
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
  local container_name="kreuzberg-config-test-yaml-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/config.yaml"

  create_yaml_config "$config_file" "$port"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/kreuzberg/kreuzberg.yaml:ro"; then
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
  local container_name="kreuzberg-config-test-json-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/config.json"

  create_json_config "$config_file" "$port"

  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/kreuzberg/kreuzberg.json:ro"; then
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
  local container_name="kreuzberg-config-test-readonly-${variant}-$$"
  local config_file="$TEST_TEMP_DIR/readonly-config.toml"

  create_toml_config "$config_file" "$port"

  # Run with read-only mount (explicitly :ro)
  if ! run_container "$container_name" "$image" "$port" \
    --volume "$config_file:/etc/kreuzberg/kreuzberg.toml:ro"; then
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
# Test Execution
################################################################################

run_test_suite() {
  local variant="$1"

  log_header "Testing variant: $(get_image_name "$variant")"

  # Check if image exists
  if ! check_image_exists "$(get_image_name "$variant")"; then
    log_warning "Skipping tests for variant: $variant (image not found)"
    return
  fi

  TESTED_VARIANTS+=("$variant")

  # Run all test cases
  test_etc_kreuzberg_mount "$variant"
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
# Main Entry Point
################################################################################

main() {
  # Parse command line arguments
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

  # Verify Docker is available
  verify_docker_available

  # Setup test environment
  setup_test_environment

  # Run tests based on variant selection
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

  # Print summary
  print_summary

  # Exit with appropriate code
  if [ $FAILED_TESTS -eq 0 ]; then
    log_success "All tests passed!"
    exit 0
  else
    log_error "Some tests failed"
    exit 1
  fi
}

main "$@"
