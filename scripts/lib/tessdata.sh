#!/usr/bin/env bash

set -euo pipefail

file_size_bytes() {
	local path="$1"
	if [ ! -f "$path" ]; then
		echo 0
		return
	fi
	if stat -c%s "$path" >/dev/null 2>&1; then
		stat -c%s "$path"
		return
	fi
	stat -f%z "$path"
}

min_traineddata_size_bytes() {
	local lang="$1"
	case "$lang" in
	eng) echo 1000000 ;;
	osd) echo 100000 ;;
	deu) echo 1000000 ;;
	*) echo 100000 ;;
	esac
}

download_traineddata() {
	local lang="$1"
	local dest="$2"
	local url="$3"
	local tmp="${dest}.tmp"
	local min_size
	min_size="$(min_traineddata_size_bytes "$lang")"

	rm -f "$tmp"

	for attempt in 1 2 3 4 5; do
		if curl -fsSL --retry 5 --retry-delay 5 --retry-all-errors "$url" -o "$tmp"; then
			local size
			size="$(file_size_bytes "$tmp")"
			if [ "$size" -ge "$min_size" ]; then
				mv -f "$tmp" "$dest"
				return 0
			fi
			echo "Downloaded ${lang}.traineddata too small (${size} bytes < ${min_size}), retrying..." >&2
		else
			echo "Failed to download ${lang}.traineddata (attempt ${attempt}), retrying..." >&2
		fi
		rm -f "$tmp"
		sleep "$attempt"
	done

	echo "ERROR: Failed to download valid ${lang}.traineddata after retries" >&2
	return 1
}

ensure_valid_traineddata() {
	local dest_dir="$1"
	local lang="$2"
	local url="$3"
	local dest_file="${dest_dir}/${lang}.traineddata"
	local min_size
	min_size="$(min_traineddata_size_bytes "$lang")"

	local size
	size="$(file_size_bytes "$dest_file")"
	if [ "$size" -ge "$min_size" ]; then
		return 0
	fi

	if [ -f "$dest_file" ]; then
		echo "Invalid ${lang}.traineddata at ${dest_file} (${size} bytes < ${min_size}); re-downloading..." >&2
		rm -f "$dest_file"
	fi

	download_traineddata "$lang" "$dest_file" "$url"
}

ensure_tessdata() {
	local dest="$1"
	mkdir -p "$dest"
	local dest_real
	dest_real="$(cd "$dest" && pwd -P)"

	local candidates=(
		"/opt/homebrew/share/tessdata"
		"/usr/local/opt/tesseract/share/tessdata"
		"/usr/share/tesseract-ocr/5/tessdata"
	)

	if [ -n "${PROGRAMFILES:-}" ] && command -v cygpath >/dev/null 2>&1; then
		candidates+=("$(cygpath -u "$PROGRAMFILES")/Tesseract-OCR/tessdata")
	fi
	if [ -d "/c/Program Files/Tesseract-OCR/tessdata" ]; then
		candidates+=("/c/Program Files/Tesseract-OCR/tessdata")
	fi

	for dir in "${candidates[@]}"; do
		if [ -f "$dir/eng.traineddata" ]; then
			local dir_real
			dir_real="$(cd "$dir" && pwd -P)"

			if [ "$dir_real" = "$dest_real" ]; then
				break
			fi

			for lang in eng osd deu; do
				if [ -f "$dir/$lang.traineddata" ]; then
					if [ -f "$dest/$lang.traineddata" ] &&
						[ "$dir_real/$lang.traineddata" -ef "$dest/$lang.traineddata" ]; then
						continue
					fi
					cp -f "$dir/$lang.traineddata" "$dest/"
				fi
			done
			break
		fi
	done

	ensure_valid_traineddata "$dest" "eng" "https://github.com/tesseract-ocr/tessdata_fast/raw/main/eng.traineddata"
	ensure_valid_traineddata "$dest" "osd" "https://github.com/tesseract-ocr/tessdata_fast/raw/main/osd.traineddata"
}

setup_tessdata() {
	local platform="${RUNNER_OS:-$(uname -s)}"

	case "$platform" in
	Linux)
		export TESSDATA_PREFIX="/usr/share/tesseract-ocr/5/tessdata"
		;;
	macOS | Darwin)
		if [ -d "/opt/homebrew/opt/tesseract/share/tessdata" ]; then
			export TESSDATA_PREFIX="/opt/homebrew/opt/tesseract/share/tessdata"
		elif [ -d "/usr/local/opt/tesseract/share/tessdata" ]; then
			export TESSDATA_PREFIX="/usr/local/opt/tesseract/share/tessdata"
		else
			export TESSDATA_PREFIX="$HOME/Library/Application Support/tesseract-rs/tessdata"
		fi
		;;
	Windows | MINGW* | MSYS* | CYGWIN*)
		export TESSDATA_PREFIX="${APPDATA:-${USERPROFILE:-}}/tesseract-rs/tessdata"
		;;
	*)
		export TESSDATA_PREFIX="${REPO_ROOT:-$(pwd)}/target/tessdata"
		;;
	esac

	ensure_tessdata "$TESSDATA_PREFIX"

	echo "✓ TESSDATA_PREFIX set to: $TESSDATA_PREFIX"
	[ -f "$TESSDATA_PREFIX/eng.traineddata" ] && echo "✓ eng.traineddata available"
	[ -f "$TESSDATA_PREFIX/osd.traineddata" ] && echo "✓ osd.traineddata available"
}

export -f ensure_tessdata
export -f setup_tessdata
