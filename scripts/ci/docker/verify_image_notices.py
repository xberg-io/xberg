#!/usr/bin/env python3
"""Verify the canonical notices inside a built Xberg image byte for byte.

The Dockerfiles copy `LICENSE` and `THIRD_PARTY_LICENSES.md` into
`/usr/local/share/licenses/xberg/`, but a text lint over the Dockerfiles cannot
tell whether the runtime stage actually kept them, whether the build context
carried the current bytes, or whether the copy was truncated. This gate opens
the image and compares the packaged bytes with the repository copies.

The Docker invocation is injected so unit tests exercise the comparison without
a daemon; `docker run --rm --pull=never --entrypoint cat IMAGE PATH` streams the
file out and fails on a missing path, which maps directly to a missing notice.
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from collections.abc import Callable, Sequence
from pathlib import Path, PurePosixPath

REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
NOTICE_NAMES = ("LICENSE", "THIRD_PARTY_LICENSES.md")
NOTICE_DIRECTORY = PurePosixPath("/usr/local/share/licenses/xberg")

DockerRunner = Callable[[Sequence[str]], subprocess.CompletedProcess[bytes]]


def run_docker(arguments: Sequence[str]) -> subprocess.CompletedProcess[bytes]:
    """Run one Docker CLI invocation with its stdout and stderr captured as bytes."""
    executable = shutil.which("docker")
    if executable is None:
        raise FileNotFoundError("docker is not on PATH")
    return subprocess.run([executable, *arguments], capture_output=True, check=False)


def canonical_notices(root: Path) -> dict[str, bytes]:
    """Return the repository's canonical notices, rejecting an empty source file."""
    notices = {}
    for name in NOTICE_NAMES:
        content = (root / name).read_bytes()
        if not content:
            raise ValueError(f"empty canonical notice: {name}")
        notices[name] = content
    return notices


def read_notice(image: str, name: str, run: DockerRunner) -> tuple[bytes | None, str]:
    """Read one notice out of the image, returning the failure detail when it cannot be read."""
    path = NOTICE_DIRECTORY / name
    try:
        result = run(["run", "--rm", "--pull=never", "--entrypoint", "cat", image, str(path)])
    except OSError as error:
        return None, f"docker could not be executed: {error}"
    if result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip().splitlines()
        return None, detail[-1] if detail else f"docker exited {result.returncode}"
    return result.stdout, ""


def describe_mismatch(packaged: bytes, expected: bytes) -> str:
    """Explain how a packaged notice differs from the canonical bytes."""
    if not packaged:
        return "is empty"
    if len(packaged) < len(expected) and expected.startswith(packaged):
        return f"is truncated ({len(packaged)} of {len(expected)} bytes)"
    return f"differs from the repository copy ({len(packaged)} bytes vs {len(expected)})"


def verify_image(image: str, root: Path, run: DockerRunner = run_docker) -> list[str]:
    """Return every reason the image's packaged notices do not match the repository."""
    findings = []
    for name, expected in canonical_notices(root).items():
        packaged, error = read_notice(image, name, run)
        if packaged is None:
            findings.append(f"{image}: {name} is missing from {NOTICE_DIRECTORY} ({error})")
        elif packaged != expected:
            findings.append(f"{image}: {name} {describe_mismatch(packaged, expected)}")
    return findings


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    """Parse the image reference and the optional repository root."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--image", required=True, help="built image reference, for example xberg:core")
    parser.add_argument(
        "--repository-root",
        type=Path,
        default=REPOSITORY_ROOT,
        help="checkout holding the canonical LICENSE and THIRD_PARTY_LICENSES.md",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    """Exit non-zero, listing every finding, when the image notices are missing or stale."""
    args = parse_args(argv)
    findings = verify_image(args.image, args.repository_root)
    if findings:
        for finding in findings:
            print(f"error: {finding}", file=sys.stderr)
        return 1
    notices = canonical_notices(args.repository_root)
    sizes = ", ".join(f"{name} ({len(content)} bytes)" for name, content in notices.items())
    print(f"{args.image}: packaged notices match the repository byte-for-byte: {sizes}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
