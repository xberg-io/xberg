#!/usr/bin/env python3
"""Package and verify every crate the release publishes to crates.io, with warnings denied.

Mirrors the verification build `cargo publish` runs in the publish-crates job: each crate is
packaged and compiled standalone from its tarball at default features, under
`CARGO_BUILD_WARNINGS=deny` as `setup-rust` sets it. One `cargo package` call covers all crates,
so crates depending on unpublished workspace versions resolve them from cargo's local overlay
instead of crates.io.

The crate list is read from the publish-crates step in `.github/workflows/publish.yaml`, so this
check and the release cannot disagree on what ships.
"""

from __future__ import annotations

import json
import os
import re
import signal
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PUBLISH_WORKFLOW = REPO_ROOT / ".github/workflows/publish.yaml"
PUBLISH_CRATES_STEP = re.compile(
    r"uses: xberg-io/actions/publish-crates@\S+\s+with:\s+crates:\s*(?P<crates>[^\n]+)",
)
DEPENDENCY_SECTION = re.compile(r"^\[(?:target\..+\.)?(?:dev-|build-)?dependencies\]\s*$")
PATH_DEP_START = re.compile(r"""^\s*[A-Za-z0-9_-]+\s*=\s*\{""")
PATH_VALUE = re.compile(r"""path\s*=\s*"[^"]*\"""")
VERSION_KEY = re.compile(r"(?<![\w-])(?:version|workspace)\s*=")


def publish_crates() -> list[str]:
    match = PUBLISH_CRATES_STEP.search(PUBLISH_WORKFLOW.read_text(encoding="utf-8"))
    if match is None:
        sys.exit(f"error: no publish-crates `crates:` input found in {PUBLISH_WORKFLOW}")
    return match.group("crates").split()


def workspace_manifests() -> tuple[str, dict[str, Path]]:
    metadata = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            check=True,
        ).stdout,
    )
    packages = {package["name"]: package for package in metadata["packages"]}
    return packages["xberg"]["version"], {name: Path(package["manifest_path"]) for name, package in packages.items()}


def inject_path_dep_versions(manifest: str, version: str) -> str:
    """Give each version-less path dependency the release version, as publish-crates does.

    Some path dependencies omit `version` on purpose (the xberg-tesseract `links` collision in
    the maturin sdist); the publish action adds it at publish time, and `cargo package` refuses
    the manifest without it.
    """
    lines = manifest.splitlines(keepends=True)
    output: list[str] = []
    in_dependencies = False
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.lstrip().startswith("["):
            in_dependencies = bool(DEPENDENCY_SECTION.match(line.strip()))
        if not (in_dependencies and PATH_DEP_START.match(line)):
            output.append(line)
            index += 1
            continue
        entry = [line]
        depth = line.count("{") - line.count("}")
        while depth > 0 and index + 1 < len(lines):
            index += 1
            entry.append(lines[index])
            depth += lines[index].count("{") - lines[index].count("}")
        text = "".join(entry)
        path_value = PATH_VALUE.search(text)
        if path_value and not VERSION_KEY.search(text):
            text = f'{text[: path_value.end()]}, version = "{version}"{text[path_value.end() :]}'
        output.append(text)
        index += 1
    return "".join(output)


def verified_crates(output: str, version: str) -> set[str]:
    return set(re.findall(rf"Verifying (\S+) v{re.escape(version)}", output))


def exit_on_signal(signum: int, _frame: object) -> None:
    sys.exit(128 + signum)


def main() -> int:
    # The `finally` below restores the rewritten manifests; SIGTERM would skip it. ~keep
    signal.signal(signal.SIGTERM, exit_on_signal)
    crates = publish_crates()
    version, manifests = workspace_manifests()
    missing = [crate for crate in crates if crate not in manifests]
    if missing:
        sys.exit(f"error: publish-crates lists crates that are not workspace members: {', '.join(missing)}")

    originals = {crate: manifests[crate].read_text(encoding="utf-8") for crate in crates}
    env = {**os.environ, "CARGO_BUILD_WARNINGS": "deny", "CARGO_TERM_COLOR": "never"}
    command = ["cargo", "package", "--allow-dirty", *(arg for crate in crates for arg in ("-p", crate))]
    print(f"Verifying {len(crates)} crate(s) at {version}: {' '.join(crates)}", flush=True)
    try:
        for crate, original in originals.items():
            rewritten = inject_path_dep_versions(original, version)
            if rewritten != original:
                manifests[crate].write_text(rewritten, encoding="utf-8")
        with subprocess.Popen(
            command, cwd=REPO_ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
        ) as process:
            assert process.stdout is not None
            lines = []
            for line in process.stdout:
                sys.stdout.write(line)
                sys.stdout.flush()
                lines.append(line)
        returncode = process.returncode
    finally:
        for crate, original in originals.items():
            manifests[crate].write_text(original, encoding="utf-8")

    if returncode != 0:
        print(f"error: `{' '.join(command)}` exited {returncode}", file=sys.stderr)
        return returncode
    unverified = sorted(set(crates) - verified_crates("".join(lines), version))
    if unverified:
        print(f"error: cargo exited 0 without verifying: {', '.join(unverified)}", file=sys.stderr)
        return 1
    print(f"Verified {len(crates)} of {len(crates)} crate package(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
