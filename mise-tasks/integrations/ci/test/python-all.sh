#!/usr/bin/env bash
#MISE description="Test every Python integration against a locally built xberg wheel"
#USAGE flag "--wheel-dir <dir>" env="WHEEL_DIR" required=#true help="Directory holding the locally built xberg wheel"
# shellcheck disable=SC2154 # usage_* are set by mise from the #USAGE lines
set -euo pipefail

# shellcheck source=mise-tasks/integrations/.lib/packages.sh
. "$MISE_CONFIG_ROOT/mise-tasks/integrations/.lib/packages.sh"
for package in "${py_test_packages[@]}"; do
  test_python_integration "$package" "$usage_wheel_dir"
done
