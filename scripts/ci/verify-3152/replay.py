"""Run one managed worker per case and retain raw evidence incrementally."""

import argparse
import hashlib
import io
import json
import os
import platform
import queue
import re
import shutil
import statistics
import subprocess
import tarfile
import threading
import time
from itertools import pairwise
from pathlib import Path
from typing import Any

from fixtures import MIB, digest, docx, generate, pdf

HERE = Path(__file__).resolve().parent
SOURCE = "packages/csharp/src"
MODES = ("path", "bytes", "factory")
TRIALS = 3
ORT_CORE = {"Linux": "libonnxruntime.so.1", "Windows": "onnxruntime.dll"}


def require(condition: object, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def private_rss(pid: int) -> int:
    values = {}
    for line in Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines():
        key, _, value = line.partition(":")
        if key in ("Private_Clean", "Private_Dirty", "Private_Hugetlb"):
            number, unit = value.split()
            require(unit == "kB", f"Unexpected smaps unit {unit}")
            values[key] = int(number) * 1024
    require({"Private_Clean", "Private_Dirty"} <= values.keys(), "Missing private RSS counters")
    return sum(values.values())


def receive(rows: queue.Queue[str | None], stage: str, timeout: int, log: Path) -> dict[str, Any]:
    line = rows.get(timeout=timeout)
    require(line is not None, f"Worker exited before {stage}. See {log}")
    value = json.loads(line)
    require(value["stage"] == stage, f"Expected {stage}, received {value}")
    return value


def send(child: subprocess.Popen[str], signal: str) -> None:
    child.stdin.write(signal + "\n")
    child.stdin.flush()


def trial(
    command: list[str],
    fixture: Path,
    mode: str,
    log: Path,
    *,
    env: dict[str, str],
    interval: float,
    timeout: int,
    runtime: str,
) -> dict[str, Any]:
    rows = queue.Queue()
    stop = threading.Event()
    samples, errors = [], []
    with (
        log.open("w", encoding="utf-8") as stderr,
        subprocess.Popen(
            [*command, mode, str(fixture), str(fixture.with_stem("warm"))],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=stderr,
            text=True,
            env=env,
        ) as child,
    ):

        def consume() -> None:
            for line in child.stdout:
                rows.put(line)
            rows.put(None)

        def sample() -> None:
            samples.append((time.monotonic(), private_rss(child.pid)))

        def poll() -> None:
            try:
                while not stop.is_set():
                    sample()
                    stop.wait(interval)
            except Exception as exc:
                errors.append(exc)

        threading.Thread(target=consume, daemon=True).start()
        sampler = None
        try:
            ready = receive(rows, "ready", timeout, log)
            require(ready["pid"] == child.pid, "Worker PID mismatch")
            require(ready["architecture"] == "X64", "Expected an x64 worker")
            require(ready["runtime"] == f".NET {runtime}", "Unexpected worker runtime")
            linux = platform.system() == "Linux"
            if linux:
                sample()
                sampler = threading.Thread(target=poll, daemon=True)
            start = time.monotonic()
            send(child, "go")
            if sampler:
                sampler.start()
            receive(rows, "measured", timeout, log)
            end = time.monotonic()
            stop.set()
            if sampler:
                sampler.join()
                require(not errors, f"Linux sampler failed: {errors}")
                sample()
            send(child, "report")
            result = receive(rows, "result", timeout, log)
            require(child.wait(timeout=timeout) == 0, f"Worker failed. See {log}")
            return {
                **ready,
                **result,
                **measure(ready, result, samples, start, end),
                "mode": mode,
                "fixture": fixture.name,
            }
        finally:
            stop.set()
            if sampler and sampler.ident is not None:
                sampler.join()
            if child.poll() is None:
                child.kill()


def measure(
    ready: dict[str, Any],
    result: dict[str, Any],
    samples: list[tuple[float, int]],
    start: float,
    end: float,
) -> dict[str, Any]:
    if samples:
        require(len(samples) >= 2 and samples[-1][0] >= end, "Missing Linux completion sample")
        baseline = samples[0][1]
        # The last read is synchronous while the worker waits for the report signal.
        completion = samples[-1][1]
        peak = max(completion, *(value for t, value in samples[:-1] if t <= end))
        extras = {
            "window_ms": (end - start) * 1000,
            "completion_private_bytes": completion,
            "samples": [[round((t - start) * 1000, 3), n] for t, n in samples],
            "max_sample_gap_ms": max(b[0] - a[0] for a, b in pairwise(samples)) * 1000,
        }
        metric = "sampled_private_resident_growth_bytes"
    else:
        require(
            0 < ready["readyCurrent"] <= ready["readyPeak"] <= result["endPeak"],
            "Invalid Windows commit high-water counters",
        )
        require(0 < result["endCurrent"] <= result["endPeak"], "Invalid Windows end private commit")
        baseline, peak = ready["readyPeak"], result["endPeak"]
        extras = {"ready_peak_minus_current": baseline - ready["readyCurrent"]}
        metric = "private_commit_high_water_growth_bytes"
    return {**extras, "metric": metric, "baseline": baseline, "peak": peak, "growth": peak - baseline}


def verify_binary(native: Path, receipt: dict[str, Any]) -> None:
    require(digest(native) == receipt["sha256"], "Native hash mismatch")
    require(isinstance(receipt["closure"], dict) and receipt["closure"], "Missing staged dependency closure")
    for name, sha in receipt["closure"].items():
        require(
            Path(name).name == name and digest(native.parent / name) == sha,
            f"Native dependency hash mismatch for {name}",
        )


def verify_worker_ort(modules: list[str], staged: Path, expected_sha: str) -> dict[str, str] | None:
    mapping = None
    for module in modules:
        path = Path(module)
        name = path.name.lower()
        if name != "onnxruntime.dll" and not re.fullmatch(r"libonnxruntime\.so(?:\.[0-9]+)*", name):
            continue
        require(path == staged, f"Worker mapped unexpected core ORT path: {path} (expected {staged})")
        actual_sha = digest(path)
        require(actual_sha == expected_sha, f"Worker core ORT digest mismatch: {path}")
        mapping = {"path": str(path), "sha256": actual_sha}
    return mapping


def verify_cases(fixtures: dict[str, Any], rows: list[dict[str, Any]]) -> None:
    expected = {(name, mode, repeat) for name in fixtures for mode in MODES for repeat in range(TRIALS)}
    actual = {(row["fixture"], row["mode"], row["repeat"]) for row in rows}
    require(len(rows) == len(expected) == 27 and actual == expected, "Incomplete or duplicate case matrix")


def evaluate(
    fixtures: dict[str, Any],
    results: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], list[str]]:
    failures, summary = [], []
    for name, fixture in fixtures.items():
        rows = [r for r in results if r["fixture"] == name]
        if len({(r["sha256"], r["mime"], r["length"]) for r in rows}) != 1 or not rows[0]["mime"]:
            failures.append(f"{name}: content or MIME parity failure")
        path = sorted((r for r in rows if r["mode"] == "path"), key=lambda r: r["repeat"])
        for mode in MODES[1:]:
            byte = sorted((r for r in rows if r["mode"] == mode), key=lambda r: r["repeat"])
            memory = [b["growth"] - p["growth"] for p, b in zip(path, byte, strict=True)]
            managed = [b["managed"] - p["managed"] for p, b in zip(path, byte, strict=True)]
            factory_copies = int(mode == "factory")
            memory_limit = (1.5 + factory_copies) * fixture["bytes"] + 16 * MIB
            managed_limit = (0.05 + factory_copies) * fixture["bytes"] + 8 * MIB
            entry = {
                "fixture": name,
                "mode": mode,
                "private_differences": memory,
                "managed_differences": managed,
                "private_median": statistics.median(memory),
                "private_limit": memory_limit,
                "managed_limit": managed_limit,
            }
            summary.append(entry)
            if entry["private_median"] > memory_limit or max(managed) > managed_limit:
                failures.append(f"{name}/{mode}: memory budget exceeded")
    return summary, failures


def run_cases(
    command: list[str],
    fixtures: dict[str, Any],
    args: argparse.Namespace,
    env: dict[str, str],
    ort: tuple[Path, str],
) -> list[dict[str, Any]]:
    results = []
    loaded = set()
    cases = (
        (name, repeat, mode)
        for name in fixtures
        for repeat in range(TRIALS)
        for mode in MODES[repeat:] + MODES[:repeat]
    )
    with (
        (args.out / "cases.jsonl").open("w", encoding="utf-8") as output,
        (args.out / "loaded-modules.jsonl").open("w", encoding="utf-8") as inventory,
    ):
        for name, repeat, mode in cases:
            row = trial(
                command,
                args.out / "fixtures" / name,
                mode,
                args.out / f"{name}-{repeat}-{mode}.stderr",
                env=env,
                interval=args.interval_ms / 1000,
                timeout=args.timeout,
                runtime=args.runtime,
            )
            row["ort_mapping"] = verify_worker_ort(row["modules"], *ort)
            for module in sorted(set(row["modules"]) - loaded):
                path = Path(module)
                require(path.is_absolute() and path.is_file(), f"Unresolved loaded module {module}")
                inventory.write(json.dumps({"path": module, "sha256": digest(path)}) + "\n")
                loaded.add(module)
            inventory.flush()
            row["repeat"] = repeat
            results.append(row)
            output.write(json.dumps(row) + "\n")
            output.flush()
    return results


def worker_env(repo: Path, staged_ort: Path, system: str) -> dict[str, str]:
    env = os.environ.copy()
    var = "PATH" if system == "Windows" else "LD_LIBRARY_PATH"
    excluded = {(repo / "crates/xberg-node").resolve()}
    if location := env.get("ORT_LIB_LOCATION"):
        lib = Path(location).resolve()
        excluded.update((lib, lib.with_name("bin")))
    search = (part for part in env.get(var, "").split(os.pathsep) if part)
    env[var] = os.pathsep.join(
        [str(staged_ort.parent), *(part for part in search if Path(part).resolve() not in excluded)]
    )
    env["ORT_DYLIB_PATH"] = str(staged_ort)
    return env


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ("repo", "native", "provenance", "out"):
        parser.add_argument(f"--{arg}", required=True, type=Path)
    parser.add_argument("--dotnet", default="dotnet")
    parser.add_argument("--runtime", default="10.0.14")
    parser.add_argument("--interval-ms", type=float, default=5)
    parser.add_argument("--timeout", type=int, default=180)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    system = platform.system()
    target = {"Windows": "x86_64-pc-windows-msvc", "Linux": "x86_64-unknown-linux-gnu"}.get(system)
    require(target and args.interval_ms > 0 and args.timeout > 0, "Requires Windows/Linux and positive timing")
    args.out = args.out.resolve()
    repo = args.repo.resolve()
    require(not args.out.is_relative_to(repo), "Evidence output must be outside the checkout")
    native = args.native.resolve()
    receipt = json.loads(args.provenance.read_text(encoding="utf-8-sig"))
    for key in ("revision", "target", "build", "origin", "sha256", "closure"):
        require(receipt.get(key) is not None, f"Missing provenance {key}")
    revision = receipt["revision"]
    require(re.fullmatch(r"[a-f0-9]{40}", revision), "Full Git SHA required")
    require(
        receipt["target"] == target and receipt["build"] and receipt["origin"],
        "Native target, build invocation and origin required",
    )
    require(
        native.name == ("xberg_ffi.dll" if system == "Windows" else "libxberg_ffi.so"), "Unexpected native FFI filename"
    )
    verify_binary(native, receipt)

    args.out.mkdir(parents=True, exist_ok=False)
    code = subprocess.check_output(["git", "-C", str(repo), "archive", revision, SOURCE])
    with tarfile.open(fileobj=io.BytesIO(code)) as archive:
        archive.extractall(args.out / "source", filter="data")
    source = args.out / "source" / SOURCE
    require((source / "Xberg" / "XbergConverter.cs").is_file(), "Generated managed source missing")
    builddir = args.out / "managed"
    builddir.mkdir()
    for name in ("Probe.csproj", "Program.cs"):
        shutil.copy2(HERE / name, builddir)
    dotnet = str(Path(shutil.which(args.dotnet) or args.dotnet).resolve())
    with (args.out / "dotnet-info.txt").open("w") as info:
        subprocess.run([dotnet, "--info"], cwd=repo, stdout=info, check=True)
    with (args.out / "managed-build.log").open("w") as log:
        subprocess.run(
            [dotnet, "build", str(builddir / "Probe.csproj"), "-c", "Release", f"-p:ManagedSource={source}"],
            cwd=repo,
            stdout=log,
            stderr=subprocess.STDOUT,
            check=True,
        )
    worker = builddir / "bin" / "Release" / "net10.0" / "Probe.dll"
    fixtures = generate(args.out / "fixtures")
    require(fixtures == json.loads((HERE / "fixtures.json").read_text()), "Fixture bytes changed")
    pdf(args.out / "fixtures" / "warm.pdf", 0)
    docx(args.out / "fixtures" / "warm.docx", 0)
    ort_name = ORT_CORE[system]
    staged_ort = native.parent / ort_name
    require(ort_name in receipt["closure"] and not staged_ort.is_symlink(), "Missing staged core ORT library")
    ls_env = worker_env(repo, staged_ort, system)
    (args.out / "run.json").write_text(
        json.dumps(
            {
                "platform": platform.platform(),
                "native": str(native),
                "provenance": receipt,
                "managed_source_archive_sha256": hashlib.sha256(code).hexdigest(),
                "managed_worker_sha256": digest(worker),
                "fixtures": fixtures,
                "runtime": args.runtime,
                "trials_per_mode": TRIALS,
                "interval_ms": args.interval_ms,
                "github_run_id": os.environ.get("GITHUB_RUN_ID"),
                "github_run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
            },
            indent=2,
        )
    )
    command = [dotnet, "--fx-version", args.runtime, str(worker), str(native)]
    results = run_cases(command, fixtures, args, ls_env, (staged_ort, receipt["closure"][ort_name]))
    verify_cases(fixtures, results)
    summary, failures = evaluate(fixtures, results)
    report = json.dumps({"cases": len(results), "comparisons": summary, "failures": failures}, indent=2)
    (args.out / "summary.json").write_text(report + "\n")
    print(report)
    require(not failures, "Acceptance checks failed")


if __name__ == "__main__":
    main()
