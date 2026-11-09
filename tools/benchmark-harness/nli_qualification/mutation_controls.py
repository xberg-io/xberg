"""Execute curated source-bypass controls in an exclusively owned worktree."""

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

PACKAGE = Path(__file__).resolve().parent
ROOT = PACKAGE.parents[2]


@dataclass(frozen=True)
class Control:
    """Declare one reversible edit and the test assertion it must invalidate."""

    id: str
    name: str
    path: str
    before: str
    after: str
    args: list[str]
    failure_marker: str


def load_controls() -> list[Control]:
    """Validate the complete manifest, source paths and allowed test commands."""
    value = json.loads((PACKAGE / "mutation-controls.json").read_text())
    if not isinstance(value, dict):
        raise ValueError("mutation manifest must be an object")
    if value.get("schema_version") != 1 or value.get("expected_controls") != 67:
        raise ValueError("unsupported mutation manifest or control inventory")
    rows = value.get("controls")
    if not isinstance(rows, list) or len(rows) != value["expected_controls"]:
        raise ValueError("mutation inventory is empty or incomplete")
    controls = []
    for row in rows:
        if not isinstance(row, dict) or set(row) != set(Control.__dataclass_fields__):
            raise ValueError("invalid mutation control fields")
        if not all(isinstance(row[key], str) for key in row if key != "args"):
            raise ValueError("control strings must be strings")
        if not isinstance(row["args"], list) or not all(isinstance(arg, str) for arg in row["args"]):
            raise ValueError("control arguments must be strings")
        control = Control(**row)
        path = (PACKAGE / control.path).resolve()
        if not path.is_relative_to(PACKAGE / "src") or path.suffix != ".rs":
            raise ValueError("mutations may touch only leaf Rust sources")
        if (
            not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", control.id)
            or not control.before
            or not control.failure_marker
        ):
            raise ValueError("empty mutation selector or failure marker")
        portable = ["--test", "cli", "--test", "refusals"]
        sdk = [
            "--features",
            "runtime",
            "--test",
            "refusals",
            "portable_admission_agrees_with_sdk_constructor_on_edge_configurations",
        ]
        if control.args not in [portable, sdk]:
            raise ValueError("only the curated leaf test commands may be executed")
        controls.append(control)
    if len({control.id for control in controls}) != len(controls):
        raise ValueError("duplicate mutation IDs")
    return controls


def run(args: list[str], log: Path) -> subprocess.CompletedProcess[str]:
    """Execute the real mise file task and retain its output outside the checkout."""
    executable = shutil.which("mise")
    if executable is None:
        raise RuntimeError("mise must be installed before running mutation controls")
    result = subprocess.run(
        [executable, "run", "nli:test", "--", *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    log.write_text(result.stdout + result.stderr)
    return result


def passing_baselines(controls: list[Control], output: Path, phase: str) -> None:
    """Require nonempty passing tests for every selected test command."""
    commands = sorted({tuple(control.args) for control in controls})
    for index, command in enumerate(commands):
        result = run(list(command), output / f"{phase}-{index}.log")
        text = result.stdout + result.stderr
        if result.returncode or "test result: ok." not in text or " ... ok" not in text:
            raise RuntimeError(f"{phase} did not execute passing tests: {command}")


def probe(control: Control, output: Path) -> dict[str, str]:
    """Apply one edit, require its declared assertion failure and restore exact bytes."""
    path = (PACKAGE / control.path).resolve()
    original = path.read_bytes()
    text = original.decode()
    if text.count(control.before) != 1:
        raise ValueError(f"{control.id}: expected exactly one matching source selector")
    try:
        path.write_text(text.replace(control.before, control.after, 1))
        result = run(control.args, output / f"{control.id}.log")
    finally:
        path.write_bytes(original)
    restored = hashlib.sha256(path.read_bytes()).hexdigest()
    if restored != hashlib.sha256(original).hexdigest():
        raise RuntimeError(f"{control.id}: source restoration failed")
    text = result.stdout + result.stderr
    killed = result.returncode != 0 and "test result: FAILED" in text and control.failure_marker in text
    return {"id": control.id, "status": "killed" if killed else "invalid-or-survived", "restored_sha256": restored}


def main() -> None:
    """Select a nonempty inventory and execute it with baseline/restoration checks."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--id", action="append", default=[])
    parser.add_argument("--results", type=Path)
    args = parser.parse_args()
    controls = load_controls()
    if args.id:
        requested = set(args.id)
        controls = [control for control in controls if control.id in requested]
        if {control.id for control in controls} != requested:
            raise ValueError("unknown or empty mutation selection")
    if args.list:
        for control in controls:
            sys.stdout.write(f"{control.id}: {control.name}\n")
        sys.stdout.write(f"{len(controls)} mutation controls\n")
        return
    if args.results is None or not controls:
        parser.error("execution requires --results and at least one control")
    output = args.results.resolve()
    if output.is_relative_to(ROOT):
        parser.error("write mutation logs outside the repository")
    if output.exists() and any(output.iterdir()):
        parser.error("choose an empty results directory to preserve existing evidence")
    output.mkdir(parents=True, exist_ok=True)
    passing_baselines(controls, output, "baseline")
    results = []
    for control in controls:
        result = probe(control, output)
        results.append(result)
        (output / "results.json").write_text(json.dumps(results, indent=2) + "\n")
        sys.stdout.write(f"{control.id}: {result['status']}\n")
        sys.stdout.flush()
    passing_baselines(controls, output, "restored")
    if any(result["status"] != "killed" for result in results):
        raise RuntimeError("not every selected control produced an intended assertion failure")
    sys.stdout.write(f"{len(results)} executable controls killed; restored tests passed\n")


if __name__ == "__main__":
    main()
