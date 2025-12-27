#!/usr/bin/env bash

set -euo pipefail

if command -v gpg2 >/dev/null 2>&1; then
	mkdir -p "${HOME}/.local/bin"
	printf '#!/usr/bin/env bash\nexec gpg2 "$@"\n' >"${HOME}/.local/bin/gpg"
	chmod +x "${HOME}/.local/bin/gpg"
	echo "${HOME}/.local/bin" >>"$GITHUB_PATH"
	echo "PATH=${HOME}/.local/bin:${PATH}" >>"$GITHUB_ENV"
	echo "gpg2 binary preference configured"
else
	echo "gpg2 not found; using default gpg"
fi
