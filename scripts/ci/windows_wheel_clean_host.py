"""Qualify a wheel inside disposable Server Core userspace without global VC runtime."""

from __future__ import annotations

import argparse
import asyncio
import base64
import csv
import ctypes
import hashlib
import io
import json
import os
import subprocess
import sys
import zipfile
from ctypes import wintypes
from pathlib import Path, PurePosixPath
from typing import TypeVar

import windows_wheel_closure as closure

T = TypeVar("T")
LOAD_LIBRARY_SEARCH_DEFAULT_DIRS = 0x1000
LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR = 0x0100
ERROR_MOD_NOT_FOUND = 126


def record_hash(data: bytes) -> str:
    return "sha256=" + base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()


def verify_record(archive: zipfile.ZipFile) -> None:
    names = archive.namelist()
    if len(names) != len(set(names)):
        raise ValueError("duplicate wheel members")
    records = [name for name in names if name.endswith(".dist-info/RECORD")]
    if len(records) != 1:
        raise ValueError("expected exactly one wheel RECORD")
    entries = list(csv.reader(io.StringIO(archive.read(records[0]).decode())))
    if any(len(row) != 3 for row in entries):
        raise ValueError("invalid RECORD row")
    indexed = {row[0]: row[1:] for row in entries}
    expected = {name for name in names if not name.endswith("/") and name != records[0]}
    if len(indexed) != len(entries) or set(indexed) - {records[0]} != expected:
        raise ValueError("RECORD inventory does not match wheel")
    for name in expected:
        data = archive.read(name)
        if indexed[name] != [record_hash(data), str(len(data))]:
            raise ValueError(f"RECORD hash mismatch: {name}")


def verify_notices(archive: zipfile.ZipFile, files: dict[str, tuple[str, bytes]]) -> None:
    notices = [name for name in archive.namelist() if name.endswith(".dist-info/licenses/MICROSOFT_VC_RUNTIME.json")]
    if len(notices) != 1:
        raise ValueError("expected exactly one Microsoft runtime notice")
    notice = json.loads(archive.read(notices[0]))
    if (
        notice.get("distribution_authority") != closure.REDIST_TERMS
        or notice.get("runtime_use_terms") != closure.RUNTIME_TERMS
    ):
        raise ValueError("runtime notice terms mismatch")
    expected = {
        key: data for key, (_, data) in files.items() if PurePosixPath(key).name.startswith(closure.MSVC_PREFIXES)
    }
    rows = notice.get("files", [])
    inventory = {row["file"].lower(): row["sha256"] for row in rows}
    if not expected or len(inventory) != len(rows) or set(inventory) != set(expected):
        raise ValueError("runtime notice inventory mismatch")
    for name, data in expected.items():
        if inventory[name] != hashlib.sha256(data).hexdigest():
            raise ValueError(f"runtime notice hash mismatch: {name}")


def verify_wheel(wheel: Path) -> dict:
    with zipfile.ZipFile(wheel) as archive:
        closure.raw_zip_members(archive)
        verify_record(archive)
        files = closure.artifact_files(wheel)
        graph = closure.audit(files, "*.pyd")
        verify_notices(archive, files)
        metadata = [name for name in archive.namelist() if name.endswith(".dist-info/METADATA")]
        if len(metadata) != 1 or "\nRequires-Dist:" in archive.read(metadata[0]).decode():
            raise ValueError("qualification requires a self-contained wheel without Python dependencies")
    msvcp = sorted(name for name in files if PurePosixPath(name).name.startswith("msvcp"))
    if not msvcp:
        raise ValueError("wheel has no MSVCP dependency to qualify")
    ort = single(
        [data for name, (_, data) in files.items() if PurePosixPath(name).name == "onnxruntime.dll"],
        "wheel ONNX Runtime",
    )
    imported_msvcp = sorted(name for name in closure.PeImage(ort).imports() if name.startswith("msvcp"))
    if not imported_msvcp:
        raise ValueError("ONNX Runtime has no MSVCP import for the native negative control")
    return {
        "closure": graph,
        "msvcp": msvcp,
        "negative_dependency": imported_msvcp[0],
        "wheel_sha256": hashlib.sha256(wheel.read_bytes()).hexdigest(),
    }


def verify_modules(modules: list[Path], wheel_root: Path, python_root: Path) -> None:
    for path in modules:
        name = path.name.lower()
        if name.startswith(("msvcp", "concrt", "vcomp")) and not path.resolve().is_relative_to(wheel_root.resolve()):
            raise ValueError(f"runtime module outside wheel: {path}")
        if name.startswith("vcruntime") and not any(
            path.resolve().is_relative_to(root.resolve()) for root in (wheel_root, python_root)
        ):
            raise ValueError(f"VCRUNTIME module outside allowed app-local roots: {path}")


def loaded_modules() -> list[Path]:
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.GetCurrentProcess.restype = wintypes.HANDLE
    kernel.K32EnumProcessModules.argtypes = [
        wintypes.HANDLE,
        ctypes.POINTER(wintypes.HMODULE),
        wintypes.DWORD,
        ctypes.POINTER(wintypes.DWORD),
    ]
    kernel.K32GetModuleFileNameExW.argtypes = [wintypes.HANDLE, wintypes.HMODULE, wintypes.LPWSTR, wintypes.DWORD]
    modules = (wintypes.HMODULE * 4096)()
    needed = wintypes.DWORD()
    process = kernel.GetCurrentProcess()
    if not kernel.K32EnumProcessModules(process, modules, ctypes.sizeof(modules), ctypes.byref(needed)):
        raise ctypes.WinError(ctypes.get_last_error())
    if needed.value > ctypes.sizeof(modules):
        raise ValueError("loaded module enumeration truncated")
    result = []
    for module in modules[: needed.value // ctypes.sizeof(wintypes.HMODULE)]:
        buffer = ctypes.create_unicode_buffer(32768)
        length = kernel.K32GetModuleFileNameExW(process, module, buffer, len(buffer))
        if not length or length >= len(buffer) - 1:
            raise ValueError("loaded module path unavailable or truncated")
        result.append(Path(buffer.value))
    return result


def load_library_error(path: str, flags: int) -> int:
    # ~keep Keep the native status: ctypes.WinDLL can replace it with a FileNotFoundError.
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    load = kernel.LoadLibraryExW
    load.argtypes = [wintypes.LPCWSTR, wintypes.HANDLE, wintypes.DWORD]
    load.restype = wintypes.HMODULE
    if load(path, None, flags):
        return 0
    error = ctypes.get_last_error()
    if not error:
        raise ValueError(f"loader failed without a Windows error code: {path}")
    return error


def baseline(python_root: Path, msvcp: list[str]) -> dict:
    import winreg

    for view in (winreg.KEY_WOW64_64KEY, winreg.KEY_WOW64_32KEY):
        for arch in ("x64", "x86", "arm64"):
            key = rf"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\{arch}"
            try:
                with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, key, 0, winreg.KEY_READ | view):
                    raise ValueError(f"global VC runtime registry entry: {key}")
            except FileNotFoundError:
                pass
    names = sorted({PurePosixPath(name).name for name in msvcp})
    inbox_clr_files = []
    unexpected_runtime_paths = []
    for directory in (
        Path(os.environ["SYSTEMROOT"]) / "System32",
        Path(os.environ["SYSTEMROOT"]) / "SysWOW64",
        python_root,
    ):
        for path in directory.rglob("*.dll"):
            name = path.name.lower()
            if name.startswith(("msvcp140", "vcruntime140", "concrt140", "vcomp140")):
                if directory == python_root and name.startswith("vcruntime"):
                    continue
                # ~keep The pinned Server Core image ships these CLR sidecars; loader checks still apply.
                if (
                    directory != python_root
                    and path.parent == directory
                    and name in ("msvcp140_clr0400.dll", "vcruntime140_clr0400.dll")
                ):
                    inbox_clr_files.append(f"{directory.name}/{path.name}")
                    continue
                unexpected_runtime_paths.append(str(path))
    if unexpected_runtime_paths:
        paths = "\n".join(sorted(unexpected_runtime_paths))
        raise ValueError(f"baseline contains VC runtime files:\n{paths}")
    for name in names:
        error = load_library_error(name, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)
        if not error:
            raise ValueError(f"baseline unexpectedly loads {name}")
        if error != ERROR_MOD_NOT_FOUND:
            raise ValueError(f"baseline load failed for wrong reason: {name}: Windows error {error}")
    verify_modules(loaded_modules(), python_root / "absent-wheel", python_root)
    return {
        "registry_absent": True,
        "global_files_absent": True,
        "inbox_clr_files": sorted(inbox_clr_files),
        "msvcp_load_failures": names,
    }


def require_missing_dependency(outcome: subprocess.CompletedProcess[str], dependency: str) -> None:
    if outcome.returncode == 0:
        raise ValueError("missing-runtime negative control unexpectedly succeeded")
    if outcome.returncode != 23 or f"MISSING_DEPENDENCY:{dependency}:126" not in outcome.stdout:
        raise ValueError(
            f"negative control is not the required missing dependency failure: {outcome.stdout} {outcome.stderr}"
        )


def run_child(python: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(python), "-E", "-s", str(Path(__file__).resolve()), *arguments],
        capture_output=True,
        text=True,
        timeout=180,
        check=False,
    )


def native_probe(dependency: str, ort: Path, *, missing: bool) -> None:
    error = load_library_error(str(ort), LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR)
    if error:
        if missing and error == ERROR_MOD_NOT_FOUND:
            print(f"MISSING_DEPENDENCY:{dependency}:{error}")
            sys.exit(23)
        raise ctypes.WinError(error)
    if missing:
        raise ValueError("native load unexpectedly succeeded without runtime")


def positive(site: Path, python_root: Path, ort: Path, fixture: Path) -> dict:
    native_probe("", ort, missing=False)
    import smoke_test_python_wheel

    asyncio.run(smoke_test_python_wheel.run(str(fixture), "XBERG MUSL SMOKE 490"))
    modules = loaded_modules()
    verify_modules(modules, ort.parent, python_root)
    if not any(path.suffix.lower() == ".pyd" and path.resolve().is_relative_to(site.resolve()) for path in modules):
        raise ValueError("no wheel native module loaded")
    if not any(path.name.lower().startswith("msvcp") for path in modules):
        raise ValueError("no wheel MSVCP runtime actually loaded")
    if ort.resolve() not in [path.resolve() for path in modules]:
        raise ValueError("wheel ONNX Runtime was not loaded")
    return {
        "extractions": 1,
        "modules": [{"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()} for path in modules],
    }


def qualify(root: Path, case: str) -> None:
    wheel = single(list(root.glob("*.whl")), "wheel")
    result = verify_wheel(wheel)
    python_root = Path(sys.base_prefix)
    result["baseline"] = baseline(python_root, result["msvcp"])
    result["python_sha256"] = hashlib.sha256(Path(sys.executable).read_bytes()).hexdigest()
    venv = root / "venv"
    subprocess.run([sys.executable, "-I", "-m", "venv", str(venv)], check=True, timeout=120)
    python = venv / "Scripts/python.exe"
    subprocess.run(
        [str(python), "-I", "-m", "pip", "install", "--no-index", "--no-deps", str(wheel)], check=True, timeout=120
    )
    site = venv / "Lib/site-packages"
    ort = single(list(site.rglob("onnxruntime.dll")), "installed ONNX Runtime")
    dependency = result["negative_dependency"]
    if case == "missing-runtime":
        result.update(qualify_missing(python, site, ort, dependency))
    else:
        outcome = run_child(python, "positive", "--root", str(root), "--ort", str(ort))
        if outcome.returncode:
            raise ValueError(f"intact wheel failed: {outcome.stdout} {outcome.stderr}")
        result.update(json.loads((root / "positive.json").read_text()))
    result["case"] = case
    (root / "receipt.json").write_text(json.dumps(result, indent=2))


def qualify_missing(python: Path, site: Path, ort: Path, dependency: str) -> dict:
    target = single([path for path in site.rglob("*.dll") if path.name.lower() == dependency], "missing-runtime target")
    target.unlink()
    try:
        closure.audit(closure.artifact_files(site / "xberg"), "*.pyd")
    except ValueError as error:
        if dependency not in str(error).lower():
            raise ValueError("negative closure failed for wrong reason") from error
    else:
        raise ValueError("negative closure unexpectedly passed")
    healthy = subprocess.run(
        [str(python), "-I", "-c", "import json; print(json.dumps({'healthy': True}))"],
        capture_output=True,
        text=True,
        check=True,
        timeout=30,
    )
    if json.loads(healthy.stdout) != {"healthy": True}:
        raise ValueError("negative Python baseline unhealthy")
    require_missing_dependency(run_child(python, "missing", "--ort", str(ort), "--dependency", dependency), dependency)
    return {
        "expected_native_failures": 1,
        "expected_closure_failures": 1,
        "python_baseline_healthy": True,
        "removed_runtime": dependency,
        "extractions": 0,
    }


def single(values: list[T], label: str) -> T:
    if len(values) != 1:
        raise ValueError(f"expected exactly one {label}, found {len(values)}")
    return values[0]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["qualify", "positive", "missing"])
    parser.add_argument("--root", type=Path)
    parser.add_argument("--case", choices=["intact", "missing-runtime"])
    parser.add_argument("--ort", type=Path)
    parser.add_argument("--dependency")
    arguments = parser.parse_args()
    required = {"qualify": ("root", "case"), "positive": ("root", "ort"), "missing": ("ort", "dependency")}
    for name in required[arguments.mode]:
        if getattr(arguments, name) is None:
            parser.error(f"--{name} is required for {arguments.mode}")
    if os.name != "nt":
        raise ValueError("runtime qualification requires Windows")
    if arguments.mode == "qualify":
        qualify(arguments.root, arguments.case)
    elif arguments.mode == "missing":
        native_probe(arguments.dependency, arguments.ort, missing=True)
    else:
        root = arguments.root
        result = positive(
            Path(sys.prefix) / "Lib/site-packages", Path(sys.base_prefix), arguments.ort, root / "musl-python-smoke.pdf"
        )
        (root / "positive.json").write_text(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
