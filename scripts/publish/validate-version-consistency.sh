#!/usr/bin/env bash
set -euo pipefail

expected="${1:-${EXPECTED_VERSION:-}}"
if [ -z "$expected" ]; then
	echo "Usage: $0 <expected-version> (or set EXPECTED_VERSION)" >&2
	exit 2
fi

errors=0

echo "Expected version: $expected"
echo "----------------------------------------"

cargo_version="$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)"
echo "Cargo.toml: $cargo_version"
[ "$cargo_version" = "$expected" ] || {
	echo "❌ Cargo.toml mismatch"
	errors=$((errors + 1))
}

root_version="$(jq -r '.version' package.json)"
echo "package.json (root): $root_version"
[ "$root_version" = "$expected" ] || {
	echo "❌ package.json (root) mismatch"
	errors=$((errors + 1))
}

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

python_version="$(grep '^version' packages/python/pyproject.toml | head -1 | cut -d'"' -f2)"
echo "packages/python/pyproject.toml: $python_version"
[ "$python_version" = "$expected" ] || {
	echo "❌ Python pyproject.toml mismatch"
	errors=$((errors + 1))
}

ruby_version="$(grep "VERSION =" packages/ruby/lib/kreuzberg/version.rb | cut -d"'" -f2)"
echo "packages/ruby/lib/kreuzberg/version.rb: $ruby_version"
[ "$ruby_version" = "$expected" ] || {
	echo "❌ Ruby version.rb mismatch"
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

php_version="$(jq -r '.version' packages/php/composer.json)"
echo "packages/php/composer.json: $php_version"
[ "$php_version" = "$expected" ] || {
	echo "❌ PHP composer.json mismatch"
	errors=$((errors + 1))
}

echo "----------------------------------------"
if [ "$errors" -gt 0 ]; then
	echo "❌ $errors version mismatches found"
	exit 1
fi

echo "✅ All 10 version sources consistent: $expected"
