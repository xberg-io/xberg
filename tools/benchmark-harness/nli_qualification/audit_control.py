"""Run actual stricter-policy refusal controls and restore exact source bytes."""

import hashlib
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
POLICY = Path(__file__).with_name("deny.toml")


def audit(*, advisories: bool = False) -> subprocess.CompletedProcess[str]:
    """Invoke the real offline mise task without replacing its executable."""
    executable = shutil.which("mise")
    if executable is None:
        raise RuntimeError("mise must be installed before running the audit control")
    args = [executable, "run", "nli:audit", "--", "--offline"]
    if advisories:
        args.append("--advisories")
    return subprocess.run(
        args,
        cwd=ROOT,
        env={**os.environ, "CARGO_TERM_COLOR": "never"},
        capture_output=True,
        text=True,
        check=False,
    )


def _plant(original: bytes, *, advisories: bool) -> tuple[bytes, list[str]]:
    if advisories:
        pattern = rb"(?ms)^ignore = \[\n.*?^\]"
        if len(re.findall(pattern, original)) != 1 or original.count(b"RUSTSEC-2024-0436") != 1:
            raise RuntimeError("expected exactly one paste maintenance exception to remove")
        return re.sub(pattern, b"ignore = []", original, count=1), [
            "error[unmaintained]",
            "RUSTSEC-2024-0436",
            "paste",
            "nli-qualification",
        ]
    marker = b'"MIT",'
    if original.count(marker) != 1:
        raise RuntimeError("expected exactly one MIT allowlist entry to plant the refusal")
    return original.replace(marker, b"", 1), ["error[rejected]", "MIT", "sysinfo", "nli-qualification"]


def verify(*, advisories: bool = False) -> None:
    """Require passing baseline, a real policy error and passing restoration."""
    baseline = audit(advisories=advisories)
    if baseline.returncode:
        raise RuntimeError(f"baseline audit failed:\n{baseline.stdout}{baseline.stderr}")
    original = POLICY.read_bytes()
    planted, expected = _plant(original, advisories=advisories)
    try:
        POLICY.write_bytes(planted)
        result = audit(advisories=advisories)
    finally:
        POLICY.write_bytes(original)
    if hashlib.sha256(POLICY.read_bytes()).digest() != hashlib.sha256(original).digest():
        raise RuntimeError("audit policy restoration failed")
    output = result.stdout + result.stderr
    if result.returncode == 0 or not all(value in output for value in expected):
        raise RuntimeError(f"planted policy was not rejected for the intended reason:\n{output}")
    restored = audit(advisories=advisories)
    if restored.returncode:
        raise RuntimeError(f"restored audit failed:\n{restored.stdout}{restored.stderr}")
    kind = "paste advisory severity" if advisories else "MIT license"
    sys.stdout.write(f"Audit control: actual {kind} refusal; policy restored and audit passed\n")


if __name__ == "__main__":
    if sys.argv[1:] not in [[], ["--advisories"]]:
        raise SystemExit("usage: audit_control.py [--advisories]")
    verify(advisories=bool(sys.argv[1:]))
