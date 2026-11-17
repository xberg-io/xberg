#!/usr/bin/env python3
"""Check tracked GitHub Actions pins without contacting GitHub.

Run via: uv run --offline --no-project --with PyYAML==6.0.3 python scripts/ci/check-action-pins.py
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from copy import deepcopy
from pathlib import Path
from typing import TYPE_CHECKING

import yaml
from yaml.nodes import MappingNode, ScalarNode, SequenceNode

if TYPE_CHECKING:
    from collections.abc import Iterator

    from yaml.nodes import Node

WORKFLOWS = Path(".github/workflows")
ACTION_NAMES = {"action.yml", "action.yaml"}
REMOTE_TARGET = re.compile(r"(?P<owner>[A-Za-z0-9-]+)/[A-Za-z0-9_.-]+(?:/[^\s@]+)?")
COMMIT_SHA = re.compile(r"[0-9a-fA-F]{40}")
MAJOR_TAG = re.compile(r"v(?:0|[1-9][0-9]*)")
QUALIFICATION_WORKFLOW = WORKFLOWS / "qualify-macos-elixir-nif.yaml"
QUALIFIED_ACTIONS_SHA = "97c04280c1d4acc16bdc456db188782afbe63803"
# Preserve the three exact native steps of the reproducible macOS Elixir qualification. ~keep
QUALIFIED_REFS = {
    f"xberg-io/actions/{action}@{QUALIFIED_ACTIONS_SHA}"
    for action in ("build-elixir-natives", "setup-mise", "setup-rust")
}


def line_number(node: Node) -> int:
    return node.start_mark.line + 1


def fields(loader: yaml.SafeLoader, node: Node | None, label: str) -> dict[str, Node]:
    if not isinstance(node, MappingNode):
        raise ValueError(f"{label} must be a YAML mapping")
    entries: dict[str, Node] = {}
    for key, value in node.value:
        if not isinstance(key, ScalarNode):
            raise ValueError(f"line {line_number(key)}: {label} field names must be scalars")
        if key.value in entries:
            raise ValueError(f"line {line_number(key)}: duplicate field {key.value!r} in {label}")
        entries[key.value] = value
    if any(key.tag == "tag:yaml.org,2002:merge" for key, _ in node.value):
        # Flatten a copy so another use of the same anchor still has its original fields. ~keep
        expanded = deepcopy(node)
        loader.flatten_mapping(expanded)
        entries.clear()
        for key, value in expanded.value:
            if not isinstance(key, ScalarNode):
                raise ValueError(f"line {line_number(key)}: {label} field names must be scalars")
            entries[key.value] = value
    return entries


def step_refs(loader: yaml.SafeLoader, steps: Node) -> Iterator[Node]:
    if not isinstance(steps, SequenceNode):
        raise ValueError(f"line {line_number(steps)}: steps must be a YAML sequence")
    for step in steps.value:
        uses = fields(loader, step, "step").get("uses")
        if uses is not None:
            yield uses


def uses_fields(loader: yaml.SafeLoader, path: Path) -> Iterator[Node]:
    root = fields(loader, loader.get_single_node(), "manifest")
    if path.parent == WORKFLOWS:
        for job in fields(loader, root.get("jobs"), "jobs").values():
            job_fields = fields(loader, job, "job")
            if "uses" in job_fields:
                yield job_fields["uses"]
            if "steps" in job_fields:
                yield from step_refs(loader, job_fields["steps"])
        return

    runs = fields(loader, root.get("runs"), "runs")
    using = runs.get("using")
    if not isinstance(using, ScalarNode):
        raise ValueError("action runs.using must be a scalar")
    if using.value == "composite":
        if "steps" not in runs:
            raise ValueError("composite action must define steps")
        yield from step_refs(loader, runs["steps"])


def pin_problem(path: Path, reference: str) -> str | None:
    target, separator, ref = reference.partition("@")
    remote = REMOTE_TARGET.fullmatch(target)
    if not separator or remote is None:
        return "must name a remote owner/repository@ref"
    if remote["owner"].casefold() == "xberg-io":
        if MAJOR_TAG.fullmatch(ref) or (path == QUALIFICATION_WORKFLOW and reference in QUALIFIED_REFS):
            return None
        return "must use a bare major tag such as @v1"
    return None if COMMIT_SHA.fullmatch(ref) else "must use a full 40-character commit SHA"


def check_manifest(path: Path, text: str) -> tuple[int, list[str]]:
    references: set[tuple[int, str]] = set()
    errors: list[str] = []
    loader = yaml.SafeLoader(text)
    try:
        for node in uses_fields(loader, path):
            line = line_number(node)
            if not isinstance(node, ScalarNode):
                raise ValueError(f"line {line}: uses must be a scalar")
            reference = node.value
            if reference.startswith(("./", "$/", "docker://")) or (line, reference) in references:
                continue
            references.add((line, reference))
            if problem := pin_problem(path, reference):
                errors.append(f"{path.as_posix()}:{line}: {reference!r} {problem}")
    finally:
        loader.dispose()
    return len(references), errors


def tracked_manifests(root: Path) -> list[Path]:
    output = subprocess.check_output(
        [
            "git",
            "-C",
            str(root),
            "ls-files",
            "-z",
            "--",
            WORKFLOWS.as_posix(),
            "action.yml",
            "action.yaml",
            ":(glob)**/action.yml",
            ":(glob)**/action.yaml",
        ]
    )
    paths = {Path(path) for path in os.fsdecode(output).split("\0") if path}
    return sorted(
        path
        for path in paths
        if path.name in ACTION_NAMES or (path.parent == WORKFLOWS and path.suffix in {".yml", ".yaml"})
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    root = parser.parse_args().root.resolve()
    references = 0
    errors: list[str] = []
    try:
        manifests = tracked_manifests(root)
    except (OSError, subprocess.CalledProcessError) as error:
        print(f"Could not list tracked action manifests: {error}", file=sys.stderr)
        return 1
    for path in manifests:
        try:
            source = root / path
            if not source.resolve().is_relative_to(root):
                raise ValueError("manifest resolves outside the checkout")
            checked, problems = check_manifest(path, source.read_text(encoding="utf-8"))
            references += checked
            errors.extend(problems)
        except (OSError, ValueError, RecursionError, yaml.YAMLError) as error:
            errors.append(f"{path.as_posix()}: {error}")
    print(f"Checked {references} remote uses reference(s) across {len(manifests)} manifest(s)")
    if references == 0:
        errors.append("Action pin check checked no remote uses references")
    for error in errors:
        print(error, file=sys.stderr)
    return int(bool(errors))


if __name__ == "__main__":
    sys.exit(main())
