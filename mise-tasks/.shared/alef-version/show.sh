#!/usr/bin/env bash
#MISE description="Print the workspace version from Cargo.toml"
set -euo pipefail

grep -m 1 '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/'
