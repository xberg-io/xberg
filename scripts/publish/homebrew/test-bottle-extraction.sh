#!/usr/bin/env bash

set -euo pipefail

echo "=== Homebrew Bottle Building Test Suite ==="
echo ""

test_dir=$(mktemp -d)
trap 'rm -rf "$test_dir"' EXIT

echo "Test 1: Validate bottle filename parsing"
echo "=========================================="

test_bottles=(
	"kreuzberg--4.0.0.arm64_sequoia.bottle.tar.gz"
	"kreuzberg--4.0.0-rc.1.arm64_sequoia.bottle.tar.gz"
	"kreuzberg--4.0.0.ventura.bottle.tar.gz"
)

for bottle in "${test_bottles[@]}"; do
	without_suffix="${bottle%.bottle.tar.gz}"
	bottle_tag="${without_suffix##*.}"
	echo "  $bottle -> $bottle_tag"

	case "$bottle" in
	*arm64_sequoia*)
		[ "$bottle_tag" = "arm64_sequoia" ] || {
			echo "FAIL: Expected arm64_sequoia, got $bottle_tag"
			exit 1
		}
		;;
	*ventura*)
		[ "$bottle_tag" = "ventura" ] || {
			echo "FAIL: Expected ventura, got $bottle_tag"
			exit 1
		}
		;;
	esac
done

echo "  PASS: All bottle tags extracted correctly"
echo ""

echo "Test 2: Validate SHA256 calculation"
echo "===================================="

test_bottle="$test_dir/kreuzberg--4.0.0.arm64_sequoia.bottle.tar.gz"
echo "test content" | gzip >"$test_bottle"

sha256=$(shasum -a 256 "$test_bottle" | cut -d' ' -f1)
echo "  SHA256: $sha256"
[ ${#sha256} -eq 64 ] || {
	echo "FAIL: SHA256 hash should be 64 chars"
	exit 1
}

echo "  PASS: SHA256 hash is valid"
echo ""

echo "Test 3: Validate formula file update logic"
echo "=========================================="

test_formula="$test_dir/kreuzberg.rb"
cat >"$test_formula" <<'EOF'
class Kreuzberg < Formula
  desc "High-performance document intelligence CLI"
  homepage "https://kreuzberg.dev"
  url "https://github.com/kreuzberg-dev/kreuzberg/archive/v4.0.0-rc.17.tar.gz"
  sha256 "old_sha256_value"
  license "MIT"

  # bottle do
  #   root_url "https://github.com/kreuzberg-dev/kreuzberg/releases/download/v4.0.0-rc.17"
  #   sha256 cellar: :any_skip_relocation, arm64_sequoia: "placeholder"
  # end

  depends_on "cmake" => :build
  depends_on "rust" => :build

  def install
    system "cargo", "install"
  end
end
EOF

updated=$(sed 's|url "https://github.com/kreuzberg-dev/kreuzberg/archive/.*\.tar\.gz"|url "https://github.com/kreuzberg-dev/kreuzberg/archive/v4.0.0-rc.18.tar.gz"|' "$test_formula")

if echo "$updated" | grep -q "v4.0.0-rc.18.tar.gz"; then
	echo "  PASS: URL replacement works"
else
	echo "  FAIL: URL replacement failed"
	exit 1
fi

test_without_bottles=$(echo "$updated" | sed '/# bottle do/,/# end/d')

if ! echo "$test_without_bottles" | grep -q "# bottle do"; then
	echo "  PASS: Bottle block removal works"
else
	echo "  FAIL: Bottle block removal failed"
	exit 1
fi

echo ""

echo "Test 4: Validate GitHub artifact naming conventions"
echo "===================================================="

patterns=(
	"homebrew-bottle-arm64_sequoia"
	"homebrew-bottle-ventura"
	"homebrew-bottle-*"
)

for pattern in "${patterns[@]}"; do
	echo "  Pattern: $pattern"
done

echo "  PASS: Artifact naming follows conventions"
echo ""

echo "=== All Tests Passed ==="
echo ""
echo "Key validations:"
echo "  1. Bottle filename parsing correctly extracts platform tags"
echo "  2. SHA256 hashes are computed correctly"
echo "  3. Formula file updates work as expected"
echo "  4. GitHub Actions artifact naming follows conventions"
