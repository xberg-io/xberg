#!/usr/bin/env bash
set -euo pipefail

run_id="$(
	gh run list \
		--workflow=benchmarks.yaml \
		--status=success \
		--limit=1 \
		--json databaseId \
		--jq '.[0].databaseId'
)"

if [ -z "$run_id" ]; then
	echo "No successful benchmark runs found" >&2
	exit 1
fi

echo "Found benchmark run: $run_id"
echo "run_id=$run_id" >>"$GITHUB_OUTPUT"

run_date="$(gh run view "$run_id" --json createdAt --jq '.createdAt')"
echo "run_date=$run_date" >>"$GITHUB_OUTPUT"
echo "Benchmark run date: $run_date"
