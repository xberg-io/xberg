"""Build and replay the FFI from the exact checkout selected by the dispatch."""

import json
import os
import platform
import shutil
import subprocess
import sys
from pathlib import Path

from replay import ORT_CORE

from fixtures import digest

TARGETS = {"Linux": "x86_64-unknown-linux-gnu", "Windows": "x86_64-pc-windows-msvc"}


def core_ort_source(lib_dir: Path, system: str) -> Path:
    name = ORT_CORE[system]
    candidates = (lib_dir / name, lib_dir.with_name("bin") / name) if system == "Windows" else (lib_dir / name,)
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise RuntimeError(f"Core ORT library {name} missing under {lib_dir.parent}")


def main() -> None:
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    if revision != os.environ["GITHUB_SHA"] or revision != os.environ["EXPECTED_SHA"]:
        raise RuntimeError("Checkout does not match the dispatched main revision")
    subprocess.run(["git", "diff", "--quiet", "HEAD", "--"], check=True)
    system = platform.system()
    target = os.environ["REPLAY_TARGET"]
    if target != TARGETS[system] or os.environ["RUNNER_ARCH"] != "X64":
        raise RuntimeError("Runner does not match the requested x64 target")
    scratch = Path(os.environ["RUNNER_TEMP"]) / "xberg-3152"
    evidence = scratch / "evidence"
    evidence.mkdir(parents=True)
    staged = scratch / "native"
    staged.mkdir()
    build = ["cargo", "build", "--locked", "-p", "xberg-ffi", "--release", "--target", target]
    ort = core_ort_source(Path(os.environ["ORT_LIB_LOCATION"]), system)
    staged_ort = staged / ORT_CORE[system]
    shutil.copy2(ort, staged_ort)
    env = {**os.environ, "CARGO_TARGET_DIR": str(scratch / "target"), "ORT_DYLIB_PATH": str(staged_ort)}
    receipt = {
        "revision": revision,
        "target": target,
        "origin": {
            key: os.environ[key]
            for key in (
                "GITHUB_SERVER_URL",
                "GITHUB_REPOSITORY",
                "GITHUB_WORKFLOW_REF",
                "GITHUB_WORKFLOW_SHA",
                "GITHUB_RUN_ID",
                "GITHUB_RUN_ATTEMPT",
                "GITHUB_JOB",
            )
        },
        "build": build,
        "ort_source": str(ort.resolve()),
        "closure": {staged_ort.name: digest(staged_ort)},
        "rustc": subprocess.check_output(["rustc", "-vV"], text=True),
        "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip(),
        "cargo_lock_sha256": digest(Path("Cargo.lock")),
        "runner_image": {key: os.environ.get(key) for key in ("ImageOS", "ImageVersion")},
        "build_overrides": {
            key: value
            for key, value in env.items()
            if key
            in (
                "RUSTFLAGS",
                "CARGO_ENCODED_RUSTFLAGS",
                "CARGO_INCREMENTAL",
                "CARGO_TARGET_DIR",
                "RUSTC_WRAPPER",
            )
            or key.startswith("CARGO_PROFILE_RELEASE_")
        },
    }

    receipt_path = evidence / "native-provenance.json"
    receipt_path.write_text(json.dumps(receipt, indent=2), encoding="utf-8")
    with (evidence / "native-build.log").open("w", encoding="utf-8") as log:
        subprocess.run(build, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    filename = "xberg_ffi.dll" if system == "Windows" else "libxberg_ffi.so"
    native = staged / filename
    shutil.copy2(scratch / "target" / target / "release" / filename, native)
    receipt["sha256"] = digest(native)
    receipt_path.write_text(json.dumps(receipt, indent=2), encoding="utf-8")

    replay(scratch / "run", native, receipt_path)


def replay(run: Path, native: Path, receipt_path: Path) -> None:
    try:
        subprocess.run(
            [
                sys.executable,
                "-B",
                str(Path(__file__).with_name("replay.py")),
                "--repo",
                str(Path.cwd()),
                "--native",
                str(native),
                "--provenance",
                str(receipt_path),
                "--out",
                str(run),
            ],
            check=True,
        )
    finally:
        for pattern in ("*.json", "*.jsonl", "*.log", "*.stderr", "dotnet-info.txt"):
            for path in run.glob(pattern):
                shutil.copy2(path, receipt_path.parent / path.name)


if __name__ == "__main__":
    main()
