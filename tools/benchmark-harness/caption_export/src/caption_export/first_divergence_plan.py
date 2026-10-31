"""Freeze diagnostic code and existing evidence before inspecting any new tensors."""

import argparse
import copy
import json
import sys
from pathlib import Path

from .assets import digest
from .first_divergence_evidence import PROTOCOL, require


def freeze_plan(original: Path) -> dict:
    """Pin exact local runtime and original receipts without loading a model."""
    root = Path(__file__).resolve().parent
    files = {}
    for name in ("plan.json", "parity.py", "result.json"):
        path = original / name
        require(path.is_file() and not path.is_symlink(), f"missing original regular file: {name}")
        files[name] = digest(path)
    sources = {}
    for path in sorted(root.rglob("*.py")):
        require(not path.is_symlink(), "runtime source symlink is unsupported")
        sources[str(path.relative_to(root))] = digest(path)
    require(bool(sources), "empty runtime source inventory")
    return {
        "version": 1,
        "source_root": str(root),
        "runtime_sources": sources,
        "original_directory": str(original.resolve()),
        "original_files": files,
        "protocol": copy.deepcopy(PROTOCOL),
    }


def main() -> None:
    """Write a new plan exclusively; its SHA must be supplied to the later runner."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--original", required=True, type=Path)
    parser.add_argument("--plan", required=True, type=Path)
    arguments = parser.parse_args()
    plan = freeze_plan(arguments.original)
    require(
        not arguments.plan.resolve().is_relative_to(arguments.original.resolve()), "plan overlaps original evidence"
    )
    with arguments.plan.open("x") as stream:
        json.dump(plan, stream, indent=2, sort_keys=True)
        stream.write("\n")
    sys.stdout.write(digest(arguments.plan) + "\n")


if __name__ == "__main__":
    main()
