"""Custom build hook for the Kreuzberg Python package.

It ensures that the sdist includes all necessary workspace crates.
"""

from __future__ import annotations

import tarfile
import tempfile
from pathlib import Path
from typing import Any

try:
    import maturin
except ImportError as exc:  # pragma: no cover - build-time dependency check
    raise ImportError(
        "The maturin build backend is required to package kreuzberg. "
        "Install it via `pip install maturin` and re-run the build."
    ) from exc


def ensure_stub_file() -> None:
    """Ensure _internal_bindings.pyi is present in the package for IDE type-checking.

    The .pyi stub file provides type hints for the compiled _internal_bindings module.
    This is critical for IDE type completion and mypy type checking to work properly.
    """
    package_dir = Path(__file__).resolve().parent / "kreuzberg"
    pyi_file = package_dir / "_internal_bindings.pyi"

    if not pyi_file.exists():
        pyi_file.write_text(
            "from typing import Any, Awaitable, Literal, Protocol, TypedDict\n"
            "from collections.abc import Callable\n\n"
            "class ExtractionResult(TypedDict): ...\n"
            "class ExtractionConfig: ...\n"
            "class OcrConfig: ...\n"
        )


def fix_sdist_workspace_members(sdist_path: str) -> None:
    """Fix the workspace members in the sdist's Cargo.toml.

    When maturin builds an sdist, it includes all path dependencies. However, we need
    to ensure the workspace's Cargo.toml correctly lists all members so builds succeed.
    """
    workspace_root = Path(__file__).resolve().parents[2]

    # Read the root Cargo.toml to get the correct members list
    root_cargo = workspace_root / "Cargo.toml"
    if not root_cargo.exists():
        return

    root_content = root_cargo.read_text()

    # Extract just the workspace section we need
    workspace_members_section = """[workspace]
members = [
    "crates/kreuzberg",
    "crates/kreuzberg-py",
    "crates/kreuzberg-ffi",
    "crates/kreuzberg-tesseract",
]
exclude = ["test_apps/rust"]
resolver = "2"

[patch.crates-io]
kreuzberg = { path = "crates/kreuzberg" }
kreuzberg-tesseract = { path = "crates/kreuzberg-tesseract" }"""

    # Extract the rest of the workspace.package and other sections
    lines = root_content.split("\n")
    workspace_pkg_idx = -1
    for i, line in enumerate(lines):
        if line.startswith("[workspace.package]"):
            workspace_pkg_idx = i
            break

    if workspace_pkg_idx == -1:
        return

    rest_of_cargo = "\n".join(lines[workspace_pkg_idx:])
    new_cargo_content = workspace_members_section + "\n" + rest_of_cargo

    # Now update the Cargo.toml in the sdist
    sdist = Path(sdist_path)
    if not sdist.exists():
        return

    try:
        # Create a temporary directory to work in
        with tempfile.TemporaryDirectory() as tmpdir:
            tmpdir_path = Path(tmpdir)

            # Extract the sdist
            with tarfile.open(sdist, "r:gz") as tar:
                tar.extractall(tmpdir_path, filter="data")

            # Find the extracted directory (should be kreuzberg-VERSION)
            extracted_dirs = list(tmpdir_path.iterdir())
            if not extracted_dirs:
                return

            extracted_dir = extracted_dirs[0]
            cargo_toml = extracted_dir / "Cargo.toml"

            if cargo_toml.exists():
                cargo_toml.write_text(new_cargo_content)

            # Remove the old tarball and create a new one
            sdist.unlink()
            with tarfile.open(sdist, "w:gz") as tar:
                tar.add(extracted_dir, arcname=extracted_dir.name)
    except Exception:
        # If anything goes wrong, just let it pass - the sdist is still valid
        pass


def build_wheel(
    wheel_directory: str,
    config_settings: dict[str, Any] | None = None,
    metadata_directory: str | None = None,
) -> str:
    """Build a wheel, ensuring stub files are present."""
    ensure_stub_file()

    return maturin.build_wheel(wheel_directory, config_settings, metadata_directory)  # type: ignore


def build_sdist(
    sdist_directory: str,
    config_settings: dict[str, Any] | None = None,
) -> str:
    """Build an sdist, ensuring stub files are present and workspace is configured correctly."""
    ensure_stub_file()

    # Build the sdist with maturin
    result: str = maturin.build_sdist(sdist_directory, config_settings)

    # Fix the workspace members in the resulting sdist
    # result is just the filename (e.g., "kreuzberg-4.0.0.tar.gz")
    sdist_path = Path(sdist_directory) / result
    fix_sdist_workspace_members(str(sdist_path))

    return result


def build_editable(
    wheel_directory: str,
    config_settings: dict[str, Any] | None = None,
    metadata_directory: str | None = None,
) -> str:
    """Build an editable wheel, ensuring stub files are present."""
    ensure_stub_file()

    return maturin.build_editable(wheel_directory, config_settings, metadata_directory)  # type: ignore
