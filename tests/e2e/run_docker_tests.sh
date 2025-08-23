#!/bin/bash
set -euo pipefail

# Enhanced Kreuzberg Docker E2E Test Runner with security and error handling
# This script builds and tests all Docker images with comprehensive validation

echo "=============================================="
echo "Kreuzberg Docker E2E Test Suite"
echo "=============================================="

# Configuration
DOCKER_DIR=".docker"
DOCKERFILE="${DOCKER_DIR}/Dockerfile"
BUILD_ARGS="${BUILD_ARGS:-}"
SKIP_BUILD="${SKIP_BUILD:-false}"
CLEANUP="${CLEANUP:-false}"
TEST_MODE="${TEST_MODE:-standard}"  # standard or comprehensive
LOG_DIR="${LOG_DIR:-tests/e2e/logs}"
MAX_PARALLEL_BUILDS="${MAX_PARALLEL_BUILDS:-2}"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create log directory
mkdir -p "$LOG_DIR"

# Trap for cleanup on exit
trap cleanup_on_exit EXIT INT TERM

# Cleanup function for exit trap
cleanup_on_exit() {
    local exit_code=$?

    if [ $exit_code -ne 0 ]; then
        echo -e "\n${RED}Script exited with error code: $exit_code${NC}"

        # Save Docker logs on failure
        echo "Saving Docker logs..."
        docker ps -a --filter "name=kreuzberg-test" > "$LOG_DIR/containers.log" 2>&1 || true

        # Clean up test containers
        echo "Cleaning up test containers..."
        docker ps -aq --filter "name=kreuzberg-test" | xargs -r docker rm -f 2>/dev/null || true
    fi

    return $exit_code
}

# Check prerequisites
check_prerequisites() {
    echo "Checking prerequisites..."

    # Check Docker
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}❌ Docker is not installed${NC}"
        exit 1
    fi

    # Check Docker daemon
    if ! docker info &> /dev/null; then
        echo -e "${RED}❌ Docker daemon is not running${NC}"
        exit 1
    fi

    # Check Python 3
    if ! command -v python3 &> /dev/null; then
        echo -e "${RED}❌ Python 3 is not installed${NC}"
        exit 1
    fi

    # Check Python version
    python_version=$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
    if [[ $(echo "$python_version < 3.8" | bc) -eq 1 ]]; then
        echo -e "${YELLOW}⚠️  Python $python_version detected. Python 3.8+ recommended${NC}"
    fi

    # Check disk space (macOS compatible)
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        available_space=$(df -g . | awk 'NR==2 {print $4}')
    else
        # Linux
        available_space=$(df -BG . | awk 'NR==2 {print $4}' | sed 's/G//')
    fi

    if [[ -n "$available_space" ]] && [[ $available_space -lt 10 ]]; then
        echo -e "${YELLOW}⚠️  Low disk space: ${available_space}GB available (10GB recommended)${NC}"
    fi

    echo -e "${GREEN}✓ All prerequisites met${NC}"
}

# Build a single Docker image with logging
build_single_image() {
    local name=$1
    local extras=$2
    local tag=$3
    local log_file="$LOG_DIR/build-${name}.log"

    echo -e "${BLUE}Building ${name} image...${NC}"

    if docker build $BUILD_ARGS -f "$DOCKERFILE" \
        --build-arg EXTRAS="$extras" \
        -t "$tag" \
        --progress=plain \
        . > "$log_file" 2>&1; then

        echo -e "${GREEN}✓ ${name} image built${NC}"
        return 0
    else
        echo -e "${RED}✗ Failed to build ${name} image${NC}"
        echo -e "${YELLOW}  See log: $log_file${NC}"
        return 1
    fi
}

# Build Docker images
build_images() {
    if [ "$SKIP_BUILD" == "true" ]; then
        echo -e "${YELLOW}Skipping image builds (SKIP_BUILD=true)${NC}"

        # Verify images exist
        local missing_images=()
        for image in kreuzberg:core kreuzberg:easyocr kreuzberg:paddle kreuzberg:gmft; do
            if ! docker image inspect "$image" &> /dev/null; then
                missing_images+=("$image")
            fi
        done

        if [ ${#missing_images[@]} -gt 0 ]; then
            echo -e "${RED}Missing images: ${missing_images[*]}${NC}"
            echo -e "${YELLOW}Run without --skip-build to build missing images${NC}"
            exit 1
        fi

        return
    fi

    echo ""
    echo "Building Docker images..."
    echo "----------------------------------------"

    # Enable BuildKit for better performance
    export DOCKER_BUILDKIT=1
    export BUILDKIT_PROGRESS=plain

    # Clean up old images if requested
    if [ "$CLEANUP" == "true" ]; then
        echo "Cleaning up old images..."
        docker rmi kreuzberg:core kreuzberg:easyocr kreuzberg:paddle kreuzberg:gmft 2>/dev/null || true
    fi

    # Build images
    local failed_builds=()

    # Build in parallel with limited concurrency
    (
        build_single_image "core" "" "kreuzberg:core" &
        build_single_image "easyocr" "easyocr" "kreuzberg:easyocr" &
        wait

        build_single_image "paddle" "paddleocr" "kreuzberg:paddle" &
        build_single_image "gmft" "gmft" "kreuzberg:gmft" &
        wait
    )

    # Check build results
    for image in kreuzberg:core kreuzberg:easyocr kreuzberg:paddle kreuzberg:gmft; do
        if ! docker image inspect "$image" &> /dev/null; then
            failed_builds+=("$image")
        fi
    done

    if [ ${#failed_builds[@]} -gt 0 ]; then
        echo -e "${RED}Failed to build images: ${failed_builds[*]}${NC}"
        echo -e "${YELLOW}Check logs in: $LOG_DIR${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ All images built successfully${NC}"

    # Show image sizes
    echo ""
    echo "Image sizes:"
    docker images --format "table {{.Repository}}:{{.Tag}}\t{{.Size}}" | grep kreuzberg || true
}

# Run E2E tests
run_tests() {
    echo ""
    echo "Running E2E tests..."
    echo "----------------------------------------"

    local test_script
    local test_log="$LOG_DIR/test-run.log"

    # Choose test script based on mode
    if [ "$TEST_MODE" == "comprehensive" ]; then
        test_script="tests/e2e/test_docker_comprehensive.py"
        echo -e "${BLUE}Running comprehensive test suite...${NC}"
    else
        test_script="tests/e2e/test_docker_images.py"
        echo -e "${BLUE}Running standard test suite...${NC}"
    fi

    # Check if test script exists
    if [ ! -f "$test_script" ]; then
        echo -e "${RED}Test script not found: $test_script${NC}"
        return 1
    fi

    # Run the Python test script with output capture
    if python3 "$test_script" 2>&1 | tee "$test_log"; then
        TEST_EXIT_CODE=0
        echo -e "${GREEN}✅ All E2E tests passed!${NC}"
    else
        TEST_EXIT_CODE=$?
        echo -e "${RED}❌ Some E2E tests failed${NC}"
        echo -e "${YELLOW}Test log saved to: $test_log${NC}"
    fi

    # Generate test report if comprehensive mode
    if [ "$TEST_MODE" == "comprehensive" ]; then
        if [ -f "tests/e2e/test_report.json" ]; then
            echo ""
            echo "Test report generated: tests/e2e/test_report.json"

            # Show summary from JSON report
            if command -v jq &> /dev/null; then
                echo ""
                echo "Test Summary:"
                jq '.summary' tests/e2e/test_report.json 2>/dev/null || true
            fi
        fi
    fi

    return $TEST_EXIT_CODE
}

# Clean up resources
cleanup_resources() {
    echo ""
    echo "Cleaning up resources..."

    # Stop and remove test containers
    echo "Removing test containers..."
    docker ps -aq --filter "name=kreuzberg-test" | xargs -r docker rm -f 2>/dev/null || true

    # Clean up dangling images
    echo "Cleaning up dangling images..."
    docker image prune -f 2>/dev/null || true

    # Clean up volumes
    echo "Cleaning up unused volumes..."
    docker volume prune -f 2>/dev/null || true

    if [ "$CLEANUP" == "true" ]; then
        echo "Removing test images..."
        docker rmi kreuzberg:core kreuzberg:easyocr kreuzberg:paddle kreuzberg:gmft 2>/dev/null || true
    fi

    echo -e "${GREEN}✓ Cleanup complete${NC}"
}

# Main execution
main() {
    local start_time=$(date +%s)

    check_prerequisites
    build_images
    run_tests

    # Store test result
    TEST_RESULT=$?

    # Calculate duration
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    echo ""
    echo "----------------------------------------"
    echo "Test suite completed in ${duration} seconds"

    # Cleanup if requested or on failure
    if [ "$CLEANUP" == "true" ] || [ $TEST_RESULT -ne 0 ]; then
        cleanup_resources
    fi

    # Exit with test result
    exit $TEST_RESULT
}

# Show usage information
show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

Enhanced Docker E2E Test Runner for Kreuzberg

OPTIONS:
    --skip-build        Skip building Docker images
    --cleanup           Remove Docker images after tests
    --test-mode MODE    Test mode: 'standard' or 'comprehensive' (default: standard)
    --log-dir DIR       Directory for log files (default: tests/e2e/logs)
    --help              Show this help message

ENVIRONMENT VARIABLES:
    SKIP_BUILD          Set to 'true' to skip builds
    CLEANUP             Set to 'true' to cleanup after tests
    TEST_MODE           Set test mode ('standard' or 'comprehensive')
    LOG_DIR             Set log directory path
    BUILD_ARGS          Additional Docker build arguments

EXAMPLES:
    # Run standard tests with builds
    $0

    # Run comprehensive tests without building
    $0 --skip-build --test-mode comprehensive

    # Build and test with cleanup
    $0 --cleanup

    # Run tests with custom log directory
    $0 --log-dir /tmp/kreuzberg-tests

EOF
}

# Handle script arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --cleanup)
            CLEANUP=true
            shift
            ;;
        --test-mode)
            TEST_MODE="$2"
            if [[ "$TEST_MODE" != "standard" && "$TEST_MODE" != "comprehensive" ]]; then
                echo -e "${RED}Invalid test mode: $TEST_MODE${NC}"
                echo "Valid modes: standard, comprehensive"
                exit 1
            fi
            shift 2
            ;;
        --log-dir)
            LOG_DIR="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo "Run '$0 --help' for usage information"
            exit 1
            ;;
    esac
done

# Run main function
main
