#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/retry.sh"

echo "::group::Installing Linux dependencies"

echo "Updating package index..."
if ! retry_with_backoff sudo apt-get update; then
  echo "::warning::apt-get update failed after retries, continuing anyway..."
fi

packages=(
  tesseract-ocr
  tesseract-ocr-eng
  tesseract-ocr-tur
  tesseract-ocr-deu
  # Korean and vertical-Japanese packs back the multilingual PNG (eng+kor) and
  # vertical JPEG (jpn_vert) OCR benchmark fixtures. The harness OCR preflight
  # fails fast unless these resolve on disk, so they must be installed here
  # rather than downloaded inside the timed extraction. ~keep
  tesseract-ocr-kor
  tesseract-ocr-jpn-vert
  libtesseract-dev
  libleptonica-dev
  fonts-liberation
  fonts-dejavu-core
  fonts-noto-core
  libssl-dev
  pkg-config
  build-essential
  python3
  libpng-dev
  libjpeg-dev
  libtiff-dev
  libgif-dev
  libwebp-dev
  libopenjp2-7-dev
  libarchive-dev
  libcurl4-openssl-dev
  patchelf
  cmake
  libmagic-dev
  libuv1-dev
  libde265-dev
  libaom-dev
  aom-tools
  libyuv-dev
  libwebm-dev
  libx265-dev
  libdav1d-dev
  libnuma-dev
  # boost (header-only spirit) and zlib headers are build-time deps of
  # librevenge + libwpd, compiled from source by the xberg-libwpd crate. ~keep
  libboost-dev
  zlib1g-dev
  # liblzma-dev provides the liblzma.so linker symlink. The swift package
  # statically links libxberg_ffi.a, whose lzma-sys transitive dep surfaces
  # `-llzma` at the swift link step; the runner ships liblzma5 (runtime) but
  # not the dev symlink, so ld.gold fails with "cannot find -llzma". ~keep
  liblzma-dev
  # libbz2-dev provides the libbz2.so linker symlink. Same reasoning as
  # liblzma-dev: the swift package's bzip2 crates (archive/zip/unhwp paths)
  # emit `-lbz2`; the runner ships libbz2 runtime but not the dev symlink, so
  # ld.gold fails with "cannot find -lbz2". ~keep
  libbz2-dev
)

# Cached libheif can require SharpYUV's pkg-config metadata. Older distributions bundle
# WebP without a separate SharpYUV package, so keep their apt batch installable. ~keep
if apt-cache show libsharpyuv-dev >/dev/null 2>&1; then
  packages+=(libsharpyuv-dev)
fi

echo "Installing dependencies..."
if retry_with_backoff_timeout 900 sudo apt-get install -y "${packages[@]}"; then
  echo "✓ All packages installed successfully"
else
  exit_code=$?
  if [ $exit_code -eq 124 ]; then
    echo "::error::Package installation timed out after 15 minutes"
  else
    echo "::warning::Some packages failed to install, attempting individual installs..."
    for pkg in tesseract-ocr libtesseract-dev libleptonica-dev libssl-dev pkg-config cmake; do
      echo "Installing $pkg..."
      if retry_with_backoff_timeout 300 sudo apt-get install -y "$pkg" 2>&1; then
        echo "  ✓ $pkg installed"
      else
        echo "  ⚠ Failed to install $pkg"
      fi
    done
  fi
fi

# Install PHP dev headers only when no PHP is already active. The php-extension
# build matrix runs shivammathur/setup-php first (matrix.php), putting phpX.Y and
# its -dev headers on PATH; apt's unversioned php-cli/php-dev pull Ubuntu Noble's
# default (8.3) and reset php-config, so ext-php-rs builds against the wrong PHP
# version. Guard on `command -v php`, mirroring the Windows/macOS scripts. ~keep
if command -v php >/dev/null 2>&1; then
  echo "✓ PHP already active: $(php --version | head -1)"
else
  echo "Installing PHP (php-cli, php-dev)..."
  retry_with_backoff_timeout 300 sudo apt-get install -y php-cli php-dev ||
    echo "::warning::Failed to install php-cli/php-dev"
fi

echo "::endgroup::"

echo "::group::Building the selected libheif runtime"

# Keep Rust's v1_19 API floor while exercising current security fixes by default.
# Select 1.19.8 explicitly only for minimum-runtime ABI qualification; that old
# upstream source is not a security-qualified production dependency. ~keep
LIBHEIF_VERSION="${LIBHEIF_VERSION:-1.23.7}"
case "$LIBHEIF_VERSION" in
1.23.7) LIBHEIF_SHA256=f7483e818503ea6224c8863e4a56ac02c7089021a98472c811c12aa19faf2475 ;;
1.19.8) LIBHEIF_SHA256=6c4a5b08e6eae66d199977468859dea3b5e059081db8928f7c7c16e53836c906 ;;
*)
  echo "::error::Unverified libheif version: $LIBHEIF_VERSION" >&2
  exit 1
  ;;
esac
LIBHEIF_PREFIX="${LIBHEIF_PREFIX:-/usr/local}"

echo "Removing apt's libheif to prevent shadowing..."
if dpkg -l | grep -q "^ii.*libheif"; then
  sudo apt-get remove -y libheif* || echo "::warning::Failed to remove apt libheif, continuing..."
else
  echo "✓ apt libheif not installed"
fi

libheif_search_path() {
  local result="$1" entry
  local -a entries
  IFS=: read -r -a entries <<<"$2"
  for entry in "${entries[@]}"; do
    if [ -n "$entry" ]; then
      result+=":$entry"
    fi
  done
  printf '%s\n' "$result"
}

libheif_runtime_links() {
  local library="$1/lib/libheif.so.1" elf report runtime_path
  if ! elf="$(LC_ALL=C readelf -h -d "$library" 2>&1)" ||
    ! grep -Eq '^[[:space:]]*Type:[[:space:]]+DYN' <<<"$elf" ||
    ! grep -Eq '\(SONAME\).*\[libheif\.so\.1\]' <<<"$elf"; then
    echo "::warning::Expected a libheif.so.1 ELF shared library at $library." >&2
    return 1
  fi
  if grep -Eq '\((RPATH|RUNPATH)\)' <<<"$elf"; then
    echo "::warning::$library has an embedded RPATH or RUNPATH." >&2
    return 1
  fi
  runtime_path="${LD_LIBRARY_PATH:-}"
  runtime_path="$(libheif_search_path "$1/lib" "${runtime_path//;/:}")"
  if ! report="$(LC_ALL=C LD_LIBRARY_PATH="$runtime_path" ldd -r "$library" 2>&1)" ||
    grep -Eq 'not found|undefined symbol' <<<"$report" ||
    ! grep -Eq '^[[:space:]]*[^[:space:]]+ => /[^[:space:]]+ \(0x[[:xdigit:]]+\)' <<<"$report"; then
    echo "::warning::Cannot verify runtime dependencies for $library." >&2
    printf '%s\n' "$report" >&2
    return 1
  fi
}

libheif_artifacts() (
  local root="$1" entry target runtime
  local -a artifacts
  [[ ! -L "$root" ]] || return 1
  cd "$root" || return 1
  for entry in lib lib/pkgconfig include include/libheif lib/cmake lib/cmake/libheif share share/libheif; do
    [[ ! -L "$entry" && (! -e "$entry" || -d "$entry") ]] || return 1
  done

  runtime="$(realpath lib/libheif.so.1)" || return 1
  [[ "${runtime%/*}" == "$(pwd -P)/lib" ]] || return 1
  runtime="${runtime##*/}"
  [[ "$runtime" == libheif.so.1 || "$runtime" =~ ^libheif\.so\.1(\.[0-9]+)+$ ]] || return 1
  [[ -f "lib/$runtime" && ! -L "lib/$runtime" ]] || return 1
  artifacts=("lib/$runtime")
  for entry in lib/libheif.so.1 lib/libheif.so; do
    [[ "$entry" != "lib/$runtime" ]] || continue
    target="$(readlink "$entry")" || return 1
    [[ "$target" == "$runtime" || ("$entry" == lib/libheif.so && "$target" == libheif.so.1) ]] || return 1
    artifacts+=("$entry")
  done

  shopt -s nullglob
  for entry in lib/pkgconfig/libheif.pc include/libheif/*.h lib/cmake/libheif/libheif-config*.cmake \
    share/libheif/example.heic share/libheif/example.avif; do
    [[ ! -L "$entry" ]] || return 1
    [[ -e "$entry" ]] || continue
    [[ -f "$entry" ]] || return 1
    artifacts+=("$entry")
  done
  printf '%s\0' "${artifacts[@]}"
)

promote_libheif() (
  local source_root="$1" destination="$2" entry manifest
  local -a artifacts=()
  manifest="$(mktemp)" || return 1
  trap 'rm -f -- "$manifest"' EXIT
  libheif_artifacts "$source_root" >"$manifest" || return 1
  while IFS= read -r -d '' entry; do
    artifacts+=("$entry")
  done <"$manifest"
  for entry in "$destination" "$destination"/{lib,lib/pkgconfig,lib/cmake,lib/cmake/libheif,include,include/libheif,share,share/libheif}; do
    [[ ! -L "$entry" && (! -e "$entry" || -d "$entry") ]] || return 1
  done
  cd "$source_root" || return 1
  sudo mkdir -p "$destination" &&
    sudo cp -P --parents --remove-destination -- "${artifacts[@]}" "$destination/"
)

probe_libheif_artifacts() (
  local source_root="$1" staging entry
  staging="$(mktemp -d)" || return 1
  trap 'rm -rf -- "$staging"' EXIT
  libheif_artifacts "$source_root" >"$staging/manifest" || return 1
  while IFS= read -r -d '' entry; do
    mkdir -p "$staging/${entry%/*}" &&
      cp -P -- "$source_root/$entry" "$staging/$entry" || return 1
  done <"$staging/manifest"
  cd "$staging" || return 1
  LD_LIBRARY_PATH='' libheif_runtime_links "$staging"
)

libheif_candidate_is_usable() {
  local candidate="$1" marker="$1/lib/pkgconfig/libheif.pc" search_path
  if [[ ! -s "$candidate/include/libheif/heif.h" || ! -f "$candidate/lib/libheif.so" ||
    ! -f "$candidate/lib/libheif.so.1" || ! -s "$marker" ]]; then
    echo "::warning::Incomplete libheif candidate at $candidate." >&2
    return 1
  fi
  if ! libheif_artifacts "$candidate" >/dev/null; then
    echo "::warning::Unsafe libheif artifact paths in $candidate." >&2
    return 1
  fi
  search_path="$(libheif_search_path "$candidate/lib/pkgconfig" "${PKG_CONFIG_PATH:-}")"
  [[ "$(PKG_CONFIG_PATH="$search_path" pkg-config --modversion "$marker")" == "$LIBHEIF_VERSION" ]] &&
    PKG_CONFIG_PATH="$search_path" pkg-config --cflags --libs "$marker" &&
    probe_libheif_artifacts "$candidate"
}

LIBHEIF_CACHE=/tmp/libheif-cache
if [[ -n "${GITHUB_ACTION:-}" ]] && libheif_candidate_is_usable "$LIBHEIF_CACHE"; then
  candidate="$LIBHEIF_CACHE"
  echo "✓ Reusing cached libheif $LIBHEIF_VERSION"
elif [[ -z "${GITHUB_ACTION:-}" ]] && libheif_candidate_is_usable "$LIBHEIF_PREFIX"; then
  candidate="$LIBHEIF_PREFIX"
else
  echo "Building libheif ${LIBHEIF_VERSION} from source..."
  build_dir="$(mktemp -d)"
  trap 'rm -rf -- "$build_dir"' EXIT
  pushd "$build_dir" >/dev/null

  if retry_with_backoff_timeout 300 curl -fsSL -o libheif.tar.gz \
    "https://github.com/strukturag/libheif/releases/download/v${LIBHEIF_VERSION}/libheif-${LIBHEIF_VERSION}.tar.gz"; then
    printf '%s  libheif.tar.gz\n' "$LIBHEIF_SHA256" | sha256sum -c -
    tar xzf libheif.tar.gz
    cd "libheif-${LIBHEIF_VERSION}"
    mkdir build
    cd build
    cmake .. \
      -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_INSTALL_PREFIX="$LIBHEIF_PREFIX" \
      -DCMAKE_INSTALL_LIBDIR=lib \
      -DCMAKE_SKIP_INSTALL_RPATH=ON \
      -DWITH_EXAMPLES=OFF \
      -DWITH_GDK_PIXBUF=OFF \
      -DBUILD_TESTING=OFF
    make -j"$(nproc)"
    make DESTDIR="$build_dir/staged" install
  else
    echo "::error::Failed to download libheif source"
    exit 1
  fi

  popd >/dev/null
  candidate="$build_dir/staged$LIBHEIF_PREFIX"
  if ! libheif_candidate_is_usable "$candidate"; then
    echo "::error::The freshly built libheif failed validation."
    exit 1
  fi
fi

if [[ "$candidate" != "$LIBHEIF_PREFIX" ]]; then
  promote_libheif "$candidate" "$LIBHEIF_PREFIX"
fi
sudo ldconfig
if ! libheif_candidate_is_usable "$LIBHEIF_PREFIX"; then
  echo "::error::The installed libheif failed runtime validation."
  exit 1
fi

if [[ -n "${GITHUB_ACTION:-}" && "$candidate" != "$LIBHEIF_CACHE" ]]; then
  # Seed from the fresh DESTDIR, which contains only this build's files. ~keep
  rm -rf -- "$LIBHEIF_CACHE"
  mv "$candidate" "$LIBHEIF_CACHE"
fi

echo "ldconfig cache contains:"
ldconfig -p | grep libheif || echo "(no libheif in ldconfig cache)"

if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "PKG_CONFIG_PATH=$(libheif_search_path "$LIBHEIF_PREFIX/lib/pkgconfig" "${PKG_CONFIG_PATH:-}")" >>"$GITHUB_ENV"
  runtime_path="${LD_LIBRARY_PATH:-}"
  echo "LD_LIBRARY_PATH=$(libheif_search_path "$LIBHEIF_PREFIX/lib" "${runtime_path//;/:}")" >>"$GITHUB_ENV"
fi

echo "::endgroup::"

echo "::group::Verifying Linux installations"

echo "CMake:"
if command -v cmake >/dev/null 2>&1; then
  cmake --version | head -1
  echo "✓ CMake available"
  CMAKE_FULL_PATH="$(command -v cmake)"
  if [[ -n "$GITHUB_ENV" ]]; then
    echo "CMAKE=$CMAKE_FULL_PATH" >>"$GITHUB_ENV"
    echo "✓ Set CMAKE=$CMAKE_FULL_PATH in GITHUB_ENV"
  fi
  CMAKE_BIN="$(dirname "$CMAKE_FULL_PATH")"
  if [[ -n "$GITHUB_PATH" && -d "$CMAKE_BIN" ]]; then
    echo "$CMAKE_BIN" >>"$GITHUB_PATH"
    echo "✓ Added cmake directory to GITHUB_PATH: $CMAKE_BIN"
  fi
else
  echo "::error::CMake not found after installation"
  exit 1
fi

echo ""
echo "Tesseract:"
if command -v tesseract >/dev/null 2>&1; then
  # Full output, not `head -1`: `tesseract --version` reports the engine
  # version on line 1 but the *compiled* leptonica version plus the linked
  # image-codec library versions (libpng/libjpeg/zlib/...) on the following
  # lines. Truncating to line 1 was throwing away exactly the evidence needed
  # to tell "same tesseract, different leptonica" apart from "same everything".
  if tesseract --version 2>&1; then
    echo "✓ Tesseract CLI available"
  else
    echo "::warning::Tesseract CLI present but failed to run"
  fi
else
  echo "::warning::Tesseract CLI not found; continuing (OCR will rely on bundled Tesseract)"
fi

# --- diagnostics for task #494 (word_language_is_forwarded_per_ocr_element /
# paragraph-metadata: an arch-dependent Tesseract FFI declination, observed on
# x86_64-linux CI only, that production code already treats as legitimate --
# see crates/xberg/tests/issue_177_180_189_ocr_metadata.rs). The install
# script previously asserted only that the same *apt install command* ran on
# every arch, never what it actually resolved to. apt mirrors serve
# independent per-architecture package builds, so "same command" does not
# imply "same tesseract-ocr/libtesseract/liblept build or even version" --
# print the resolved package versions and binary checksums explicitly so two
# runs (ubuntu-latest vs ubuntu-24.04-arm) can be diffed directly instead of
# re-deriving this from a guess. ~keep
echo ""
echo "Runner identity (for cross-arch diffing):"
uname -a
[ -f /etc/os-release ] && cat /etc/os-release

echo ""
echo "Resolved package versions (tesseract/leptonica), this runner:"
dpkg-query -W -f='${Package}\t${Version}\t${Architecture}\n' \
  'tesseract-ocr*' 'libtesseract*' 'liblept*' 2>/dev/null ||
  echo "(dpkg-query found no matching packages)"

echo ""
echo "Available Tesseract languages (full list, not truncated):"
if command -v tesseract >/dev/null 2>&1; then
  tesseract --list-langs || true
else
  echo "(tesseract CLI not available)"
fi

echo ""
echo "PHP:"
if command -v php >/dev/null 2>&1; then
  php --version | head -1
  echo "✓ PHP available"
else
  echo "::error::PHP not found after installation"
  exit 1
fi

echo ""
echo "Checking Tesseract data path..."

tessdata_found=0
for tessdata_path in "/usr/share/tesseract-ocr/5/tessdata" "/usr/share/tesseract-ocr/tessdata"; do
  if [ -d "$tessdata_path" ]; then
    echo "Found tessdata at: $tessdata_path"

    echo "Required language files:"
    for lang in eng tur deu; do
      if [ -f "$tessdata_path/${lang}.traineddata" ]; then
        size=$(stat -c%s "$tessdata_path/${lang}.traineddata" 2>/dev/null || echo "unknown")
        # sha256, not just size: two traineddata files can happen to match in
        # byte count while differing in content. A byte-identical hash across
        # the x86_64 and aarch64 runner logs is the strongest available proof
        # that "same language pack" is actually true rather than assumed --
        # if the hashes differ, the model data itself is the divergence, not
        # a Tesseract/leptonica code-version skew.
        checksum=$(sha256sum "$tessdata_path/${lang}.traineddata" 2>/dev/null | awk '{print $1}')
        echo "  ✓ ${lang}.traineddata ($size bytes, sha256 ${checksum:-unknown})"
      else
        echo "  ⚠ ${lang}.traineddata (missing)"
      fi
    done
    tessdata_found=1
    break
  fi
done

if [ $tessdata_found -eq 0 ]; then
  echo "::error::Tessdata directory not found in standard locations"
  exit 1
fi

echo "::endgroup::"

echo "::group::Verifying current source-built OCR stack"
bash "$(dirname "${BASH_SOURCE[0]}")/build-ocr-linux.sh"
if [[ -n "${GITHUB_PATH:-}" ]]; then echo /usr/local/bin >>"$GITHUB_PATH"; fi
echo "::endgroup::"
