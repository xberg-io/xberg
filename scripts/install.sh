#!/usr/bin/env bash
# Kreuzberg CLI installer
# Usage: curl -fsSL https://kreuzberg.dev/install.sh | bash
#
# Environment variables:
#   KREUZBERG_VERSION  - Specific version to install (default: latest)
#   KREUZBERG_INSTALL  - Installation directory (default: ~/.kreuzberg/bin or /usr/local/bin)

set -euo pipefail

REPO="kreuzberg-dev/kreuzberg"
BINARY_NAME="kreuzberg"

# --- Helpers ---

info() { printf '\033[1;34m%s\033[0m\n' "$*"; }
warn() { printf '\033[1;33m%s\033[0m\n' "$*" >&2; }
error() {
  printf '\033[1;31merror: %s\033[0m\n' "$*" >&2
  exit 1
}

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    error "need '$1' (command not found)"
  fi
}

# --- Detect platform ---

detect_os() {
  local os
  os="$(uname -s)"
  case "$os" in
  Linux*) echo "linux" ;;
  Darwin*) echo "darwin" ;;
  *) error "unsupported OS: $os" ;;
  esac
}

detect_arch() {
  local arch
  arch="$(uname -m)"
  case "$arch" in
  x86_64 | amd64) echo "x86_64" ;;
  aarch64 | arm64) echo "aarch64" ;;
  *) error "unsupported architecture: $arch" ;;
  esac
}

detect_target() {
  local os arch
  os="$(detect_os)"
  arch="$(detect_arch)"

  case "${os}-${arch}" in
  linux-x86_64) echo "x86_64-unknown-linux-musl" ;;
  linux-aarch64) echo "aarch64-unknown-linux-musl" ;;
  darwin-x86_64) echo "aarch64-apple-darwin" ;; # Rosetta compatible
  darwin-aarch64) echo "aarch64-apple-darwin" ;;
  *) error "unsupported platform: ${os}-${arch}" ;;
  esac
}

# --- Version resolution ---

get_latest_version() {
  need_cmd curl

  # List recent releases and pick the first tag starting with "v" (skip benchmark runs etc.)
  local url="https://api.github.com/repos/${REPO}/releases?per_page=20"
  local tag
  tag="$(curl -fsSL "$url" | grep '"tag_name"' | sed 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/' | grep '^v' | head -1 || true)"

  if [ -z "$tag" ]; then
    error "failed to fetch latest release tag from GitHub"
  fi
  echo "$tag"
}

# --- Download and install ---

install() {
  need_cmd curl
  need_cmd tar

  local os arch target version install_dir

  os="$(detect_os)"
  arch="$(detect_arch)"
  target="$(detect_target)"

  if [ -n "${KREUZBERG_VERSION:-}" ]; then
    version="${KREUZBERG_VERSION}"
    # Ensure 'v' prefix
    case "$version" in
    v*) ;;
    *) version="v${version}" ;;
    esac
  else
    info "Fetching latest release..."
    version="$(get_latest_version)"
  fi

  info "Installing kreuzberg ${version} for ${target}"

  # Determine install directory
  if [ -n "${KREUZBERG_INSTALL:-}" ]; then
    install_dir="${KREUZBERG_INSTALL}"
  elif [ "$(id -u)" -eq 0 ]; then
    install_dir="/usr/local/bin"
  else
    install_dir="${HOME}/.kreuzberg/bin"
  fi

  mkdir -p "$install_dir"

  # Download
  local artifact="kreuzberg-cli-${target}.tar.gz"
  local url="https://github.com/${REPO}/releases/download/${version}/${artifact}"

  info "Downloading ${url}"

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  curl -fsSL "$url" -o "${tmpdir}/${artifact}"

  # Extract
  tar -xzf "${tmpdir}/${artifact}" -C "$tmpdir"

  # Install binary
  local stage_dir="${tmpdir}/kreuzberg-cli-${target}"
  local binary_path="${stage_dir}/${BINARY_NAME}"
  if [ ! -f "$binary_path" ]; then
    error "binary not found in archive at ${binary_path}"
  fi

  cp "$binary_path" "${install_dir}/${BINARY_NAME}"
  chmod +x "${install_dir}/${BINARY_NAME}"

  # Install the actual binary (musl builds use wrapper + .bin)
  if [ -f "${stage_dir}/${BINARY_NAME}.bin" ]; then
    cp "${stage_dir}/${BINARY_NAME}.bin" "${install_dir}/${BINARY_NAME}.bin"
    chmod +x "${install_dir}/${BINARY_NAME}.bin"
  fi

  # Install bundled runtime libraries (musl builds only)
  if [ -d "${stage_dir}/lib" ] && [ "$(ls -A "${stage_dir}/lib" 2>/dev/null)" ]; then
    mkdir -p "${install_dir}/lib"
    cp "${stage_dir}/lib/"* "${install_dir}/lib/"
    info "Installed runtime libraries to ${install_dir}/lib/"
  fi

  info "Installed ${BINARY_NAME} to ${install_dir}/${BINARY_NAME}"

  # Verify
  if "${install_dir}/${BINARY_NAME}" --version >/dev/null 2>&1; then
    info "Verified: $("${install_dir}/${BINARY_NAME}" --version)"
  else
    warn "Binary installed but --version check failed"
  fi

  # PATH hint
  case ":${PATH}:" in
  *":${install_dir}:"*) ;;
  *)
    warn ""
    warn "Add ${install_dir} to your PATH:"
    warn ""
    warn "  export PATH=\"${install_dir}:\$PATH\""
    warn ""
    warn "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.) to make it permanent."
    ;;
  esac
}

install
