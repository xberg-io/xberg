#!/usr/bin/env python3
"""
Vendor kreuzberg core crate into R package
Used by: ci-r.yaml - Vendor kreuzberg core crate step

This script:
1. Reads workspace.dependencies from root Cargo.toml
2. Copies core crates to packages/r/vendor/
3. Replaces workspace = true with explicit versions
4. Generates vendor/Cargo.toml with proper workspace setup
"""

import os
import sys
import shutil
import re
from pathlib import Path

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore


def get_repo_root() -> Path:
    """Get repository root directory."""
    repo_root_env = os.environ.get("REPO_ROOT")
    if repo_root_env:
        return Path(repo_root_env)

    script_dir = Path(__file__).parent.absolute()
    return (script_dir / ".." / ".." / "..").resolve()


def read_toml(path: Path) -> dict[str, object]:
    """Read TOML file."""
    with open(path, "rb") as f:
        return tomllib.load(f)


def get_workspace_deps(repo_root: Path) -> dict[str, object]:
    """Extract workspace.dependencies from root Cargo.toml."""
    cargo_toml_path = repo_root / "Cargo.toml"
    data = read_toml(cargo_toml_path)
    return data.get("workspace", {}).get("dependencies", {})


def get_workspace_version(repo_root: Path) -> str:
    """Extract version from workspace.package."""
    cargo_toml_path = repo_root / "Cargo.toml"
    data = read_toml(cargo_toml_path)
    return data.get("workspace", {}).get("package", {}).get("version", "4.0.0")


def format_dependency(name: str, dep_spec: object) -> str:
    """Format a dependency spec for Cargo.toml."""
    if isinstance(dep_spec, str):
        return f'{name} = "{dep_spec}"'
    elif isinstance(dep_spec, dict):
        version: str = dep_spec.get("version", "")
        package: str | None = dep_spec.get("package")
        features: list[str] = dep_spec.get("features", [])
        default_features: bool | None = dep_spec.get("default-features")

        parts: list[str] = []

        if package:
            parts.append(f'package = "{package}"')

        parts.append(f'version = "{version}"')

        if features:
            features_str = ', '.join(f'"{f}"' for f in features)
            parts.append(f'features = [{features_str}]')

        if default_features is False:
            parts.append('default-features = false')
        elif default_features is True:
            parts.append('default-features = true')

        spec_str = ", ".join(parts)
        return f"{name} = {{ {spec_str} }}"

    return f'{name} = "{dep_spec}"'


def replace_workspace_deps_in_toml(toml_path: Path, workspace_deps: dict[str, object]) -> None:
    """Replace workspace = true with explicit versions in a Cargo.toml file."""
    with open(toml_path, "r") as f:
        content = f.read()

    for name, dep_spec in workspace_deps.items():
        pattern1 = rf'^{re.escape(name)} = \{{ workspace = true \}}$'
        content = re.sub(pattern1, format_dependency(name, dep_spec), content, flags=re.MULTILINE)

        def replace_with_fields(match: re.Match[str]) -> str:
            other_fields_str = match.group(1).strip()
            base_spec = format_dependency(name, dep_spec)
            if " = { " not in base_spec:
                # Simple string dep like `ctor = "0.6"` - wrap it
                version_val = base_spec.split(" = ", 1)[1].strip('"')
                spec_part = f'version = "{version_val}"'
            else:
                spec_part = base_spec.split(" = { ", 1)[1].rstrip("}")

            existing_keys: set[str] = set()
            for part in spec_part.split(","):
                part = part.strip()
                if "=" in part:
                    key = part.split("=")[0].strip()
                    existing_keys.add(key)

            filtered_fields: list[str] = []
            for field in other_fields_str.split(","):
                field = field.strip()
                if field and "=" in field:
                    key = field.split("=")[0].strip()
                    if key not in existing_keys:
                        filtered_fields.append(field)
                elif field:
                    filtered_fields.append(field)

            if filtered_fields:
                return f"{name} = {{ {spec_part}, {', '.join(filtered_fields)} }}"
            else:
                return f"{name} = {{ {spec_part} }}"

        pattern2 = rf'^{re.escape(name)} = \{{ workspace = true, (.+?) \}}$'
        content = re.sub(pattern2, replace_with_fields, content, flags=re.MULTILINE | re.DOTALL)

    with open(toml_path, "w") as f:
        f.write(content)


def generate_vendor_cargo_toml(repo_root: Path, workspace_deps: dict[str, object], core_version: str, copied_crates: list[str]) -> None:
    """Generate vendor/Cargo.toml with workspace setup.

    Args:
        repo_root: Repository root directory
        workspace_deps: Workspace dependencies from Cargo.toml
        core_version: Core version string
        copied_crates: List of crates that were successfully copied
    """

    deps_lines: list[str] = []
    for name, dep_spec in sorted(workspace_deps.items()):
        deps_lines.append(format_dependency(name, dep_spec))

    deps_str = "\n".join(deps_lines)

    # Build members list based on actually copied crates
    members = [name for name in ["kreuzberg", "kreuzberg-ffi", "kreuzberg-tesseract", "kreuzberg-paddle-ocr", "kreuzberg-pdfium-render"]
               if name in copied_crates]
    members_str = ', '.join(f'"{m}"' for m in members)

    vendor_toml = f'''[workspace]
members = [{members_str}]

[workspace.package]
version = "{core_version}"
edition = "2024"
rust-version = "1.91"
authors = ["Na'aman Hirschfeld <nhirschfeld@gmail.com>"]
license = "MIT"
repository = "https://github.com/kreuzberg-dev/kreuzberg"
homepage = "https://kreuzberg.dev"

[workspace.dependencies]
{deps_str}
'''

    vendor_dir = repo_root / "packages" / "r" / "vendor"
    vendor_dir.mkdir(parents=True, exist_ok=True)

    toml_path = vendor_dir / "Cargo.toml"
    with open(toml_path, "w") as f:
        f.write(vendor_toml)


def main() -> None:
    """Main vendoring function."""
    repo_root: Path = get_repo_root()

    print("=== Vendoring kreuzberg core crate ===")

    workspace_deps: dict[str, object] = get_workspace_deps(repo_root)
    core_version: str = get_workspace_version(repo_root)

    print(f"Core version: {core_version}")
    print(f"Workspace dependencies: {len(workspace_deps)}")

    vendor_base: Path = repo_root / "packages" / "r" / "vendor"

    # Clean only crate directories
    crate_names = ["kreuzberg", "kreuzberg-ffi", "kreuzberg-tesseract",
                   "kreuzberg-paddle-ocr", "kreuzberg-pdfium-render"]
    for name in crate_names:
        crate_path = vendor_base / name
        if crate_path.exists():
            shutil.rmtree(crate_path)
    # Also clean the vendor Cargo.toml (will be regenerated)
    vendor_cargo = vendor_base / "Cargo.toml"
    if vendor_cargo.exists():
        vendor_cargo.unlink()
    print("Cleaned vendor crate directories")

    vendor_base.mkdir(parents=True, exist_ok=True)

    crates_to_copy: list[tuple[str, str]] = [
        ("crates/kreuzberg", "kreuzberg"),
        ("crates/kreuzberg-ffi", "kreuzberg-ffi"),
        ("crates/kreuzberg-tesseract", "kreuzberg-tesseract"),
        ("crates/kreuzberg-paddle-ocr", "kreuzberg-paddle-ocr"),
        ("crates/kreuzberg-pdfium-render", "kreuzberg-pdfium-render"),
    ]

    copied_crates: list[str] = []
    for src_rel, dest_name in crates_to_copy:
        src: Path = repo_root / src_rel
        dest: Path = vendor_base / dest_name
        if src.exists():
            try:
                shutil.copytree(src, dest)
                copied_crates.append(dest_name)
                print(f"Copied {dest_name}")
            except Exception as e:
                print(f"Warning: Failed to copy {dest_name}: {e}", file=sys.stderr)
        else:
            print(f"Warning: Source directory not found: {src_rel}")

    artifact_dirs: list[str] = [".fastembed_cache", "target"]
    temp_patterns: list[str] = ["*.swp", "*.bak", "*.tmp", "*~"]

    for crate_dir in copied_crates:
        crate_path: Path = vendor_base / crate_dir
        if crate_path.exists():
            for artifact_dir in artifact_dirs:
                artifact: Path = crate_path / artifact_dir
                if artifact.exists():
                    shutil.rmtree(artifact)

            for pattern in temp_patterns:
                for f in crate_path.rglob(pattern):
                    f.unlink()

    print("Cleaned build artifacts")

    # Update workspace inheritance in Cargo.toml files
    for crate_dir in copied_crates:
        crate_toml = vendor_base / crate_dir / "Cargo.toml"
        if crate_toml.exists():
            with open(crate_toml, "r") as f:
                content = f.read()

            content = re.sub(r'^version\.workspace = true$', f'version = "{core_version}"', content, flags=re.MULTILINE)
            content = re.sub(r'^edition\.workspace = true$', 'edition = "2024"', content, flags=re.MULTILINE)
            content = re.sub(r'^rust-version\.workspace = true$', 'rust-version = "1.91"', content, flags=re.MULTILINE)
            content = re.sub(r'^authors\.workspace = true$', 'authors = ["Na\'aman Hirschfeld <nhirschfeld@gmail.com>"]', content, flags=re.MULTILINE)
            content = re.sub(r'^license\.workspace = true$', 'license = "MIT"', content, flags=re.MULTILINE)

            with open(crate_toml, "w") as f:
                f.write(content)

            replace_workspace_deps_in_toml(crate_toml, workspace_deps)
            print(f"Updated {crate_dir}/Cargo.toml")

    # Update path dependencies in kreuzberg crate if tesseract was copied
    if "kreuzberg" in copied_crates:
        kreuzberg_toml = vendor_base / "kreuzberg" / "Cargo.toml"
        if kreuzberg_toml.exists():
            with open(kreuzberg_toml, "r") as f:
                content = f.read()

            # Only update tesseract path if it was actually copied
            if "kreuzberg-tesseract" in copied_crates:
                content = re.sub(
                    r'kreuzberg-tesseract = \{ version = "[^"]*", optional = true \}',
                    'kreuzberg-tesseract = { path = "../kreuzberg-tesseract", optional = true }',
                    content
                )
            # Only update paddle-ocr path if it was actually copied
            if "kreuzberg-paddle-ocr" in copied_crates:
                content = re.sub(
                    r'kreuzberg-paddle-ocr = \{ version = "[^"]*", optional = true \}',
                    'kreuzberg-paddle-ocr = { path = "../kreuzberg-paddle-ocr", optional = true }',
                    content
                )
            # Only update pdfium-render path if it was actually copied
            if "kreuzberg-pdfium-render" in copied_crates:
                content = re.sub(
                    r'pdfium-render = \{ package = "kreuzberg-pdfium-render", version = "[^"]*"',
                    'pdfium-render = { package = "kreuzberg-pdfium-render", path = "../kreuzberg-pdfium-render"',
                    content
                )

            with open(kreuzberg_toml, "w") as f:
                f.write(content)

    generate_vendor_cargo_toml(repo_root, workspace_deps, core_version, copied_crates)
    print("Generated vendor/Cargo.toml")

    # Update R package Cargo.toml to use vendored crates
    r_toml = repo_root / "packages" / "r" / "src" / "rust" / "Cargo.toml"
    if r_toml.exists():
        with open(r_toml, "r") as f:
            content = f.read()

        # Replace path dependencies to point to vendored crates
        # From: path = "../../../../crates/kreuzberg"
        # To: path = "../../vendor/kreuzberg"
        content = re.sub(
            r'path = "\.\./\.\./\.\./\.\./crates/kreuzberg"',
            'path = "../../vendor/kreuzberg"',
            content
        )
        content = re.sub(
            r'path = "\.\./\.\./\.\./\.\./crates/kreuzberg-ffi"',
            'path = "../../vendor/kreuzberg-ffi"',
            content
        )

        with open(r_toml, "w") as f:
            f.write(content)

        print("Updated R package Cargo.toml to use vendored crates")

    print(f"\nVendoring complete (core version: {core_version})")
    print(f"Copied crates: {', '.join(sorted(copied_crates))}")

    if "kreuzberg" in copied_crates and "kreuzberg-ffi" in copied_crates:
        print("R package Cargo.toml uses:")
        print("  - path '../../vendor/kreuzberg' for kreuzberg crate")
        print("  - path '../../vendor/kreuzberg-ffi' for kreuzberg-ffi crate")
    else:
        print("Warning: Some required crates were not copied. Check for missing source directories.")


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
