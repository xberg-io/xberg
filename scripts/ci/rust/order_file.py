#!/usr/bin/env python3
"""Generate, and verify the effect of, the CLI linker order file (ADR-0020).

``generate`` builds ``xberg-cli`` with ``-Cprofile-generate``, runs the training corpus
(``order-file-corpus.txt``) and writes every function a run touched, grouped by the set of runs
that touched it, as a linker order file. ``crates/xberg-cli/build.rs`` passes the file to the
linker when ``XBERG_ORDER_FILE`` names it. The stages are also exposed one by one
(``build-instrumented``, ``train``, ``emit``) so a failed stage can be rerun alone.

Symbol names only match when the instrumented build and the release build share every input of
cargo's ``-C metadata`` hash: the same ``--release`` profile including any
``CARGO_PROFILE_RELEASE_*`` override, the same features, the same ``--target`` (or the lack of
one) and the same toolchain. ``RUSTFLAGS`` is excluded from that hash, which is what lets the
instrumentation flag ride on it.

``verify-map`` reads the linker map ``XBERG_LINK_MAP`` makes the release link write and fails
unless the order file matched the binary's symbols and the linker laid them out in file order.
"""

from __future__ import annotations

import argparse
import itertools
import os
import re
import shlex
import shutil
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_CORPUS = Path(__file__).with_name("order-file-corpus.txt")
COMMON_RUN_ARGS = (
    "--no-config-discovery",
    "--format",
    "json",
    "--config-json",
    '{"use_cache":false}',
    "--max-threads",
    "4",
)
RUN_TIMEOUT_SECONDS = 900
PAGE_BYTES = 16384


@dataclass(frozen=True)
class TrainingRun:
    """One CLI extraction of the training corpus."""

    path: str
    content_format: str
    extra_args: tuple[str, ...]

    def label(self) -> str:
        return f"{self.path} ({self.content_format}{' ' + ' '.join(self.extra_args) if self.extra_args else ''})"


def read_corpus(corpus: Path) -> list[TrainingRun]:
    runs = []
    for number, line in enumerate(corpus.read_text().splitlines(), 1):
        fields = shlex.split(line, comments=True)
        if not fields:
            continue
        if len(fields) < 2:
            raise ValueError(f"{corpus}:{number}: expected <path> <content format> [args...]")
        runs.append(TrainingRun(fields[0], fields[1], tuple(fields[2:])))
    if not runs:
        raise ValueError(f"{corpus} lists no training runs")
    return runs


def rustc_host() -> str:
    output = subprocess.run(["rustc", "-vV"], check=True, capture_output=True, text=True).stdout
    for line in output.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ").strip()
    raise RuntimeError("rustc -vV printed no host line")


def find_llvm_profdata(explicit: str | None) -> str:
    if explicit:
        return explicit
    if os.environ.get("LLVM_PROFDATA"):
        return os.environ["LLVM_PROFDATA"]
    sysroot = subprocess.run(["rustc", "--print", "sysroot"], check=True, capture_output=True, text=True).stdout.strip()
    tool = Path(sysroot) / "lib" / "rustlib" / rustc_host() / "bin" / "llvm-profdata"
    if not tool.is_file():
        raise RuntimeError(
            f"{tool} is missing: the llvm-tools component of the pinned toolchain provides it "
            "(rust-toolchain.toml), and only its LLVM reads this rustc's .profraw format"
        )
    return str(tool)


def instrumented_env(profile_dir: Path) -> dict[str, str]:
    env = dict(os.environ)
    flag = f"-Cprofile-generate={profile_dir}"
    if "CARGO_ENCODED_RUSTFLAGS" in env:
        encoded = env["CARGO_ENCODED_RUSTFLAGS"]
        env["CARGO_ENCODED_RUSTFLAGS"] = f"{encoded}\x1f{flag}" if encoded else flag
    else:
        env["RUSTFLAGS"] = f"{env.get('RUSTFLAGS', '')} {flag}".strip()
    for name in ("XBERG_ORDER_FILE", "XBERG_LINK_MAP"):
        env.pop(name, None)
    return env


def build_instrumented(target: str | None, profile_dir: Path, cargo_args: list[str]) -> Path:
    command = ["cargo", "build", "--locked", "--release", "--package", "xberg-cli"]
    if target:
        command += ["--target", target]
    command += cargo_args
    env = instrumented_env(profile_dir)
    print("+", shlex.join(command), f"(RUSTFLAGS={env.get('RUSTFLAGS', '')!r})", flush=True)
    subprocess.run(command, check=True, cwd=REPO_ROOT, env=env)
    target_dir = Path(env.get("CARGO_TARGET_DIR", REPO_ROOT / "target"))
    if not target_dir.is_absolute():
        target_dir = REPO_ROOT / target_dir
    binary = target_dir / (target or "") / "release" / "xberg"
    if not binary.is_file():
        raise RuntimeError(f"instrumented build produced no {binary}")
    return binary


def train(binary: Path, runs: list[TrainingRun], runs_dir: Path) -> None:
    if runs_dir.exists():
        shutil.rmtree(runs_dir)
    for index, run in enumerate(runs):
        source = REPO_ROOT / run.path
        if not source.is_file():
            raise RuntimeError(f"training input {run.path} is missing")
        run_dir = runs_dir / f"{index:03d}"
        run_dir.mkdir(parents=True)
        (run_dir / "label.txt").write_text(run.label() + "\n")
        command = [
            str(binary),
            "extract",
            str(source),
            *COMMON_RUN_ARGS,
            "--content-format",
            run.content_format,
            *run.extra_args,
        ]
        env = dict(os.environ, LLVM_PROFILE_FILE=str(run_dir / "%p-%m.profraw"))
        result = subprocess.run(command, check=False, env=env, stdout=subprocess.DEVNULL, timeout=RUN_TIMEOUT_SECONDS)
        if result.returncode != 0:
            raise RuntimeError(f"training run {run.label()} exited {result.returncode}")
        raws = [raw for raw in run_dir.glob("*.profraw") if raw.stat().st_size > 0]
        if not raws:
            raise RuntimeError(f"training run {run.label()} wrote no profile: is {binary} instrumented?")
        print(f"trained {run.label()}: {len(raws)} profile(s)", flush=True)


def profile_symbol(profile_name: str) -> str:
    """Return the linker symbol of a PGO function name.

    Local functions carry ``<source or codegen unit>;`` (``:`` in older LLVM) ahead of the
    symbol. Rust symbols, legacy or v0, never contain either separator.
    """
    return re.split(r"[;:]", profile_name)[-1]


def parse_profdata_show(text: str) -> dict[str, int]:
    """Return ``{symbol: max block count}`` for every function with a non-zero counter."""
    touched: dict[str, int] = {}
    name: str | None = None
    for line in text.splitlines():
        if line.startswith("  ") and not line.startswith("   ") and line.endswith(":"):
            name = line.strip()[:-1]
            continue
        stripped = line.strip()
        if name is not None and stripped.startswith("Block counts: ["):
            values = [int(value) for value in stripped.removeprefix("Block counts: [").rstrip("]").split(",") if value]
            peak = max(values, default=0)
            if peak > 0:
                symbol = profile_symbol(name)
                touched[symbol] = max(touched.get(symbol, 0), peak)
            name = None
    return touched


def order_symbols(per_run: list[dict[str, int]]) -> list[str]:
    """Order touched functions so each run's working set lands on as few pages as possible.

    Functions are grouped by the exact set of runs that touched them. Groups shared by more runs
    come first (start-up and the common PDF path), groups of equal reach in corpus order, and
    the hottest function first inside a group.
    """
    runs_of: dict[str, list[int]] = defaultdict(list)
    weight: dict[str, int] = defaultdict(int)
    for index, touched in enumerate(per_run):
        for symbol, count in touched.items():
            runs_of[symbol].append(index)
            weight[symbol] += count
    return sorted(runs_of, key=lambda symbol: (-len(runs_of[symbol]), runs_of[symbol], -weight[symbol], symbol))


def emit(runs_dir: Path, out: Path, mach_o: bool, llvm_profdata: str) -> list[str]:
    run_dirs = sorted(path for path in runs_dir.iterdir() if path.is_dir())
    if not run_dirs:
        raise RuntimeError(f"{runs_dir} holds no training runs")
    per_run = []
    for run_dir in run_dirs:
        raws = sorted(str(raw) for raw in run_dir.glob("*.profraw"))
        merged = run_dir / "merged.profdata"
        subprocess.run([llvm_profdata, "merge", "--output", str(merged), *raws], check=True)
        shown = subprocess.run(
            [llvm_profdata, "show", "--all-functions", "--counts", str(merged)],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
        touched = parse_profdata_show(shown)
        if not touched:
            raise RuntimeError(f"{run_dir}: the profile records no executed function")
        per_run.append(touched)
    symbols = order_symbols(per_run)
    prefix = "_" if mach_o else ""
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("".join(f"{prefix}{symbol}\n" for symbol in symbols))
    shared = sum(1 for symbol in symbols if all(symbol in touched for touched in per_run))
    print(f"wrote {out}: {len(symbols)} functions from {len(per_run)} runs, {shared} touched by every run")
    return symbols


def parse_ld64_map(text: str) -> tuple[dict[str, int], int]:
    """Return ``({symbol: address}, __text size)`` for ``__TEXT,__text`` from an ld64 map."""
    section = re.search(r"(?m)^0x([0-9A-Fa-f]+)\s+0x([0-9A-Fa-f]+)\s+__TEXT\s+__text\s*$", text)
    if section is None:
        raise ValueError("the ld64 map has no __TEXT,__text section")
    start, size = int(section.group(1), 16), int(section.group(2), 16)
    symbols: dict[str, int] = {}
    in_symbols = False
    for line in text.splitlines():
        if line.startswith("# Symbols:"):
            in_symbols = True
            continue
        if line.startswith("# Dead Stripped Symbols:"):
            break
        if not in_symbols or not line.startswith("0x"):
            continue
        match = re.match(r"0x([0-9A-Fa-f]+)\s+0x[0-9A-Fa-f]+\s+\[\s*\d+\]\s+(\S+)", line)
        if match:
            address = int(match.group(1), 16)
            if start <= address < start + size:
                symbols.setdefault(match.group(2), address)
    return symbols, size


LLD_ROW = re.compile(r"^\s*([0-9a-f]+)\s+[0-9a-f]+\s+([0-9a-f]+)\s+\d+ (\s*)(\S.*)$")
LLD_SYMBOL_INDENT = 16


def parse_lld_map(text: str) -> tuple[dict[str, int], int]:
    """Return ``({symbol: address}, .text size)`` for the ``.text`` output section of an lld map."""
    symbols: dict[str, int] = {}
    size = 0
    in_text = False
    for line in text.splitlines():
        match = LLD_ROW.match(line)
        if match is None:
            continue
        indent, name = len(match.group(3)), match.group(4)
        if indent == 0:
            in_text = name == ".text"
            if in_text:
                size = int(match.group(2), 16)
        elif in_text and indent == LLD_SYMBOL_INDENT:
            symbols.setdefault(name, int(match.group(1), 16))
    if size == 0:
        raise ValueError("the lld map has no .text output section")
    return symbols, size


SYSROOT_CRATES = frozenset(
    {
        "addr2line",
        "adler2",
        "alloc",
        "compiler_builtins",
        "core",
        "gimli",
        "miniz_oxide",
        "object",
        "panic_abort",
        "panic_unwind",
        "proc_macro",
        "rustc_demangle",
        "std",
        "std_detect",
        "unwind",
    }
)
V0_CRATE_ROOT = re.compile(r"Cs[0-9A-Za-z]*_(\d+)_?")
LEGACY_FIRST_COMPONENT = re.compile(r"^_+ZN(\d+)")


def symbol_crates(symbol: str) -> set[str]:
    """Return the crate names a Rust symbol mentions (v0: every crate root; legacy: the first path)."""
    legacy = LEGACY_FIRST_COMPONENT.match(symbol)
    if legacy:
        start = legacy.end()
        return {symbol[start : start + int(legacy.group(1))]}
    crates = set()
    for root in V0_CRATE_ROOT.finditer(symbol):
        start = root.end()
        crates.add(symbol[start : start + int(root.group(1))])
    return crates


def names_a_workspace_crate(symbol: str) -> bool:
    """Whether the symbol's name hashes depend on this build's crate metadata.

    Precompiled standard library symbols keep their names whatever cargo hashes the build with,
    so they would match even when the instrumented and release builds disagree.
    """
    return bool(symbol_crates(symbol) - SYSROOT_CRATES)


@dataclass(frozen=True)
class MapReport:
    entries: int
    applicable: int
    inversions: int
    span_bytes: int
    text_bytes: int
    workspace_entries: int
    workspace_applicable: int

    @property
    def applicable_ratio(self) -> float:
        return self.applicable / self.entries if self.entries else 0.0

    @property
    def workspace_ratio(self) -> float:
        return self.workspace_applicable / self.workspace_entries if self.workspace_entries else 0.0

    @property
    def ordered_ratio(self) -> float:
        return 1.0 - self.inversions / (self.applicable - 1) if self.applicable > 1 else 1.0


def check_map(order: list[str], map_text: str) -> MapReport:
    if "# Symbols:" in map_text:
        symbols, text_bytes = parse_ld64_map(map_text)
    else:
        symbols, text_bytes = parse_lld_map(map_text)
    if not symbols:
        raise ValueError("the link map lists no text symbols")
    unique = list(dict.fromkeys(order))
    addresses = [symbols[entry] for entry in unique if entry in symbols]
    workspace = [entry for entry in unique if names_a_workspace_crate(entry)]
    inversions = sum(1 for before, after in itertools.pairwise(addresses) if after < before)
    span = max(addresses) - min(addresses) if addresses else 0
    return MapReport(
        entries=len(unique),
        applicable=len(addresses),
        inversions=inversions,
        span_bytes=span,
        text_bytes=text_bytes,
        workspace_entries=len(workspace),
        workspace_applicable=sum(1 for entry in workspace if entry in symbols),
    )


def verify_map(order_file: Path, link_map: Path, min_applicable: float, min_ordered: float) -> int:
    order = [line.strip() for line in order_file.read_text().splitlines() if line.strip()]
    if not order:
        print(f"::error::{order_file} is empty")
        return 1
    report = check_map(order, link_map.read_text(errors="replace"))
    print(
        f"order file entries: {report.entries}\n"
        f"present in the binary: {report.applicable} ({report.applicable_ratio:.1%})\n"
        f"entries naming a non-sysroot crate present: {report.workspace_applicable} of "
        f"{report.workspace_entries} ({report.workspace_ratio:.1%})\n"
        f"laid out in order: {report.ordered_ratio:.2%} ({report.inversions} inversions)\n"
        f"span of ordered functions: {report.span_bytes / 1048576:.2f} MiB "
        f"(~{report.span_bytes // PAGE_BYTES + 1} 16 KiB pages) of {report.text_bytes / 1048576:.2f} MiB text"
    )
    failed = False
    if report.workspace_ratio < min_applicable:
        print(
            f"::error::only {report.workspace_ratio:.1%} of the order file's non-sysroot functions exist in the "
            f"binary (minimum {min_applicable:.0%}): the instrumented and release builds disagree on symbol names"
        )
        failed = True
    if report.ordered_ratio < min_ordered:
        print(f"::error::the linker did not apply the order file ({report.ordered_ratio:.2%} in order)")
        failed = True
    return 1 if failed else 0


def target_is_mach_o(target: str | None) -> bool:
    return "apple" in (target or rustc_host())


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)

    def add_common(command: argparse.ArgumentParser) -> None:
        command.add_argument("--work-dir", type=Path, required=True, help="scratch directory for profiles")
        command.add_argument("--target", default=None, help="the --target the release build passes, if any")

    generate = commands.add_parser("generate", help="build instrumented, train and emit")
    add_common(generate)
    generate.add_argument("--out", type=Path, required=True)
    generate.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    generate.add_argument("--llvm-profdata")
    generate.add_argument("cargo_args", nargs=argparse.REMAINDER, help="-- then the release build's cargo arguments")

    build = commands.add_parser("build-instrumented", help="only build the instrumented CLI")
    add_common(build)
    build.add_argument("cargo_args", nargs=argparse.REMAINDER)

    train_command = commands.add_parser("train", help="only run the corpus through an instrumented CLI")
    add_common(train_command)
    train_command.add_argument("--binary", type=Path, required=True)
    train_command.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)

    emit_command = commands.add_parser("emit", help="only turn trained profiles into an order file")
    add_common(emit_command)
    emit_command.add_argument("--out", type=Path, required=True)
    emit_command.add_argument("--llvm-profdata")

    verify = commands.add_parser("verify-map", help="prove the release link applied the order file")
    verify.add_argument("--order-file", type=Path, required=True)
    verify.add_argument("--map", type=Path, required=True)
    verify.add_argument("--min-applicable", type=float, default=0.10)
    verify.add_argument("--min-ordered", type=float, default=0.99)

    args = parser.parse_args(argv)
    if args.command == "verify-map":
        return verify_map(args.order_file, args.map, args.min_applicable, args.min_ordered)

    work_dir = args.work_dir.resolve()
    runs_dir = work_dir / "runs"
    cargo_args = [arg for arg in getattr(args, "cargo_args", []) if arg != "--"]
    if args.command in ("generate", "build-instrumented"):
        binary = build_instrumented(args.target, work_dir / "build-profiles", cargo_args)
        print(f"instrumented binary: {binary}")
        if args.command == "build-instrumented":
            return 0
    else:
        binary = getattr(args, "binary", None)
    if args.command in ("generate", "train"):
        train(binary, read_corpus(args.corpus), runs_dir)
    if args.command in ("generate", "emit"):
        emit(runs_dir, args.out.resolve(), target_is_mach_o(args.target), find_llvm_profdata(args.llvm_profdata))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
