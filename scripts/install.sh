#!/usr/bin/env bash
#   XBERG_VERSION    - Specific version to install (default: latest)
#   XBERG_LINUX_LIBC - Linux archive variant: musl or gnu (default: auto)

set -euo pipefail

REPO="xberg-io/xberg"
BINARY_NAME="xberg"

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

detect_libc() {
  if [ -e /lib/ld-musl-x86_64.so.1 ] || [ -e /lib/ld-musl-aarch64.so.1 ]; then
    echo musl
  else
    local version
    version="$(ldd --version 2>&1 || true)"
    case "$version" in *musl*) echo musl ;; *) echo gnu ;; esac
  fi
}

detect_target() {
  local os="$1" arch="$2" libc
  case "$os" in
  linux)
    libc="${XBERG_LINUX_LIBC:-$(detect_libc)}"
    case "$libc" in
    musl | gnu) echo "${arch}-unknown-linux-${libc}" ;;
    *) error "XBERG_LINUX_LIBC must be musl or gnu" ;;
    esac
    ;;
  darwin) echo "${arch}-apple-darwin" ;;
  esac
}

fetch_release_metadata() (
  set +x
  local url="https://api.github.com/repos/${REPO}/releases/$1" output="$2"
  if [ -n "${GITHUB_TOKEN:-}" ]; then
    case "$GITHUB_TOKEN" in
    *[!A-Za-z0-9_]*) error "GITHUB_TOKEN contains invalid characters" ;;
    esac
    # Do not put the token on the command line or follow redirects with it. ~keep
    printf 'Authorization: Bearer %s\n' "$GITHUB_TOKEN" |
      curl -q -fsS --proto '=https' --proto-redir '=https' --header @- "$url" -o "$output"
  else
    curl -q -fsS --proto '=https' --proto-redir '=https' "$url" -o "$output"
  fi
)

read_release_metadata() {
  local metadata="$1" artifact="${2:-}"
  # Tokenize JSON strings intact, then read only direct fields of objects in .assets. ~keep
  LC_ALL=C awk -v asset="$artifact" '
    BEGIN { separator = "\036" }
    {
      gsub(/"(\\.|[^"\\])*"|[{}:,]|\[|\]|true|false|null|-?[0-9][0-9.eE+-]*/, "&" separator)
      count = split($0, tokens, separator)
      for (i = 1; i <= count; i++) {
        token = tokens[i]
        sub(/^[[:space:]]+/, "", token)
        if (token == "") continue

        if (depth == 1 && previous == ":" && key == "\"tag_name\"") {
          tag_fields++
          tag = token
        } else if (depth == 1 && previous == ":" && key == "\"assets\"") {
          assets_fields++
          in_assets = (token == "[")
        } else if (in_assets && depth == 2 && token == "{") {
          selected = name_fields = digest_fields = 0
          digest = ""
        } else if (in_assets && depth == 3) {
          if (token == "}" && selected) {
            matches++
            if (name_fields == 1 && digest_fields == 1) expected = digest
          } else if (previous == ":" && key == "\"name\"") {
            name_fields++
            if (token == "\"" asset "\"") selected = 1
          } else if (previous == ":" && key == "\"digest\"") {
            digest_fields++
            digest = token
          }
        } else if (in_assets && depth == 2 && token == "]") {
          in_assets = 0
        }

        if (token == "{" || token == "[") depth++
        if (token == "}" || token == "]") depth--
        key = previous
        previous = token
      }
    }
    END {
      if (depth != 0) exit 1
      if (asset == "") {
        if (tag_fields != 1 || tag !~ /^"v[0-9][0-9A-Za-z._+-]*"$/) exit 1
        print substr(tag, 2, length(tag) - 2)
        exit
      }
      if (assets_fields != 1 || matches != 1 || expected !~ /^"sha256:[[:xdigit:]]+"$/) exit 1
      sub(/^"sha256:/, "", expected)
      sub(/"$/, "", expected)
      if (length(expected) != 64) exit 1
      print tolower(expected)
    }
  ' "$metadata"
}

verify_archive() {
  local artifact="$1" archive="$2" metadata="$3" expected actual
  shift 3
  expected="$(read_release_metadata "$metadata" "$artifact")" ||
    error "no valid SHA-256 digest for ${artifact} in GitHub release metadata"
  actual="$("$@" <"$archive")" || error "failed to compute SHA-256 for ${artifact}"
  [ "${actual%% *}" = "$expected" ] || error "SHA-256 mismatch for ${artifact}"
}

install() {
  need_cmd curl
  need_cmd tar
  need_cmd awk

  local -a hash_command
  if command -v sha256sum >/dev/null 2>&1; then
    hash_command=(sha256sum)
  elif command -v shasum >/dev/null 2>&1; then
    hash_command=(shasum -a 256)
  else
    error "need 'sha256sum' or 'shasum' to verify release archives"
  fi

  local os arch target version install_dir

  os="$(detect_os)"
  arch="$(detect_arch)"
  target="$(detect_target "$os" "$arch")"

  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT
  local metadata="$tmpdir/release.json"

  if [ -n "${XBERG_VERSION:-}" ]; then
    version="${XBERG_VERSION}"
    case "$version" in
    v*) ;;
    *) version="v${version}" ;;
    esac
  else
    info "Fetching latest release..."
    fetch_release_metadata latest "$metadata" ||
      error "failed to fetch latest release metadata from GitHub"
    version="$(read_release_metadata "$metadata")" ||
      error "failed to fetch latest release tag from GitHub"
  fi

  info "Installing xberg ${version} for ${target}"

  if [ -n "${XBERG_INSTALL:-}" ]; then
    install_dir="${XBERG_INSTALL}"
  elif [ "$(id -u)" -eq 0 ]; then
    install_dir="/usr/local/bin"
  else
    install_dir="${HOME}/.xberg/bin"
  fi

  mkdir -p "$install_dir"

  local stage_dir binary_path version_output artifact url http_status status
  while :; do
    artifact="xberg-cli-${target}.tar.gz"
    url="https://github.com/${REPO}/releases/download/${version}/${artifact}"
    info "Downloading ${url}"
    if http_status="$(curl -q -fsSL --proto '=https' --proto-redir '=https' -w '%{http_code}' "$url" -o "${tmpdir}/${artifact}")"; then
      if [ ! -f "$metadata" ]; then
        fetch_release_metadata "tags/${version}" "$metadata" ||
          error "failed to fetch SHA-256 metadata for ${version} from GitHub"
      fi
      verify_archive "$artifact" "${tmpdir}/${artifact}" "$metadata" "${hash_command[@]}"
      tar -xzf "${tmpdir}/${artifact}" -C "$tmpdir"
      stage_dir="${tmpdir}/xberg-cli-${target}"
      binary_path="${stage_dir}/${BINARY_NAME}"
      [ -f "$binary_path" ] || error "binary not found in archive at ${binary_path}"
      if [ "$os" != linux ]; then
        break
      fi
      chmod +x "$binary_path"
      if [ -f "${stage_dir}/${BINARY_NAME}.bin" ]; then
        chmod +x "${stage_dir}/${BINARY_NAME}.bin"
      fi
      if version_output="$("$binary_path" --version 2>&1)"; then
        break
      fi
      if [ -n "${XBERG_LINUX_LIBC:-}" ] || [[ "$target" != *-linux-gnu ]]; then
        error "archive for ${target} cannot run on this host: ${version_output}"
      fi
      warn "Archive for ${target} cannot run on this host: ${version_output}"
    else
      status=$?
      if [ "$status" != 22 ] || [ "$http_status" != 404 ] ||
        [ -n "${XBERG_LINUX_LIBC:-}" ] || [[ "$target" != *-linux-gnu ]]; then
        exit "$status"
      fi
      warn "Archive for ${target} is unavailable (HTTP 404)"
    fi
    target="${arch}-unknown-linux-musl"
    info "Trying ${target}"
  done

  if [ "$os" = linux ]; then
    local library
    for library in "${stage_dir}"/*.so "${stage_dir}"/*.so.*; do
      [ -f "$library" ] || continue
      cp "$library" "$install_dir/"
    done
  fi

  if [ -f "${stage_dir}/${BINARY_NAME}.bin" ]; then
    cp "${stage_dir}/${BINARY_NAME}.bin" "${install_dir}/${BINARY_NAME}.bin"
    chmod +x "${install_dir}/${BINARY_NAME}.bin"
  fi

  if [ -d "${stage_dir}/lib" ] && [ "$(ls -A "${stage_dir}/lib" 2>/dev/null)" ]; then
    mkdir -p "${install_dir}/lib"
    # -R so nested library directories (e.g. lib/libheif) are copied, not skipped
    cp -R "${stage_dir}/lib/"* "${install_dir}/lib/"
    info "Installed runtime libraries to ${install_dir}/lib/"
  fi

  for dylib in "${stage_dir}"/*.dylib; do
    [ -f "$dylib" ] || continue
    cp "$dylib" "${install_dir}/"
    info "Installed runtime library $(basename "$dylib") to ${install_dir}/"
  done

  cp "$binary_path" "${install_dir}/${BINARY_NAME}"
  chmod +x "${install_dir}/${BINARY_NAME}"

  if version_output="$("${install_dir}/${BINARY_NAME}" --version 2>&1)"; then
    info "Installed ${BINARY_NAME} to ${install_dir}/${BINARY_NAME}"
    info "Verified: ${version_output}"
  elif [ "$os" = linux ]; then
    error "installed CLI failed verification: ${version_output}"
  else
    info "Installed ${BINARY_NAME} to ${install_dir}/${BINARY_NAME}"
    warn "Binary installed but --version check failed"
  fi

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
