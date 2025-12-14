#!/usr/bin/env bash
set -euo pipefail

version=""

json="$(brew info --json=v2 tesseract 2>/dev/null || true)"
if [[ -n "${json}" ]]; then
	version="$(
		python3 -c 'import json, re, sys; data = json.loads(sys.argv[1]); stable = (((data.get("formulae") or [{}])[0].get("versions") or {}).get("stable") or ""); m = re.match(r"^(\d+\.\d+)", stable); print(m.group(1) if m else "")' "${json}" || true
	)"
fi

if [[ -z "${version}" ]]; then
	first_line="$(brew info tesseract 2>/dev/null | head -1 || true)"
	if [[ "${first_line}" =~ ([0-9]+\.[0-9]+) ]]; then
		version="${BASH_REMATCH[1]}"
	fi
fi

if [[ -z "${version}" ]]; then
	version="unknown"
fi

echo "version=${version}" >>"${GITHUB_OUTPUT}"
echo "::notice title=Tesseract Version::Detected version: ${version}"
