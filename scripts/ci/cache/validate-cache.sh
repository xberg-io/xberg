#!/usr/bin/env bash
# Validate cached artifacts to ensure they're not corrupted
#
# Usage:
#   validate-cache.sh <artifact-type> <path...>
#
# Example:
#   validate-cache.sh ffi target/release/libkreuzberg_ffi.so
#   validate-cache.sh python target/wheels/*.whl

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

error() {
	echo -e "${RED}Error: $*${NC}" >&2
	exit 1
}

info() {
	echo -e "${GREEN}$*${NC}" >&2
}

warn() {
	echo -e "${YELLOW}$*${NC}" >&2
}

# Cross-platform file size function
# Gets file size in bytes, works on Linux, macOS, and Windows
get_file_size() {
	local file="$1"

	# Try macOS/BSD stat first
	if stat -f%z "$file" 2>/dev/null; then
		return 0
	fi

	# Try Linux stat
	if stat -c%s "$file" 2>/dev/null; then
		return 0
	fi

	# Try Windows PowerShell
	if command -v powershell &>/dev/null; then
		powershell -NoProfile -Command "if (Test-Path '$file') { (Get-Item '$file').Length }"
		return 0
	fi

	# Fallback: use wc -c if available
	if command -v wc &>/dev/null; then
		wc -c <"$file" 2>/dev/null
		return 0
	fi

	# If all else fails, return 0 (will be treated as unknown)
	echo "0"
	return 0
}

# Check WASM magic bytes - works cross-platform
check_wasm_magic() {
	local file="$1"

	# Try xxd first (Linux/macOS)
	if command -v xxd &>/dev/null && xxd -l 4 -p "$file" 2>/dev/null | grep -q "0061736d"; then
		return 0
	fi

	# Try od command (available on all Unix-like systems including macOS and Windows git bash)
	if command -v od &>/dev/null && od -A n -t x1 -N 4 "$file" 2>/dev/null | grep -q "00 61 73 6d"; then
		return 0
	fi

	# Try hexdump (might be available)
	if command -v hexdump &>/dev/null && hexdump -C -n 4 "$file" 2>/dev/null | grep -q "00 61 73 6d"; then
		return 0
	fi

	# Fallback to file command if available
	if command -v file &>/dev/null && file "$file" 2>/dev/null | grep -q "WebAssembly"; then
		return 0
	fi

	return 1
}

if [[ $# -lt 2 ]]; then
	error "Usage: $0 <artifact-type> <path...>"
fi

ARTIFACT_TYPE="$1"
shift

info "Validating $ARTIFACT_TYPE artifacts..."

# Track validation results
VALID_COUNT=0
INVALID_COUNT=0
MISSING_COUNT=0

for path in "$@"; do
	# Check if path is a directory
	if [[ -d "$path" ]]; then
		# For WASM artifacts, look for .wasm files recursively in the directory
		if [[ "$ARTIFACT_TYPE" == "wasm" ]]; then
			wasm_files=$(find "$path" -type f -name "*.wasm" 2>/dev/null || true)
			if [[ -z "$wasm_files" ]]; then
				warn "No WASM files found in directory: $path"
				((MISSING_COUNT++))
			else
				while IFS= read -r artifact; do
					# Skip empty lines
					[[ -z "$artifact" ]] && continue

					# File exists, check size
					SIZE=$(du -sh "$artifact" 2>/dev/null | cut -f1 || echo "unknown")
					FILE_SIZE=$(get_file_size "$artifact")

					if [[ "$FILE_SIZE" -eq 0 ]]; then
						warn "Empty file: $artifact"
						((INVALID_COUNT++))
						continue
					fi

					# Check for WASM magic bytes (\0asm)
					if check_wasm_magic "$artifact"; then
						info "✓ Valid WASM module: $artifact ($SIZE)"
						((VALID_COUNT++))
					else
						warn "Invalid WASM format: $artifact"
						((INVALID_COUNT++))
					fi
				done <<<"$wasm_files"
			fi
		# For Node artifacts, look for .node files recursively in the directory
		elif [[ "$ARTIFACT_TYPE" == "node" ]]; then
			node_files=$(find "$path" -type f -name "*.node" 2>/dev/null || true)
			if [[ -z "$node_files" ]]; then
				warn "No Node.js modules found in directory: $path"
				((MISSING_COUNT++))
			else
				while IFS= read -r artifact; do
					# Skip empty lines
					[[ -z "$artifact" ]] && continue

					# File exists, check size
					SIZE=$(du -sh "$artifact" 2>/dev/null | cut -f1 || echo "unknown")
					FILE_SIZE=$(get_file_size "$artifact")

					if [[ "$FILE_SIZE" -eq 0 ]]; then
						warn "Empty file: $artifact"
						((INVALID_COUNT++))
						continue
					fi

					# Check for valid Node.js native module
					if file "$artifact" 2>/dev/null | grep -qE "(shared object|shared library|Mach-O|DLL)"; then
						info "✓ Valid Node.js module: $artifact ($SIZE)"
						((VALID_COUNT++))
					else
						warn "Invalid .node format: $artifact"
						((INVALID_COUNT++))
					fi
				done <<<"$node_files"
			fi
		# For FFI artifacts, look for library files recursively in the directory
		elif [[ "$ARTIFACT_TYPE" == "ffi" ]]; then
			ffi_files=$(find "$path" -type f \( -name "*.so" -o -name "*.dylib" -o -name "*.dll" -o -name "*.a" -o -name "*.lib" \) 2>/dev/null || true)
			if [[ -z "$ffi_files" ]]; then
				warn "No FFI library files found in directory: $path"
				((MISSING_COUNT++))
			else
				while IFS= read -r artifact; do
					# Skip empty lines
					[[ -z "$artifact" ]] && continue

					# File exists, check size
					SIZE=$(du -sh "$artifact" 2>/dev/null | cut -f1 || echo "unknown")
					FILE_SIZE=$(get_file_size "$artifact")

					if [[ "$FILE_SIZE" -eq 0 ]]; then
						warn "Empty file: $artifact"
						((INVALID_COUNT++))
						continue
					fi

					# Check for valid library file format
					if file "$artifact" 2>/dev/null | grep -qE "(shared object|shared library|Mach-O|DLL|current ar archive)"; then
						info "✓ Valid FFI library: $artifact ($SIZE)"
						((VALID_COUNT++))
					else
						warn "Invalid FFI library format: $artifact"
						((INVALID_COUNT++))
					fi
				done <<<"$ffi_files"
			fi
		else
			# For other types, just check that the directory exists
			info "✓ Directory exists: $path"
			((VALID_COUNT++))
		fi
		continue
	fi

	# Expand glob patterns
	for artifact in $path; do
		if [[ ! -e "$artifact" ]]; then
			warn "Missing: $artifact"
			((MISSING_COUNT++))
			continue
		fi

		# File exists, check size
		SIZE=$(du -sh "$artifact" 2>/dev/null | cut -f1 || echo "unknown")
		FILE_SIZE=$(get_file_size "$artifact")

		if [[ "$FILE_SIZE" -eq 0 ]]; then
			warn "Empty file: $artifact"
			((INVALID_COUNT++))
			continue
		fi

		# Artifact type-specific validation
		case "$ARTIFACT_TYPE" in
		ffi)
			# Validate shared library
			if [[ "$artifact" == *.so || "$artifact" == *.dylib || "$artifact" == *.dll ]]; then
				# Check if it's a valid binary (has ELF/Mach-O/PE magic bytes)
				if file "$artifact" 2>/dev/null | grep -qE "(shared object|shared library|Mach-O|PE32|DLL)"; then
					info "✓ Valid FFI library: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid binary format: $artifact"
					((INVALID_COUNT++))
				fi
			elif [[ "$artifact" == *.a || "$artifact" == *.lib ]]; then
				# Static library
				if file "$artifact" 2>/dev/null | grep -qE "(archive|library)"; then
					info "✓ Valid static library: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid archive format: $artifact"
					((INVALID_COUNT++))
				fi
			else
				# Other files (e.g., .pc files)
				info "✓ File exists: $artifact ($SIZE)"
				((VALID_COUNT++))
			fi
			;;

		python)
			# Validate Python wheels
			if [[ "$artifact" == *.whl ]]; then
				# Check if it's a valid ZIP archive
				if file "$artifact" 2>/dev/null | grep -q "Zip archive"; then
					info "✓ Valid Python wheel: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid wheel format: $artifact"
					((INVALID_COUNT++))
				fi
			elif [[ "$artifact" == *.tar.gz ]]; then
				# Check if it's a valid tarball
				if file "$artifact" 2>/dev/null | grep -q "gzip compressed"; then
					info "✓ Valid Python sdist: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid sdist format: $artifact"
					((INVALID_COUNT++))
				fi
			else
				info "✓ File exists: $artifact ($SIZE)"
				((VALID_COUNT++))
			fi
			;;

		ruby)
			# Validate Ruby gems
			if [[ "$artifact" == *.gem ]]; then
				if file "$artifact" 2>/dev/null | grep -q "tar archive"; then
					info "✓ Valid Ruby gem: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid gem format: $artifact"
					((INVALID_COUNT++))
				fi
			elif [[ "$artifact" == *.bundle || "$artifact" == *.so ]]; then
				if file "$artifact" 2>/dev/null | grep -qE "(shared object|shared library|Mach-O|bundle)"; then
					info "✓ Valid Ruby extension: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid extension format: $artifact"
					((INVALID_COUNT++))
				fi
			else
				info "✓ File exists: $artifact ($SIZE)"
				((VALID_COUNT++))
			fi
			;;

		node)
			# Validate Node.js native modules
			if [[ "$artifact" == *.node ]]; then
				if file "$artifact" 2>/dev/null | grep -qE "(shared object|shared library|Mach-O|DLL)"; then
					info "✓ Valid Node.js module: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid .node format: $artifact"
					((INVALID_COUNT++))
				fi
			elif [[ "$artifact" == *.tgz || "$artifact" == *.tar.gz ]]; then
				if file "$artifact" 2>/dev/null | grep -q "gzip compressed"; then
					info "✓ Valid npm package: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid package format: $artifact"
					((INVALID_COUNT++))
				fi
			else
				info "✓ File exists: $artifact ($SIZE)"
				((VALID_COUNT++))
			fi
			;;

		wasm)
			# Validate WebAssembly modules
			if [[ "$artifact" == *.wasm ]]; then
				# Check for WASM magic bytes (\0asm) using check_wasm_magic function
				if check_wasm_magic "$artifact"; then
					info "✓ Valid WASM module: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid WASM format: $artifact"
					((INVALID_COUNT++))
				fi
			else
				info "✓ File exists: $artifact ($SIZE)"
				((VALID_COUNT++))
			fi
			;;

		java)
			# Validate Java JARs
			if [[ "$artifact" == *.jar ]]; then
				if file "$artifact" 2>/dev/null | grep -q "Zip archive"; then
					info "✓ Valid JAR file: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid JAR format: $artifact"
					((INVALID_COUNT++))
				fi
			else
				info "✓ File exists: $artifact ($SIZE)"
				((VALID_COUNT++))
			fi
			;;

		csharp)
			# Validate .NET packages
			if [[ "$artifact" == *.nupkg ]]; then
				if file "$artifact" 2>/dev/null | grep -q "Zip archive"; then
					info "✓ Valid NuGet package: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid NuGet format: $artifact"
					((INVALID_COUNT++))
				fi
			elif [[ "$artifact" == *.dll || "$artifact" == *.so || "$artifact" == *.dylib ]]; then
				if file "$artifact" 2>/dev/null | grep -qE "(shared object|shared library|Mach-O|DLL|PE32)"; then
					info "✓ Valid native library: $artifact ($SIZE)"
					((VALID_COUNT++))
				else
					warn "Invalid library format: $artifact"
					((INVALID_COUNT++))
				fi
			else
				info "✓ File exists: $artifact ($SIZE)"
				((VALID_COUNT++))
			fi
			;;

		*)
			# Generic validation - just check existence and non-zero size
			info "✓ File exists: $artifact ($SIZE)"
			((VALID_COUNT++))
			;;
		esac
	done
done

# Summary
echo ""
echo "=== Validation Summary ==="
echo "Valid:   $VALID_COUNT"
echo "Invalid: $INVALID_COUNT"
echo "Missing: $MISSING_COUNT"

# Only fail if there are invalid artifacts (corrupted files)
# Missing artifacts are OK as they may be platform-specific
if [[ $INVALID_COUNT -gt 0 ]]; then
	error "Validation failed: $INVALID_COUNT invalid artifacts found"
fi

# Ensure at least one valid artifact was found
if [[ $VALID_COUNT -eq 0 ]]; then
	error "Validation failed: no valid artifacts found"
fi

info "All artifacts validated successfully!"
exit 0
