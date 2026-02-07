#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

source "$REPO_ROOT/scripts/lib/common.sh"

validate_repo_root "$REPO_ROOT" || exit 1

cd "$REPO_ROOT/packages/java"

# Skip tests and static analysis for benchmark builds - these are already run in CI
# This reduces build time from 3+ hours to just compilation time
mvn -q -B -U package \
  -DskipTests \
  -Dcheckstyle.skip=true \
  -Dpmd.skip=true \
  -Djacoco.skip=true

# Copy runtime dependencies (e.g. Jackson) to target/dependency/ for benchmark classpath
mvn -q -B dependency:copy-dependencies \
  -DincludeScope=runtime \
  -DoutputDirectory=target/dependency

# Compile the benchmark wrapper class into target/classes
BENCH_SCRIPT="$REPO_ROOT/tools/benchmark-harness/scripts/KreuzbergExtractJava.java"
if [ -f "$BENCH_SCRIPT" ]; then
  CP="target/classes"
  for jar in target/dependency/*.jar; do
    [ -f "$jar" ] && CP="$CP:$jar"
  done
  javac -cp "$CP" -d target/classes "$BENCH_SCRIPT"
fi
