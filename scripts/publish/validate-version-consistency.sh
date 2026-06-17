#!/usr/bin/env bash
set -euo pipefail

expected="${1:-${EXPECTED_VERSION:-}}"
if [ -z "$expected" ]; then
  echo "Usage: $0 <expected-version> (or set EXPECTED_VERSION)" >&2
  exit 2
fi

# Ecosystem-specific normalized forms of the canonical version. The canonical
# form is `4.10.0-rc.1` (semver). Some manifest formats normalize prereleases:
#   - Python (PEP 440):  `4.10.0rc1`           — drop hyphen, drop dot before N
#   - RubyGems:          `4.10.0.pre.rc.1`     — substitute dash → `.pre.`
expected_python="$(echo "$expected" | sed -E 's/-rc\.?([0-9]+)/rc\1/; s/-alpha\.?([0-9]+)/a\1/; s/-beta\.?([0-9]+)/b\1/')"
expected_ruby="$(echo "$expected" | sed -E 's/-rc\.?([0-9]+)/.pre.rc.\1/; s/-alpha\.?([0-9]+)/.pre.alpha.\1/; s/-beta\.?([0-9]+)/.pre.beta.\1/')"

errors=0

echo "Expected version: $expected (python: $expected_python, ruby: $expected_ruby)"
echo "----------------------------------------"

cargo_version="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2 || true)"
echo "Cargo.toml: $cargo_version"
[ "$cargo_version" = "$expected" ] || {
  echo "❌ Cargo.toml mismatch"
  errors=$((errors + 1))
}

# Root package.json is the pnpm workspace marker (private, not published). Not
# part of any registry release; skipped intentionally.

wasm_version="$(jq -r '.version' crates/kreuzberg-wasm/package.json)"
echo "crates/kreuzberg-wasm/package.json: $wasm_version"
[ "$wasm_version" = "$expected" ] || {
  echo "❌ WASM package.json mismatch"
  errors=$((errors + 1))
}

node_version="$(jq -r '.version' crates/kreuzberg-node/package.json)"
echo "crates/kreuzberg-node/package.json: $node_version"
[ "$node_version" = "$expected" ] || {
  echo "❌ Node package.json mismatch"
  errors=$((errors + 1))
}

python_version="$(grep '^version' packages/python/pyproject.toml | head -1 | cut -d'"' -f2 || true)"
echo "packages/python/pyproject.toml: $python_version"
[ "$python_version" = "$expected_python" ] || {
  echo "❌ Python pyproject.toml mismatch (expected $expected_python, got $python_version)"
  errors=$((errors + 1))
}

ruby_version_file="$(find packages/ruby \( -path 'packages/ruby/lib/*/version.rb' -o -path 'packages/ruby/ext/*/src/*/version.rb' -o -path 'packages/ruby/ext/*/native/src/*/version.rb' \) -type f 2>/dev/null | head -1)"
if [ -n "$ruby_version_file" ]; then
  ruby_version="$(grep "VERSION =" "$ruby_version_file" | sed -E 's/.*VERSION[[:space:]]*=[[:space:]]*["'\''"]([^"'\'']+)["'\''"].*/\1/')"
  echo "$ruby_version_file: $ruby_version"
  [ "$ruby_version" = "$expected_ruby" ] || {
    echo "❌ Ruby version.rb mismatch (expected $expected_ruby, got $ruby_version)"
    errors=$((errors + 1))
  }
else
  echo "⚠ no Ruby version.rb found under packages/ruby — skipping"
fi

r_version="$(grep '^Version:' packages/r/DESCRIPTION | sed 's/Version: //')"
echo "packages/r/DESCRIPTION: $r_version"
[ "$r_version" = "$expected" ] || {
  echo "❌ R DESCRIPTION version mismatch"
  errors=$((errors + 1))
}

java_version="$(
  python3 - <<'PY'
import re
import xml.etree.ElementTree as ET
from pathlib import Path

text = Path("packages/java/pom.xml").read_text(encoding="utf-8")
text = re.sub(r'xmlns="[^"]+"', '', text, count=1)
root = ET.fromstring(text)
version = root.findtext("version") or ""
print(version.strip())
PY
)"
echo "packages/java/pom.xml: $java_version"
[ "$java_version" = "$expected" ] || {
  echo "❌ Java pom.xml mismatch"
  errors=$((errors + 1))
}

csharp_version="$(
  python3 - <<'PY'
import re
import xml.etree.ElementTree as ET
from pathlib import Path

text = Path("packages/csharp/Kreuzberg/Kreuzberg.csproj").read_text(encoding="utf-8")
text = re.sub(r'xmlns="[^"]+"', '', text, count=1)
root = ET.fromstring(text)
version = ""
for elem in root.iter():
    if elem.tag == "Version" and (elem.text or "").strip():
        version = elem.text.strip()
        break
print(version)
PY
)"
echo "packages/csharp/Kreuzberg/Kreuzberg.csproj: $csharp_version"
[ "$csharp_version" = "$expected" ] || {
  echo "❌ C# csproj mismatch"
  errors=$((errors + 1))
}

if [ -f "packages/go/v4/doc.go" ]; then
  go_version="$(
    python3 - <<'PY'
import re
from pathlib import Path

text = Path("packages/go/v4/doc.go").read_text(encoding="utf-8")
m = re.search(r"This binding targets Kreuzberg\s+([^\s]+)", text)
print(m.group(1) if m else "")
PY
  )"
  echo "packages/go/v4/doc.go: $go_version"
  [ "$go_version" = "$expected" ] || {
    echo "❌ Go doc.go mismatch"
    errors=$((errors + 1))
  }
else
  echo "⚠ packages/go/v4/doc.go not present — skipping (Go uses git tags for versioning)"
fi

# PHP: composer.json typically has no `version` field — Composer relies on git
# tags. Only check if a value is actually declared in the manifest.
php_version="$(jq -r '.version // empty' packages/php/composer.json)"
if [ -n "$php_version" ]; then
  echo "packages/php/composer.json: $php_version"
  [ "$php_version" = "$expected" ] || {
    echo "❌ PHP composer.json mismatch"
    errors=$((errors + 1))
  }
else
  echo "⚠ packages/php/composer.json has no version field — skipping (Composer uses git tags)"
fi

elixir_version="$(grep -E '^\s*(@version|version:)' packages/elixir/mix.exs | head -1 | cut -d'"' -f2 || true)"
echo "packages/elixir/mix.exs: $elixir_version"
[ "$elixir_version" = "$expected" ] || {
  echo "❌ Elixir mix.exs mismatch"
  errors=$((errors + 1))
}

echo "----------------------------------------"
if [ "$errors" -gt 0 ]; then
  echo "❌ $errors version mismatches found"
  exit 1
fi

echo "✅ All 12 version sources consistent: $expected"
