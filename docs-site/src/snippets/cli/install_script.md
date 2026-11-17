```bash title="Bash"
curl -fsSL https://raw.githubusercontent.com/xberg-io/xberg/main/scripts/install.sh | bash
```

The installer needs HTTPS access to GitHub release downloads and `api.github.com`, plus `awk` and
either `sha256sum` or `shasum`. It checks the archive against GitHub's release digest before extraction.
Set `GITHUB_TOKEN` in the environment if you need authenticated GitHub API requests.
When `XBERG_VERSION` is unset, the script chooses the latest stable release listed by GitHub.
