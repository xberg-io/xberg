#!/usr/bin/env python3
"""Verify that benchmark workflow cells exactly match the Rust release contracts."""

from __future__ import annotations

import argparse
import itertools
import json
import re
import subprocess
from pathlib import Path

COHORTS = ("native", "ocr", "office", "markup", "ebook", "email", "data", "images")
COHORT_FILES = {
    "native": "native-pdf-fast-b8",
    "ocr": "ocr-pdf-fast-b4",
    "office": "native-office-fast",
    "markup": "native-markup-fast",
    "ebook": "native-ebook-fast",
    "email": "native-email-fast",
    "data": "native-data-fast",
    "images": "ocr-images-fast",
}
JOB_HEADER = re.compile(r"^  ([a-z0-9-]+):$")
MATRIX_VALUE = re.compile(r"\$\{\{\s*matrix\.([a-z_]+)\s*\}\}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workflow", type=Path, default=Path(".github/workflows/benchmarks.yaml"))
    parser.add_argument("--harness", type=Path, default=Path("target/release/benchmark-harness"))
    return parser.parse_args()


def split_jobs(workflow: str) -> dict[str, list[str]]:
    jobs: dict[str, list[str]] = {}
    current: str | None = None
    for line in workflow.splitlines():
        match = JOB_HEADER.match(line)
        if match:
            current = match.group(1)
            jobs[current] = []
        elif current is not None and re.match(r"^  [a-z0-9-]+:$", line):
            current = None
        elif current is not None:
            jobs[current].append(line)
    return jobs


def validate_aggregation_gate(jobs: dict[str, list[str]], benchmark_jobs: set[str]) -> None:
    try:
        aggregate = jobs["aggregate-and-publish"]
    except KeyError as error:
        raise ValueError("workflow has no aggregate-and-publish job") from error

    aggregate_text = "\n".join(aggregate)
    missing_needs = sorted(job for job in benchmark_jobs if f"        {job}," not in aggregate)
    if missing_needs:
        raise ValueError(f"aggregate-and-publish does not need jobs: {', '.join(missing_needs)}")
    # `always()` lets optional jobs fail; raw validation still rejects absent required cells. ~keep
    if "if: ${{ always()" not in aggregate_text:
        raise ValueError("aggregate-and-publish must use always() so optional cells are best-effort")
    if "      - name: Validate benchmark artifacts (all cohorts)" not in aggregate:
        raise ValueError("aggregate-and-publish must validate raw artifacts before publication")
    ordered_gates = [
        "      - name: Validate benchmark artifacts (all cohorts)",
        "      - name: Consolidate results (all cohorts)",
        "      - name: Validate aggregates (all cohorts)",
        "      - name: Prepare cohort release assets",
        "      - name: Create GitHub Release with benchmark results",
    ]
    try:
        gate_indexes = [aggregate.index(step) for step in ordered_gates]
    except ValueError as error:
        raise ValueError("aggregate-and-publish is missing a validation or publication gate") from error
    # Both raw and consolidated data must be validated before release assets can be prepared. ~keep
    if gate_indexes != sorted(gate_indexes):
        raise ValueError("aggregate-and-publish validation gates must precede publication")


def validate_tika_artifact(jobs: dict[str, list[str]]) -> None:
    prefetch = jobs.get("prefetch-tika", [])
    tika = jobs.get("bench-tika", [])
    prefetch_text = "\n".join(prefetch)
    tika_text = "\n".join(tika)
    if prefetch_text.count("repo.maven.apache.org/maven2/org/apache/tika/") != 1:
        raise ValueError("prefetch-tika must download Apache Tika exactly once")
    if "${TIKA_SHA256}  ${jar_path}" not in prefetch_text:
        raise ValueError("prefetch-tika must verify the pinned Apache Tika checksum")
    for job, text in (("prefetch-tika", prefetch_text), ("bench-tika", tika_text)):
        if "          name: apache-tika-jar" not in text:
            raise ValueError(f"{job} must use the shared apache-tika-jar artifact")
    if "repo1.maven.org" in tika_text or "repo.maven.apache.org" in tika_text:
        raise ValueError("bench-tika matrix jobs must not download Apache Tika independently")


def strip_comment(value: str) -> str:
    return value.split("#", 1)[0].strip()


def parse_list(lines: list[str], start: int, initial: str) -> tuple[list[str], int]:
    if not initial and start + 1 < len(lines) and lines[start + 1].strip().startswith("["):
        start += 1
        initial = strip_comment(lines[start])
    if initial.startswith("["):
        chunks = [initial]
        while not chunks[-1].rstrip().endswith("]"):
            start += 1
            chunks.append(strip_comment(lines[start]))
        raw = " ".join(chunks)[1:-1]
        return [item.strip().strip("'\"") for item in raw.split(",") if item.strip()], start

    values: list[str] = []
    while start + 1 < len(lines):
        candidate = lines[start + 1]
        match = re.match(r"^          -\s+([^{}].*)$", candidate)
        if not match:
            break
        values.append(strip_comment(match.group(1)).strip("'\""))
        start += 1
    return values, start


def parse_inline_map(value: str) -> dict[str, str]:
    return {
        key.strip(): item.strip().strip("'\"")
        for key, item in (pair.split(":", 1) for pair in value.strip("{} ").split(","))
    }


def parse_excludes(lines: list[str], start: int) -> tuple[list[dict[str, str]], int]:
    excludes: list[dict[str, str]] = []
    while start + 1 < len(lines):
        candidate = lines[start + 1]
        inline = re.match(r"^          -\s+(\{.*\})\s*$", candidate)
        block = re.match(r"^          -\s+([a-z_]+):\s*(.+?)\s*$", candidate)
        if inline:
            excludes.append(parse_inline_map(inline.group(1)))
            start += 1
        elif block:
            entry = {block.group(1): strip_comment(block.group(2)).strip("'\"")}
            start += 1
            while start + 1 < len(lines):
                continuation = re.match(r"^            ([a-z_]+):\s*(.+?)\s*$", lines[start + 1])
                if not continuation:
                    break
                entry[continuation.group(1)] = strip_comment(continuation.group(2)).strip("'\"")
                start += 1
            excludes.append(entry)
        else:
            break
    return excludes, start


def parse_matrix(job: str, lines: list[str]) -> tuple[dict[str, list[str]], list[dict[str, str]]]:
    try:
        start = lines.index("      matrix:") + 1
    except ValueError as error:
        raise ValueError(f"{job}: benchmark job has no matrix") from error

    dimensions: dict[str, list[str]] = {}
    excludes: list[dict[str, str]] = []
    index = start
    while index < len(lines):
        line = lines[index]
        if line == "    steps:":
            break
        field = re.match(r"^        ([a-z_]+):\s*(.*?)\s*$", line)
        if not field:
            index += 1
            continue
        key, initial = field.groups()
        if key == "exclude":
            excludes, index = parse_excludes(lines, index)
        elif key != "include":
            values, index = parse_list(lines, index, strip_comment(initial))
            if not values:
                raise ValueError(f"{job}: matrix dimension {key!r} has no values")
            dimensions[key] = values
        index += 1

    if not dimensions:
        raise ValueError(f"{job}: benchmark matrix is empty")
    return dimensions, excludes


def named_step(job: str, lines: list[str], name: str) -> list[str]:
    header = f"      - name: {name}"
    try:
        start = lines.index(header)
    except ValueError as error:
        raise ValueError(f"{job}: missing {name!r} step") from error
    end = start + 1
    while end < len(lines) and not lines[end].startswith("      - "):
        end += 1
    return lines[start:end]


def require_no_continue_on_error(job: str, name: str, step: list[str]) -> None:
    for line in step:
        match = re.match(r"^        continue-on-error:\s*(.+?)\s*$", line)
        if match and match.group(1) != "false":
            raise ValueError(f"{job}: {name} must not use continue-on-error")


def artifact_template(job: str, step: list[str]) -> str:
    matches = []
    for line in step:
        match = re.match(
            r"^          name:\s*(benchmarks-.+)-\$\{\{\s*github\.run_id\s*\}\}\s*$",
            line,
        )
        if match:
            matches.append(match.group(1))
    if len(matches) != 1:
        raise ValueError(f"{job}: expected one benchmark artifact upload name, found {len(matches)}")
    return matches[0]


def step_environment(job: str, step: list[str]) -> dict[str, str]:
    environment = {}
    for line in step:
        match = re.match(r"^          ([A-Z_]+):\s*(.+?)\s*$", line)
        if match:
            environment[match.group(1)] = match.group(2).strip("'\"")
    required = {"FRAMEWORK", "MODE", "OUTPUT_FORMAT", "COHORT"}
    if missing := required - environment.keys():
        raise ValueError(f"{job}: Run benchmark is missing environment keys: {', '.join(sorted(missing))}")
    if not any(line.strip() == "run: scripts/benchmarks/run-benchmark.sh" for line in step):
        raise ValueError(f"{job}: Run benchmark must invoke scripts/benchmarks/run-benchmark.sh")
    return environment


def interpolate(job: str, template: str, cell: dict[str, str]) -> str:
    try:
        value = MATRIX_VALUE.sub(lambda match: cell[match.group(1)], template)
    except KeyError as error:
        raise ValueError(f"{job}: expression references missing matrix key {error.args[0]!r}") from error
    if "${{" in value:
        raise ValueError(f"{job}: unresolved expression in {template!r}")
    return value


def is_optional_cell(framework: str, cohort: str) -> bool:
    if framework == "mineru":
        return True
    if cohort in {"native-pdf-fast-b8", "ocr-pdf-fast-b4"}:
        return cohort == "ocr-pdf-fast-b4" and framework in {"tika", "unstructured"}
    return not framework.startswith("xberg-")


def workflow_cells(workflow: str) -> list[dict[str, object]]:
    cells: list[dict[str, object]] = []
    jobs = split_jobs(workflow)
    benchmark_jobs = {job for job in jobs if job.startswith("bench-") and "      matrix:" in jobs[job]}
    validate_aggregation_gate(jobs, benchmark_jobs)
    validate_tika_artifact(jobs)
    for job in sorted(benchmark_jobs):
        lines = jobs[job]
        dimensions, excludes = parse_matrix(job, lines)
        run_step = named_step(job, lines, "Run benchmark")
        upload_step = named_step(job, lines, "Upload artifacts")
        require_no_continue_on_error(job, "Run benchmark", run_step)
        require_no_continue_on_error(job, "Upload artifacts", upload_step)
        environment = step_environment(job, run_step)
        template = artifact_template(job, upload_step)
        keys = tuple(dimensions)
        for values in itertools.product(*(dimensions[key] for key in keys)):
            cell = dict(zip(keys, values, strict=True))
            if any(all(cell.get(key) == value for key, value in exclusion.items()) for exclusion in excludes):
                continue

            # Matrix interpolation is intentionally the only supported dynamic component. ~keep
            framework = interpolate(job, environment["FRAMEWORK"], cell)
            cohort_path = interpolate(job, environment["COHORT"], cell)
            cohort = Path(cohort_path).stem
            cells.append(
                {
                    "artifact": interpolate(job, template, cell),
                    "framework": framework,
                    "output_format": interpolate(job, environment["OUTPUT_FORMAT"], cell),
                    "mode": interpolate(job, environment["MODE"], cell),
                    "cohort": cohort,
                    "optional": is_optional_cell(framework, cohort),
                }
            )
    return cells


def contract_cells(harness: Path) -> list[dict[str, object]]:
    cells: list[dict[str, object]] = []
    for cohort in COHORTS:
        result = subprocess.run(
            [str(harness), "cohort-contract", "--cohort", cohort],
            check=True,
            capture_output=True,
            text=True,
        )
        contract = json.loads(result.stdout)
        matrix = contract.get("matrix")
        if not isinstance(matrix, list):
            raise ValueError(f"{cohort}: cohort-contract output has no matrix array")
        for entry in matrix:
            cells.append({**entry, "cohort": COHORT_FILES[cohort]})
    return cells


def cell_key(cell: dict[str, object]) -> tuple[object, ...]:
    return tuple(
        cell[key]
        for key in (
            "artifact",
            "framework",
            "output_format",
            "mode",
            "cohort",
            "optional",
        )
    )


def require_unique(name: str, cells: list[dict[str, object]]) -> set[tuple[object, ...]]:
    keys = [cell_key(cell) for cell in cells]
    unique = set(keys)
    if len(unique) != len(keys):
        duplicates = sorted({key for key in keys if keys.count(key) > 1})
        raise ValueError(f"{name} contains duplicate benchmark cells: {duplicates}")
    return unique


def main() -> None:
    args = parse_args()
    actual = require_unique("benchmark workflow", workflow_cells(args.workflow.read_text()))
    expected = require_unique("Rust benchmark contracts", contract_cells(args.harness))
    if actual != expected:
        missing = sorted(expected - actual)
        unexpected = sorted(actual - expected)
        details = []
        if missing:
            details.append("missing workflow cells:\n  " + "\n  ".join(map(str, missing)))
        if unexpected:
            details.append("unexpected workflow cells:\n  " + "\n  ".join(map(str, unexpected)))
        raise SystemExit("benchmark workflow matrix does not match Rust contracts\n" + "\n".join(details))
    print(f"benchmark workflow matrix matches all {len(expected)} Rust contract cells")


if __name__ == "__main__":
    main()
