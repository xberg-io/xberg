#!/usr/bin/env bash

set -euo pipefail

pom_file="${1:-packages/java/pom.xml}"

if [ ! -f "$pom_file" ]; then
  echo "Error: pom.xml not found: $pom_file" >&2
  exit 1
fi

if grep -q '<arg>--pinentry-mode</arg>' "$pom_file"; then
  python3 - "$pom_file" <<'PYTHON'
import re
import sys
from pathlib import Path

path = Path(sys.argv[1])
path.write_text(re.sub(
    r"<arg>--pinentry-mode</arg>\s*<arg>loopback</arg>",
    "<arg>--pinentry-mode=loopback</arg>",
    path.read_text(),
))
PYTHON
  echo "Patched legacy GPG pinentry argument format in $pom_file"
else
  echo "No legacy GPG arguments found in $pom_file"
fi
