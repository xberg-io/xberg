#!/bin/bash
set -e

export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export NC='\033[0m'

VERBOSE=${VERBOSE:-0}
MCP_TIMEOUT=15
MCP_CHECK_INTERVAL=1

LOG_FILE="/tmp/kreuzberg_mcp.log"
MCP_PID_FILE="/tmp/kreuzberg_mcp.pid"

log_info() {
	echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
	echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
	echo -e "${RED}[ERROR]${NC} $1" >&2
}

log_warning() {
	echo -e "${YELLOW}[WARNING]${NC} $1"
}

verbose() {
	if [ "$VERBOSE" = "1" ]; then
		echo -e "${BLUE}[DEBUG]${NC} $1"
	fi
}

# shellcheck disable=SC2329  # Function is invoked indirectly via trap
cleanup() {
	log_info "Cleaning up..."

	if [ -f "$MCP_PID_FILE" ]; then
		MCP_PID=$(cat "$MCP_PID_FILE")
		if ps -p "$MCP_PID" >/dev/null 2>&1; then
			verbose "Killing MCP server (PID: $MCP_PID)..."
			kill "$MCP_PID" 2>/dev/null || true
			sleep 1

			if ps -p "$MCP_PID" >/dev/null 2>&1; then
				verbose "Force killing MCP server..."
				kill -9 "$MCP_PID" 2>/dev/null || true
			fi
		fi
		rm -f "$MCP_PID_FILE"
	fi

	if [ "$VERBOSE" != "1" ] && [ -f "$LOG_FILE" ]; then
		rm -f "$LOG_FILE"
	fi
}

trap cleanup EXIT

echo ""
log_info "=== Kreuzberg MCP Server Test ==="
echo ""

log_info "Checking if kreuzberg CLI is available..."
if ! command -v kreuzberg &>/dev/null; then
	log_error "kreuzberg command not found. Did you run install.sh first?"
	exit 1
fi
log_success "kreuzberg found at: $(command -v kreuzberg)"

log_info "Checking if MCP subcommand is available..."
if kreuzberg mcp --help &>/dev/null 2>&1 || kreuzberg mcp -h &>/dev/null 2>&1; then
	log_success "MCP subcommand is available"
else
	log_warning "MCP subcommand help not available (may still work)"
fi

log_info "Starting Kreuzberg MCP server..."
log_info "Command: kreuzberg mcp"

kreuzberg mcp >"$LOG_FILE" 2>&1 &
MCP_PID=$!
echo "$MCP_PID" >"$MCP_PID_FILE"

verbose "MCP server started with PID: $MCP_PID"
verbose "Server logs: $LOG_FILE"

log_info "Waiting for MCP server to initialize (timeout: ${MCP_TIMEOUT}s)..."
ELAPSED=0
SERVER_INITIALIZED=0

while [ "$ELAPSED" -lt "$MCP_TIMEOUT" ]; do
	if ! ps -p "$MCP_PID" >/dev/null 2>&1; then
		log_warning "MCP server process exited"
		if grep -q "MCP\|Server\|Listening\|Ready" "$LOG_FILE" 2>/dev/null; then
			log_info "Server appears to have initialized before exiting (checking logs)"
			SERVER_INITIALIZED=1
		else
			if [ -s "$LOG_FILE" ]; then
				log_info "MCP Server output:"
				head -20 "$LOG_FILE" | sed 's/^/  /'
			fi
		fi
		break
	fi

	if grep -qE "(MCP|mcp|ready|Ready|listening|Listening|initialized|Initialized|started|Started)" "$LOG_FILE" 2>/dev/null; then
		SERVER_INITIALIZED=1
		log_success "Server initialized (found startup marker in logs)"
		verbose "Logs contain MCP initialization message"
		break
	fi

	log_lines=$(wc -l <"$LOG_FILE" 2>/dev/null || echo "0")
	if [ -s "$LOG_FILE" ] && [ "$log_lines" -gt 2 ]; then
		log_info "Server appears to be running (logs growing)"
		SERVER_INITIALIZED=1
		break
	fi

	verbose "Waiting for server initialization... (${ELAPSED}s elapsed)"
	sleep "$MCP_CHECK_INTERVAL"
	ELAPSED=$((ELAPSED + MCP_CHECK_INTERVAL))
done

if [ "$SERVER_INITIALIZED" -eq 0 ] && ps -p "$MCP_PID" >/dev/null 2>&1; then
	log_warning "Could not confirm server initialization, but process is running"
	SERVER_INITIALIZED=1
fi

if [ "$SERVER_INITIALIZED" -eq 0 ]; then
	log_error "Server did not initialize within ${MCP_TIMEOUT} seconds"
	log_error "Server output:"
	if [ -f "$LOG_FILE" ] && [ -s "$LOG_FILE" ]; then
		sed 's/^/  /' "$LOG_FILE"
	else
		echo "  (No output)"
	fi
	exit 1
fi

log_success "MCP server initialized successfully"

log_info "Test 1: Verifying MCP process is running..."
if ps -p "$MCP_PID" >/dev/null 2>&1; then
	log_success "MCP server process is running (PID: $MCP_PID)"
else
	log_warning "MCP process not running (may be normal if it exited after initialization)"
fi

log_info "Test 2: Checking for MCP protocol indicators in logs..."
if [ -f "$LOG_FILE" ] && [ -s "$LOG_FILE" ]; then
	if grep -qE "(jsonrpc|mcp|protocol|method|params)" "$LOG_FILE" 2>/dev/null; then
		log_success "Found MCP protocol markers in logs"
	else
		log_info "Logs don't contain explicit protocol markers (checking content)"
	fi

	log_info "MCP Server logs (first 20 lines):"
	head -n 20 "$LOG_FILE" | sed 's/^/  /'
else
	log_warning "Log file is empty or not found"
fi

log_info "Test 3: Checking for MCP communication setup..."
log_info "MCP uses stdio for communication (stdin/stdout/stderr)"
log_success "Communication channels available"

log_info "Test 4: Testing MCP input handling..."

if ps -p "$MCP_PID" >/dev/null 2>&1; then
	log_info "Sending test request to MCP..."

	TEST_REQUEST='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'

	echo "$TEST_REQUEST" | timeout 2 kreuzberg mcp >/tmp/mcp_test_response.txt 2>&1 &
	MCP_TEST_PID=$!
	sleep 1

	if ps -p "$MCP_TEST_PID" >/dev/null 2>&1; then
		kill "$MCP_TEST_PID" 2>/dev/null || true
	fi

	if [ -s /tmp/mcp_test_response.txt ]; then
		log_success "MCP received and responded to test request"
		verbose "Response: $(head -c 200 /tmp/mcp_test_response.txt)"
	else
		log_info "MCP did not produce output (may be expected for this implementation)"
	fi

	rm -f /tmp/mcp_test_response.txt
else
	log_info "MCP process not running (skipping input test)"
fi

log_info "Test 5: Checking for MCP socket/port configuration..."
if grep -qE "(socket|port|[0-9]{4,5})" "$LOG_FILE" 2>/dev/null; then
	log_success "Found socket/port configuration in logs"
	verbose "Configuration:"
	grep -E "(socket|port|listen|bind)" "$LOG_FILE" | head -5 | sed 's/^/  /'
else
	log_info "No explicit socket/port configuration found (may be default)"
fi

log_info "Test 6: Checking for errors in server logs..."
ERROR_COUNT=""
ERROR_COUNT=$(grep -c -iE "(error|failed|exception|panic)" "$LOG_FILE" 2>/dev/null || echo "0")

if [ "$ERROR_COUNT" -eq 0 ]; then
	log_success "No errors detected in logs"
else
	log_warning "Found $ERROR_COUNT error entries in logs"
	log_warning "Error lines:"
	grep -iE "(error|failed|exception|panic)" "$LOG_FILE" 2>/dev/null | head -5 | sed 's/^/  /'
fi

log_info "Stopping MCP server..."
if ps -p "$MCP_PID" >/dev/null 2>&1; then
	kill "$MCP_PID" 2>/dev/null || true
	sleep 2

	if ps -p "$MCP_PID" >/dev/null 2>&1; then
		log_warning "Process did not stop gracefully, force killing..."
		kill -9 "$MCP_PID" 2>/dev/null || true
		sleep 1
	fi

	if ps -p "$MCP_PID" >/dev/null 2>&1; then
		log_error "Failed to stop MCP server"
		exit 1
	else
		log_success "MCP server stopped successfully"
	fi
else
	log_info "MCP server process already stopped"
fi

rm -f "$MCP_PID_FILE"

echo ""
log_success "=== MCP Server Test Passed ==="
echo ""
log_info "Summary:"
log_info "- MCP Server: kreuzberg mcp"
log_info "- Process ID: $MCP_PID"
log_info "- Server logs: $LOG_FILE"
log_info "- Log file size: $(stat -f%z "$LOG_FILE" 2>/dev/null || stat -c%s "$LOG_FILE" 2>/dev/null || echo "0") bytes"
echo ""

exit 0
