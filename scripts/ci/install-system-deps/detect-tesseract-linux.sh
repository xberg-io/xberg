#!/usr/bin/env bash
set -euo pipefail

version="$(
	apt-cache policy tesseract-ocr 2>/dev/null |
		grep 'Candidate:' |
		grep -Eo '[0-9]+\.[0-9]+' |
		head -1 ||
		true
)"

if [[ -z "${version}" ]]; then
	version="unknown"
fi

echo "version=${version}" >>"${GITHUB_OUTPUT}"
echo "::notice title=Tesseract Version::Detected version: ${version}"
