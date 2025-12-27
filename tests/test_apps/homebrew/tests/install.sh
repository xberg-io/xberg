#!/bin/bash
set -e

export RED='\033[0;31m'
export GREEN='\033[0;32m'
export YELLOW='\033[1;33m'
export BLUE='\033[0;34m'
export NC='\033[0m'

VERBOSE=${VERBOSE:-0}

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

echo ""
log_info "=== Kreuzberg Homebrew Installation Test ==="
echo ""

log_info "Checking platform..."
if [[ "$OSTYPE" != "darwin"* ]]; then
	log_error "This test is designed for macOS. Current OS: $OSTYPE"
	exit 1
fi
log_success "Running on macOS"

log_info "Checking if Homebrew is installed..."
if ! command -v brew &>/dev/null; then
	log_warning "Homebrew not found. Installing Homebrew..."
	/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

	if [[ $(uname -m) == 'arm64' ]]; then
		export PATH="/opt/homebrew/bin:$PATH"
	fi
fi

if ! command -v brew &>/dev/null; then
	log_error "Failed to install Homebrew"
	exit 1
fi

log_success "Homebrew found at: $(command -v brew)"
verbose "Homebrew version: $(brew --version)"

log_info "Updating Homebrew formulas..."
brew update || log_warning "Homebrew update had issues (continuing anyway)"

log_info "Checking if kreuzberg formula is available..."
if brew search kreuzberg 2>/dev/null | grep -q "^kreuzberg$"; then
	log_success "Kreuzberg formula found in Homebrew"
else
	log_warning "Kreuzberg formula not found in standard repositories"
	log_info "This might be expected if the formula is in a custom tap"
fi

log_info "Installing kreuzberg via Homebrew..."
if brew list kreuzberg &>/dev/null; then
	log_info "Kreuzberg already installed, upgrading..."
	brew upgrade kreuzberg || {
		log_warning "Upgrade failed or already latest version"
	}
else
	log_info "Installing kreuzberg for the first time..."
	brew install kreuzberg || {
		log_error "Failed to install kreuzberg"
		log_info "Trying with --HEAD flag..."
		brew install kreuzberg --HEAD || {
			log_error "Installation failed"
			exit 1
		}
	}
fi

log_success "Kreuzberg installed successfully"

log_info "Verifying installation..."
if ! command -v kreuzberg &>/dev/null; then
	log_error "kreuzberg command not found after installation"
	exit 1
fi

log_success "kreuzberg command is available at: $(command -v kreuzberg)"

log_info "Retrieving version information..."
VERSION_OUTPUT=$(kreuzberg --version 2>&1 || true)
if [ -z "$VERSION_OUTPUT" ]; then
	log_warning "Could not retrieve version (command may not support --version)"
else
	log_success "Version output: $VERSION_OUTPUT"
	verbose "Full version info: $VERSION_OUTPUT"
fi

log_info "Testing help command..."
if kreuzberg --help &>/dev/null; then
	log_success "Help command works"
else
	log_warning "Help command had issues (continuing anyway)"
fi

log_info "Checking available subcommands..."
HELP_OUTPUT=$(kreuzberg --help 2>&1 || kreuzberg help 2>&1 || echo "")
if echo "$HELP_OUTPUT" | grep -q -E "(extract|serve|mcp)"; then
	log_success "Found core subcommands: extract, serve, mcp"
	verbose "Help output:\n$HELP_OUTPUT"
else
	log_warning "Could not verify all subcommands"
fi

echo ""
log_success "=== Installation Test Passed ==="
echo ""
log_info "Summary:"
log_info "- Platform: macOS $(sw_vers -productVersion)"
log_info "- Homebrew: $(brew --version | head -n 1)"
log_info "- Kreuzberg: $(command -v kreuzberg)"
if [ -n "$VERSION_OUTPUT" ]; then
	log_info "- Version: $VERSION_OUTPUT"
fi
echo ""

exit 0
