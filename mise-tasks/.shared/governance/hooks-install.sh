#!/usr/bin/env bash
#MISE description="Provision machine preferences and install reviewed AI governance hooks"
set -euo pipefail

command -v npx >/dev/null || command -v ai-rulez >/dev/null || {
  echo "Neither npx nor ai-rulez is on PATH; install Node 22 or later so the hooks run npx -y ai-rulez@latest" >&2
  exit 1
}

# poly runs hooks only from the channels poly.local.toml (untracked, per machine) allows. The reviewed
# ai-rulez hooks offer an npm channel (npx -y ai-rulez@latest, org ADR-0015) and a system channel (ai-rulez
# on PATH); npm comes first so the hooks follow the latest release. ~keep
preferences=poly.local.toml
if ! grep -q '^\[hook_preferences\]' "$preferences" 2>/dev/null; then
  if [ -s "$preferences" ]; then
    printf '\n' >>"$preferences"
  fi
  printf '[hook_preferences]\nchannels = ["npm", "system"]\n' >>"$preferences"
elif ! awk '/^\[/ { in_table = ($0 == "[hook_preferences]") }
  in_table && /^channels[[:space:]]*=/ && /"(npm|system)"/ { found = 1 }
  END { exit !found }' "$preferences"; then
  echo "Add \"npm\" to hook_preferences.channels in $preferences" >&2
  exit 1
fi

poly hooks install
