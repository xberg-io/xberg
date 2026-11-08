#!/usr/bin/env bash
# Load the macOS PHP extension from a release archive with the job's PHP and decode the upstream
# libheif example HEIC and AVIF files through it, so a build whose static HEIF stack is missing
# or broken fails before upload (xberg-io/xberg#2547). `php -n` ignores every ini, so no
# Homebrew-configured module can stand in for the archived binary.
set -euo pipefail

archive="${1:?usage: smoke-php-macos-heif.sh <php-archive.tgz>}"

LIBHEIF_VERSION=1.23.7
LIBHEIF_SHA256=f7483e818503ea6224c8863e4a56ac02c7089021a98472c811c12aa19faf2475
LIBHEIF_URL="https://github.com/strukturag/libheif/releases/download/v$LIBHEIF_VERSION/libheif-$LIBHEIF_VERSION.tar.gz"
FIXTURES=(
  "example.heic 718114 7f8b363e4936c0666a25f64f3a92fda10bd8e5453be4592530b65a55dd98f3f2"
  "example.avif 113604 54a0dc31d02b6f5d9d4b66027d4787861b7af15ffd8fab8eab963d10c5411469"
)

log() { echo "smoke-php-macos-heif: $*" >&2; }
die() {
  log "$*"
  exit 1
}

[ -f "$archive" ] || die "archive does not exist: $archive"
command -v php >/dev/null 2>&1 || die "php is required"

workdir="$(mktemp -d)"
[ -n "$workdir" ] && [ -d "$workdir" ] || exit 90
trap 'rm -rf "$workdir"' EXIT

mkdir "$workdir/ext" "$workdir/src"
tar -xzf "$archive" -C "$workdir/ext"
extension="$workdir/ext/xberg.so"
[ -f "$extension" ] || die "xberg.so is not at the archive root"

php -n -d "extension=$extension" -r 'exit(extension_loaded("xberg") ? 0 : 1);' ||
  die "PHP did not load the archived xberg.so"

curl -fsSL --retry 3 -o "$workdir/libheif.tar.gz" "$LIBHEIF_URL"
printf '%s  %s\n' "$LIBHEIF_SHA256" "$workdir/libheif.tar.gz" | shasum -a 256 -c - >/dev/null ||
  die "libheif source archive checksum mismatch"
tar -xzf "$workdir/libheif.tar.gz" -C "$workdir/src" \
  "libheif-$LIBHEIF_VERSION/examples/example.heic" "libheif-$LIBHEIF_VERSION/examples/example.avif"
for row in "${FIXTURES[@]}"; do
  read -r name size digest <<<"$row"
  fixture="$workdir/src/libheif-$LIBHEIF_VERSION/examples/$name"
  [ "$(wc -c <"$fixture" | tr -d ' ')" = "$size" ] || die "unexpected size for $name"
  [ "$(shasum -a 256 <"$fixture" | awk '{print $1}')" = "$digest" ] || die "unexpected checksum for $name"
done
# A truncated copy must fail inside libheif, proving the decode assertions can fail.
head -c 4000 "$workdir/src/libheif-$LIBHEIF_VERSION/examples/example.heic" >"$workdir/truncated.heic"

cat >"$workdir/probe.php" <<'PHP'
<?php
[, $valid, $truncated] = [$argv[0], array_slice($argv, 1, -1), end($argv)];
foreach ($valid as $path) {
    $output = \Xberg\XbergApi::extract(\Xberg\ExtractInput::fromUri($path), \Xberg\ExtractionConfig::default());
    $results = $output->getResults();
    if (count($results) !== 1 || count($output->getErrors()) !== 0 || $results[0]->getMimeType() !== 'image/png') {
        fwrite(STDERR, basename($path) . " was not decoded to an image\n");
        exit(1);
    }
    echo basename($path), " decoded\n";
}
try {
    \Xberg\XbergApi::extract(\Xberg\ExtractInput::fromUri($truncated), \Xberg\ExtractionConfig::default());
    fwrite(STDERR, "a truncated HEIC unexpectedly decoded\n");
    exit(1);
} catch (\Xberg\ParsingError $error) {
    if (!str_contains($error->getMessage(), 'Failed to decode HEIF image')) {
        fwrite(STDERR, 'unexpected failure: ' . $error->getMessage() . "\n");
        exit(1);
    }
    echo "truncated HEIC rejected by libheif\n";
}
PHP
examples="$workdir/src/libheif-$LIBHEIF_VERSION/examples"
php -n -d "extension=$extension" "$workdir/probe.php" "$examples/example.heic" "$examples/example.avif" "$workdir/truncated.heic" ||
  die "the archived extension did not decode HEIC and AVIF"
log "loaded xberg.so with $(php -n -r 'echo PHP_VERSION;') and decoded HEIC and AVIF"
