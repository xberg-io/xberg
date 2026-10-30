#!/usr/bin/env bash
#MISE description="Test every Node integration against a locally built Node binding"
#USAGE flag "--binding-dir <dir>" env="BINDING_DIR" required=#true help="Directory holding the locally built Node binding"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

# shellcheck source=mise-tasks/integrations/.lib/packages.sh
. "$MISE_CONFIG_ROOT/mise-tasks/integrations/.lib/packages.sh"
binding_dir="$usage_binding_dir"
npm pkg delete optionalDependencies --prefix "$binding_dir"
# Packages with no test script yet. `npm test --if-present` is a SILENT no-op for a package without
# one, so without this list a package that has tests today and loses them tomorrow keeps reporting
# success while verifying nothing. Naming the exception here means the gap is visible and a
# regression in the other two is caught. Remove an entry as soon as that package gains real
# tests. ~keep
untested_packages="n8n-nodes-xberg"
tested=0
for package in "${node_test_packages[@]}"; do
  (
    cd "integrations/node/${package}"
    npm pkg set "dependencies.@xberg-io/xberg=file:${binding_dir}"
    npm install --no-audit --no-fund
    npm run lint --if-present
    npm run build --if-present
    if node -e 'process.exit((require("./package.json").scripts||{}).test?0:1)'; then
      npm test
    elif printf '%s\n' ${untested_packages} | grep -qx "${package}"; then
      echo "note: ${package} declares no test script (known gap, see untested_packages)"
    else
      echo "error: ${package} declares no test script and is not a known gap -- its" \
        "tests would be skipped silently" >&2
      exit 1
    fi
  )
  tested=$((tested + 1))
done
expected="${#node_test_packages[@]}"
if [ "${tested}" -ne "${expected}" ]; then
  echo "error: iterated ${tested} node packages but expected ${expected}" >&2
  exit 1
fi
