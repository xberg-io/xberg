#!/usr/bin/env bash

set -euo pipefail

event="${GITHUB_EVENT_NAME:-${1:-}}"
tag=""
dry_run_input="false"
ref_input=""
targets_input=""

if [[ "$event" == "workflow_dispatch" ]]; then
	tag="${INPUT_TAG:-}"
	dry_run_input="${INPUT_DRY_RUN:-false}"
	ref_input="${INPUT_REF:-}"
	targets_input="${INPUT_TARGETS:-}"
elif [[ "$event" == "release" ]]; then
	tag="${EVENT_RELEASE_TAG:-}"
	dry_run_input="false"
	ref_input="refs/tags/${tag}"
	targets_input=""
elif [[ "$event" == "repository_dispatch" ]]; then
	tag="${EVENT_DISPATCH_TAG:-}"
	dry_run_input="${EVENT_DISPATCH_DRY_RUN:-false}"
	ref_input="${EVENT_DISPATCH_REF:-}"
	targets_input="${EVENT_DISPATCH_TARGETS:-}"
else
	tag="${GITHUB_REF_NAME:-}"
	dry_run_input="false"
	ref_input=""
	targets_input=""
	if [[ "$tag" == *-pre* || "$tag" == *-rc* ]]; then
		dry_run_input="true"
	fi
fi

if [[ -z "$tag" ]]; then
	echo "Release tag could not be determined" >&2
	exit 1
fi

if [[ "$tag" != v* ]]; then
	echo "Tag must start with 'v' (e.g., v4.0.0-rc.1)" >&2
	exit 1
fi

version="${tag#v}"

if [[ -n "$ref_input" ]]; then
	if [[ "$ref_input" == "$tag" ]]; then
		ref="refs/tags/${tag}"
	elif [[ "$ref_input" =~ ^v[0-9] ]]; then
		ref="refs/tags/${ref_input}"
	else
		ref="$ref_input"
	fi
else
	ref="refs/tags/${tag}"
fi

if [[ "$ref" =~ ^[0-9a-f]{40}$ ]]; then
	checkout_ref="refs/heads/main"
	target_sha="$ref"
elif [[ "$ref" =~ ^refs/ ]]; then
	checkout_ref="$ref"
	target_sha=""
else
	checkout_ref="refs/heads/${ref}"
	target_sha=""
fi

if [[ "$ref" =~ ^[0-9a-f]{40}$ ]]; then
	matrix_ref="main"
elif [[ "$ref" =~ ^refs/heads/(.+)$ ]]; then
	matrix_ref="${BASH_REMATCH[1]}"
elif [[ "$ref" =~ ^refs/tags/(.+)$ ]]; then
	matrix_ref="${BASH_REMATCH[1]}"
else
	matrix_ref="$ref"
fi

if [[ "$ref" =~ ^refs/tags/ ]]; then
	is_tag="true"
else
	is_tag="false"
fi

normalize_target_list() {
	local raw="$1"
	raw="${raw:-all}"
	if [[ -z "$raw" ]]; then
		echo "all"
	else
		echo "$raw"
	fi
}

targets_value=$(normalize_target_list "$targets_input")

release_python=false
release_node=false
release_ruby=false
release_cli=false
release_crates=false
release_docker=false
release_homebrew=false
release_java=false
release_csharp=false
release_go=false
release_wasm=false
release_php=false

set_all_targets() {
	release_python=true
	release_node=true
	release_ruby=true
	release_cli=true
	release_crates=true
	release_docker=true
	release_homebrew=true
	release_java=true
	release_csharp=true
	release_go=true
	release_wasm=true
	release_php=true
}

mapfile -t requested_targets < <(echo "$targets_value" | tr ',' '\n')

processed_any=false
for raw_target in "${requested_targets[@]}"; do
	trimmed=$(echo "$raw_target" | tr '[:upper:]' '[:lower:]' | xargs)
	if [[ -z "$trimmed" ]]; then
		continue
	fi
	processed_any=true
	case "$trimmed" in
	all | '*' | 'default')
		set_all_targets
		break
		;;
	python)
		release_python=true
		;;
	node)
		release_node=true
		;;
	ruby)
		release_ruby=true
		;;
	cli)
		release_cli=true
		;;
	crates)
		release_crates=true
		;;
	docker)
		release_docker=true
		;;
	homebrew)
		release_homebrew=true
		;;
	java)
		release_java=true
		;;
	csharp | dotnet | cs | nuget)
		release_csharp=true
		;;
	go | golang)
		release_go=true
		;;
	wasm | webassembly)
		release_wasm=true
		;;
	php)
		release_php=true
		;;
	none)
		release_python=false
		release_node=false
		release_ruby=false
		release_cli=false
		release_crates=false
		release_docker=false
		release_homebrew=false
		release_java=false
		release_csharp=false
		release_go=false
		release_wasm=false
		release_php=false
		;;
	*)
		echo "Unknown release target '$trimmed'. Allowed: all, python, node, ruby, cli, crates, docker, homebrew, java, csharp, go, wasm, php." >&2
		exit 1
		;;
	esac
done

if [[ "$release_homebrew" == "true" ]]; then
	release_cli=true
fi

if [[ "$processed_any" == "false" ]]; then
	set_all_targets
fi

enabled_targets=()
if [[ "$release_python" == "true" ]]; then enabled_targets+=("python"); fi
if [[ "$release_node" == "true" ]]; then enabled_targets+=("node"); fi
if [[ "$release_ruby" == "true" ]]; then enabled_targets+=("ruby"); fi
if [[ "$release_cli" == "true" ]]; then enabled_targets+=("cli"); fi
if [[ "$release_crates" == "true" ]]; then enabled_targets+=("crates"); fi
if [[ "$release_docker" == "true" ]]; then enabled_targets+=("docker"); fi
if [[ "$release_homebrew" == "true" ]]; then enabled_targets+=("homebrew"); fi
if [[ "$release_java" == "true" ]]; then enabled_targets+=("java"); fi
if [[ "$release_csharp" == "true" ]]; then enabled_targets+=("csharp"); fi
if [[ "$release_go" == "true" ]]; then enabled_targets+=("go"); fi
if [[ "$release_wasm" == "true" ]]; then enabled_targets+=("wasm"); fi
if [[ "$release_php" == "true" ]]; then enabled_targets+=("php"); fi

if [[ ${#enabled_targets[@]} -eq 12 ]]; then
	release_targets_summary="all"
elif [[ ${#enabled_targets[@]} -eq 0 ]]; then
	release_targets_summary="none"
else
	release_targets_summary=$(
		IFS=','
		echo "${enabled_targets[*]}"
	)
fi

release_any="false"
if [[ ${#enabled_targets[@]} -gt 0 ]]; then
	release_any="true"
fi

determine_npm_tag() {
	local ver="$1"
	if [[ "$ver" == *-rc* ]] || [[ "$ver" == *-alpha* ]] || [[ "$ver" == *-beta* ]] || [[ "$ver" == *-pre* ]]; then
		echo "next"
	else
		echo "latest"
	fi
}

npm_tag=$(determine_npm_tag "$version")

cat <<JSON
{
  "tag": "$tag",
  "version": "$version",
  "npm_tag": "$npm_tag",
  "ref": "$ref",
  "checkout_ref": "$checkout_ref",
  "target_sha": "$target_sha",
  "matrix_ref": "$matrix_ref",
  "dry_run": ${dry_run_input:-false},
  "is_tag": $is_tag,
  "release_targets": "$release_targets_summary",
  "release_any": $release_any,
  "release_python": $release_python,
  "release_node": $release_node,
  "release_ruby": $release_ruby,
  "release_cli": $release_cli,
  "release_crates": $release_crates,
  "release_docker": $release_docker,
  "release_homebrew": $release_homebrew,
  "release_java": $release_java,
  "release_csharp": $release_csharp,
  "release_go": $release_go,
  "release_wasm": $release_wasm,
  "release_php": $release_php
}
JSON
