#!/usr/bin/env bash
#MISE description="Validate that AI governance inputs resolve and render without modifying the checkout"
#MISE depends=["governance:validate"]
set -euo pipefail

governance_verify_root="$(mktemp -d)"
trap 'rm -rf -- "$governance_verify_root"' EXIT
cp -R .ai-rulez "$governance_verify_root/.ai-rulez"
cd "$governance_verify_root"
npx -y ai-rulez@latest generate --no-self-mcp --quiet
