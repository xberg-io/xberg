"""Exercise the replay gates without running a native worker."""

import json
import os
import tempfile
from argparse import Namespace
from pathlib import Path
from unittest.mock import patch

import pytest
from ci import core_ort_source
from replay import MODES, TRIALS, evaluate, measure, run_cases, verify_worker_ort, worker_env

from fixtures import MIB, digest


def test_selects_core_library_instead_of_provider_or_setup_alias() -> None:
    with tempfile.TemporaryDirectory() as scratch:
        lib = Path(scratch) / "lib"
        bin_dir = lib.with_name("bin")
        lib.mkdir()
        bin_dir.mkdir()
        provider = lib / "libonnxruntime_providers_shared.so"
        provider.touch()
        with pytest.raises(RuntimeError, match="Core ORT library"):
            core_ort_source(lib, "Linux")
        core = lib / "libonnxruntime.so.1"
        core.touch()
        assert core_ort_source(lib, "Linux") == core
        (lib / "onnxruntime_providers_shared.dll").touch()
        windows_core = bin_dir / "onnxruntime.dll"
        windows_core.touch()
        assert core_ort_source(lib, "Windows") == windows_core


def test_checks_each_workers_mapped_core_path_and_digest() -> None:
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        alias = root / "alias"
        alias.mkdir()
        for filename in ("libonnxruntime.so.1", "onnxruntime.dll"):
            staged = root / filename
            staged.write_bytes(b"core")
            actual_alias = alias / filename
            actual_alias.write_bytes(staged.read_bytes())
            expected = digest(staged)
            assert verify_worker_ort([str(staged)], staged, expected) == {"path": str(staged), "sha256": expected}
            assert verify_worker_ort([str(root / "onnxruntime_providers_shared.dll")], staged, expected) is None
            with pytest.raises(RuntimeError, match="unexpected core ORT path"):
                verify_worker_ort([str(staged), str(actual_alias)], staged, expected)
            staged.write_bytes(b"changed")
            with pytest.raises(RuntimeError, match="core ORT digest mismatch"):
                verify_worker_ort([str(staged)], staged, expected)


def replay_module_inventory(tmp_path: Path, modules: list[str], system: str = "Linux") -> list[dict]:
    staged = tmp_path / "libonnxruntime.so.1"
    staged.write_bytes(b"core")
    row = {"modules": [str(staged), *modules]}
    args = Namespace(out=tmp_path, interval_ms=5, timeout=1, runtime="10.0.14")
    with (
        patch("replay.platform.system", return_value=system),
        patch("replay.trial", return_value=row),
        patch("replay.TRIALS", 1),
        patch("replay.MODES", ("path",)),
    ):
        return run_cases([], {"fixture.pdf": {}}, args, {}, (staged, digest(staged)))


@pytest.mark.parametrize("pseudo", ["/memfd:doublemapper (deleted)", "[vdso]"], ids=["jit", "vdso"])
def test_linux_pseudo_mapping_stays_in_raw_receipt_without_a_file_digest(tmp_path: Path, pseudo: str) -> None:
    native = tmp_path / "libxberg_ffi.so"
    native.write_bytes(b"ffi")
    [row] = replay_module_inventory(tmp_path, [pseudo, str(native)])
    assert pseudo in row["modules"]
    assert row["ort_mapping"] == {
        "path": str(tmp_path / "libonnxruntime.so.1"),
        "sha256": digest(tmp_path / "libonnxruntime.so.1"),
    }
    inventory = [json.loads(line) for line in (tmp_path / "loaded-modules.jsonl").read_text().splitlines()]
    assert {item["path"]: item["sha256"] for item in inventory} == {
        str(native): digest(native),
        row["ort_mapping"]["path"]: row["ort_mapping"]["sha256"],
    }


@pytest.mark.parametrize(
    ("module", "system"),
    [
        ("/memfd:other (deleted)", "Linux"),
        ("/memfd:doublemapper", "Linux"),
        ("/memfd:doublemapper (deleted)", "Windows"),
        ("[vdso]", "Windows"),
        ("[vvar]", "Linux"),
        ("[vdso] (deleted)", "Linux"),
        ("/missing/libxberg_ffi.so", "Linux"),
        ("/missing/libxberg_ffi.so (deleted)", "Linux"),
    ],
)
def test_unresolved_module_inventory_still_fails_closed(tmp_path: Path, module: str, system: str) -> None:
    with pytest.raises(RuntimeError, match="Unresolved loaded module") as error:
        replay_module_inventory(tmp_path, [module], system)
    assert str(error.value) == f"Unresolved loaded module {module}"


def test_removes_setup_library_dirs_from_worker_search() -> None:
    repo = Path.cwd()
    lib = repo / "ort/lib"
    setup_dest = repo / "crates/xberg-node"
    stage = repo / "stage/libonnxruntime.so.1"
    other = repo / "other"
    inherited = os.pathsep.join(map(str, (lib, lib.with_name("bin"), setup_dest, other)))
    with patch.dict(os.environ, {"ORT_LIB_LOCATION": str(lib), "LD_LIBRARY_PATH": inherited}):
        env = worker_env(repo, stage, "Linux")
    assert env["LD_LIBRARY_PATH"].split(os.pathsep) == [str(stage.parent), str(other)]
    assert env["ORT_DYLIB_PATH"] == str(stage)


def test_completion_only_spike_exceeds_private_memory_budget() -> None:
    baseline = 10 * MIB
    samples = [(0.0, baseline), (0.5, baseline), (1.1, baseline + 90 * MIB)]
    completed = measure({}, {}, samples, start=0.1, end=1.0)
    assert completed["growth"] == 90 * MIB
    rows = [
        {
            "fixture": "pdf-40.pdf",
            "mode": mode,
            "repeat": repeat,
            "sha256": "same",
            "mime": "application/pdf",
            "length": 31,
            "managed": 0,
            "growth": completed["growth"] if mode == "bytes" else 0,
        }
        for mode in MODES
        for repeat in range(TRIALS)
    ]
    fixtures = {"pdf-40.pdf": {"bytes": 40 * MIB}}
    assert evaluate(fixtures, rows)[1] == ["pdf-40.pdf/bytes: memory budget exceeded"]
    for row in rows:
        row["growth"] = 0
    assert evaluate(fixtures, rows)[1] == []


@pytest.mark.parametrize("periodic", [[], [(1.05, 10)]], ids=["none", "after-extraction"])
def test_missing_in_window_periodic_sample_is_rejected(periodic: list[tuple[float, int]]) -> None:
    with pytest.raises(RuntimeError, match="Missing in-window Linux extraction sample"):
        measure({}, {}, [(0.0, 10), *periodic, (1.1, 10)], start=0.1, end=1.0)


def test_missing_completion_sample_is_rejected() -> None:
    with pytest.raises(RuntimeError, match="Missing Linux completion sample"):
        measure({}, {}, [(0.0, 10), (0.5, 10)], start=0.1, end=1.0)
