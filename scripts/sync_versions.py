"""
Sync version from Cargo.toml workspace to all package manifests.

This script reads the version from Cargo.toml [workspace.package] and updates:
- All package.json files (TypeScript/Node.js packages)
- Python pyproject.toml files
- Ruby version.rb file
- Elixir mix.exs file
- R DESCRIPTION file
- Cargo.toml files with hardcoded versions (not using workspace)
"""

import json
import re
import sys
from pathlib import Path
from typing import List, Tuple


def get_repo_root() -> Path:
    """Get the repository root directory."""
    script_dir = Path(__file__).resolve().parent
    return script_dir.parent


def get_workspace_version(repo_root: Path) -> str:
    """Extract version from Cargo.toml [workspace.package]."""
    cargo_toml = repo_root / "Cargo.toml"
    if not cargo_toml.exists():
        raise FileNotFoundError(f"Cargo.toml not found at {cargo_toml}")

    content = cargo_toml.read_text()
    match = re.search(
        r'^\[workspace\.package\]\s*\nversion\s*=\s*"([^"]+)"',
        content,
        re.MULTILINE
    )

    if not match:
        raise ValueError("Could not find version in Cargo.toml [workspace.package]")

    return match.group(1)


def update_package_json(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update a package.json file.

    Returns: (changed, old_version, new_version)
    """
    data = json.loads(file_path.read_text())
    old_version = data.get("version", "N/A")
    changed = False

    if data.get("version") != version:
        data["version"] = version
        changed = True

    def maybe_update(dep_section: str) -> None:
        nonlocal changed
        if dep_section not in data:
            return

        for dep_name, dep_version in list(data[dep_section].items()):
            if not dep_name.startswith(("kreuzberg-", "@kreuzberg/")):
                continue
            if isinstance(dep_version, str) and dep_version.startswith(("workspace:", "file:", "link:", "portal:")):
                continue
            if dep_version != version:
                data[dep_section][dep_name] = version
                changed = True

    for section in ("dependencies", "optionalDependencies", "devDependencies", "peerDependencies"):
        maybe_update(section)

    if changed:
        file_path.write_text(json.dumps(data, indent=2) + "\n")

    return changed, old_version, version


def update_pyproject_toml(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update a pyproject.toml file.

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()
    original_content = content
    match = re.search(r'^version\s*=\s*"([^"]+)"', content, re.MULTILINE)
    old_version = match.group(1) if match else "NOT FOUND"

    if old_version != version:
        content = re.sub(
            r'^(version\s*=\s*)"[^"]+"',
            rf'\1"{version}"',
            content,
            count=1,
            flags=re.MULTILINE
        )

    dep_version = version.replace("rc.", "rc")
    dep_pattern = r'(kreuzberg\s*==\s*")([^"]+)(")'
    dep_match = re.search(dep_pattern, content)
    if dep_match and dep_match.group(2) != dep_version:
        content = re.sub(dep_pattern, rf'\g<1>{dep_version}\g<3>', content)

    if content != original_content:
        file_path.write_text(content)
        return True, old_version, version

    return False, old_version, version


def update_ruby_version(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update Ruby version.rb file.

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()
    match = re.search(r'VERSION\s*=\s*(["\'])([^"\']+)\1', content)
    old_version = match.group(2) if match else "NOT FOUND"
    quote = match.group(1) if match else '"'

    if old_version == version:
        return False, old_version, version

    new_content = re.sub(
        r'(VERSION\s*=\s*)(["\'])([^"\']+)\2',
        rf"\g<1>{quote}{version}{quote}",
        content,
    )

    file_path.write_text(new_content)
    return True, old_version, version


def update_cargo_toml(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update a Cargo.toml file that has hardcoded version (not using workspace).

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()
    original_content = content
    match = re.search(r'^version\s*=\s*"([^"]+)"', content, re.MULTILINE)
    old_version = match.group(1) if match else "NOT FOUND"

    if old_version != version:
        content = re.sub(
            r'^(version\s*=\s*)"[^"]+"',
            rf'\1"{version}"',
            content,
            count=1,
            flags=re.MULTILINE
        )

    dep_pattern = r'(kreuzberg\s*=\s*")([^"]+)(")'
    dep_match = re.search(dep_pattern, content)
    if dep_match and dep_match.group(2) != version:
        content = re.sub(dep_pattern, rf'\g<1>{version}\g<3>', content)

    if content != original_content:
        file_path.write_text(content)
        return True, old_version, version

    return False, old_version, version


def update_go_mod(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update a go.mod file module version in require statements.

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()

    pattern = r'(github\.com/kreuzberg-dev/kreuzberg(?:/[^\s]+)?\s+)v([0-9]+\.[0-9]+\.[0-9]+(?:-[^\s]+)?)'
    match = re.search(pattern, content)
    old_version = match.group(2) if match else "NOT FOUND"

    if old_version == version:
        return False, old_version, version

    if not re.search(pattern, content):
        return False, "NOT FOUND", version

    new_content = re.sub(
        pattern,
        rf'\g<1>v{version}',
        content,
        flags=re.MULTILINE
    )

    if new_content != content:
        file_path.write_text(new_content)
        return True, old_version, version

    return False, old_version, version


def update_text_file(file_path: Path, pattern: str, repl: str) -> Tuple[bool, str, str]:
    """
    Update a plain text file using regex substitution.

    Returns: (changed, old_value, new_value)
    """
    content = file_path.read_text()
    match = re.search(pattern, content, re.MULTILINE)
    if match:
        old_value = match.group(1) if match.groups() else match.group(0)
    else:
        old_value = "NOT FOUND"

    new_content, count = re.subn(
        pattern,
        repl,
        content,
        flags=re.MULTILINE | re.DOTALL,
    )

    if count == 0:
        return False, old_value, old_value

    if new_content == content:
        return False, old_value, old_value

    file_path.write_text(new_content)
    return True, old_value, repl


def normalize_rubygems_version(version: str) -> str:
    if "-" not in version:
        return version
    base, prerelease = version.split("-", 1)
    return f"{base}.pre.{prerelease.replace('-', '.')}"


def normalize_python_version(version: str) -> str:
    """Convert semver version to Python package version format (replace - with no separator)."""
    return version.replace("-", "")


def update_pom_xml(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update kreuzberg dependency version in pom.xml.

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()

    pattern = r'(<artifactId>kreuzberg</artifactId>\s*<version>)([^<]+)(</version>)'
    match = re.search(pattern, content, re.DOTALL)
    old_version = match.group(2) if match else "NOT FOUND"

    if old_version == version:
        return False, old_version, version

    new_content = re.sub(
        pattern,
        rf"\g<1>{version}\g<3>",
        content,
        flags=re.DOTALL
    )

    if new_content != content:
        file_path.write_text(new_content)
        return True, old_version, version

    return False, old_version, version


def update_csproj(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update Kreuzberg package version in .csproj file.

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()

    pattern = r'(<PackageReference Include="Kreuzberg" Version=")([^"]+)(" />)'
    match = re.search(pattern, content)
    old_version = match.group(2) if match else "NOT FOUND"

    if old_version == version:
        return False, old_version, version

    new_content = re.sub(
        pattern,
        rf"\g<1>{version}\g<3>",
        content
    )

    if new_content != content:
        file_path.write_text(new_content)
        return True, old_version, version

    return False, old_version, version


def update_gemfile(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update kreuzberg gem version in Gemfile.

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()

    pattern = r"(gem\s+['\"]kreuzberg['\"]\s*,\s*['\"])([^'\"]+)(['\"])"
    match = re.search(pattern, content)
    old_version = match.group(2) if match else "NOT FOUND"

    ruby_version = normalize_rubygems_version(version)

    if old_version == ruby_version:
        return False, old_version, ruby_version

    new_content = re.sub(
        pattern,
        rf"\g<1>{ruby_version}\g<3>",
        content
    )

    if new_content != content:
        file_path.write_text(new_content)
        return True, old_version, ruby_version

    return False, old_version, ruby_version


def update_composer_json(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update a composer.json file.

    Returns: (changed, old_version, new_version)
    """
    data = json.loads(file_path.read_text())
    old_version = data.get("version", "N/A")
    changed = False

    if data.get("version") != version:
        data["version"] = version
        changed = True

    if changed:
        file_path.write_text(json.dumps(data, indent=4) + "\n")

    return changed, old_version, version


def update_mix_exs(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update Elixir mix.exs file.

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()
    match = re.search(r'@version\s+"([^"]+)"', content)
    old_version = match.group(1) if match else "NOT FOUND"

    if old_version == version:
        return False, old_version, version

    new_content = re.sub(
        r'(@version\s+)"[^"]+"',
        rf'\1"{version}"',
        content
    )

    if new_content != content:
        file_path.write_text(new_content)
        return True, old_version, version

    return False, old_version, version


def update_mix_exs_testapp(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update Elixir test app mix.exs file (version + dependency).

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()
    original_content = content

    # Update version field
    version_match = re.search(r'version:\s+"([^"]+)"', content)
    old_version = version_match.group(1) if version_match else "NOT FOUND"

    content = re.sub(
        r'(version:\s+)"[^"]+"',
        rf'\1"{version}"',
        content
    )

    # Update kreuzberg dependency
    content = re.sub(
        r'(\{:kreuzberg,\s+"~>\s+)[^"]+("})',
        rf'\g<1>{version}\g<2>',
        content
    )

    if content != original_content:
        file_path.write_text(content)
        return True, old_version, version

    return False, old_version, version


def update_r_description(file_path: Path, version: str) -> Tuple[bool, str, str]:
    """
    Update an R DESCRIPTION file's Version field.

    Returns: (changed, old_version, new_version)
    """
    content = file_path.read_text()
    match = re.search(r'^Version:\s*(.+)$', content, re.MULTILINE)
    old_version = match.group(1).strip() if match else "NOT FOUND"

    if old_version == version:
        return False, old_version, version

    new_content = re.sub(
        r'^(Version:\s*).+$',
        rf'\g<1>{version}',
        content,
        count=1,
        flags=re.MULTILINE
    )

    if new_content != content:
        file_path.write_text(new_content)
        return True, old_version, version

    return False, old_version, version


def main():
    repo_root = get_repo_root()

    try:
        version = get_workspace_version(repo_root)
    except (FileNotFoundError, ValueError) as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    print(f"\n📦 Syncing version {version} from Cargo.toml\n")

    updated_files: List[str] = []
    unchanged_files: List[str] = []

    for pkg_json in repo_root.rglob("package.json"):
        if any(part in pkg_json.parts for part in ["node_modules", ".git", "target", "dist", "bin", "obj", "tmp"]):
            continue

        changed, old_ver, new_ver = update_package_json(pkg_json, version)
        rel_path = pkg_json.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    for pyproject in [
        repo_root / "packages/python/pyproject.toml",
        repo_root / "examples/python/pyproject.toml",
    ]:
        if pyproject.exists():
            changed, old_ver, new_ver = update_pyproject_toml(pyproject, version)
            rel_path = pyproject.relative_to(repo_root)

            if changed:
                print(f"✓ {rel_path}: {old_ver} → {new_ver}")
                updated_files.append(str(rel_path))
            else:
                unchanged_files.append(str(rel_path))

    ruby_version = repo_root / "packages/ruby/lib/kreuzberg/version.rb"
    if ruby_version.exists():
        changed, old_ver, new_ver = update_ruby_version(ruby_version, version)
        rel_path = ruby_version.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    elixir_mix = repo_root / "packages/elixir/mix.exs"
    if elixir_mix.exists():
        changed, old_ver, new_ver = update_mix_exs(elixir_mix, version)
        rel_path = elixir_mix.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    r_description = repo_root / "packages/r/DESCRIPTION"
    if r_description.exists():
        changed, old_ver, new_ver = update_r_description(r_description, version)
        rel_path = r_description.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    # Ruby native extension Cargo.toml (has its own [workspace] so needs explicit handling)
    ruby_native_cargo = repo_root / "packages/ruby/ext/kreuzberg_rb/native/Cargo.toml"
    if ruby_native_cargo.exists():
        changed, old_ver, new_ver = update_cargo_toml(ruby_native_cargo, version)
        rel_path = ruby_native_cargo.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    # Elixir native extension Cargo.toml
    elixir_native_cargo = repo_root / "packages/elixir/native/kreuzberg_rustler/Cargo.toml"
    if elixir_native_cargo.exists():
        changed, old_ver, new_ver = update_cargo_toml(elixir_native_cargo, version)
        rel_path = elixir_native_cargo.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    php_composer = repo_root / "packages/php/composer.json"
    if php_composer.exists():
        changed, old_ver, new_ver = update_composer_json(php_composer, version)
        rel_path = php_composer.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    root_composer = repo_root / "composer.json"
    if root_composer.exists():
        changed, old_ver, new_ver = update_composer_json(root_composer, version)
        rel_path = root_composer.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    text_targets = [
        (
            repo_root / "crates/kreuzberg-node/typescript/index.ts",
            r'__version__ = "([^"]+)"',
            f'__version__ = "{version}"',
        ),
        (
            repo_root / "packages/typescript/tests/binding/cli.spec.ts",
            r'kreuzberg-cli ([0-9A-Za-z\.\-]+)',
            f'kreuzberg-cli {version}',
        ),
        (
            repo_root / "packages/ruby/Gemfile.lock",
            r'(^\s{4}kreuzberg \()[^\)]+(\))',
            rf"\g<1>{normalize_rubygems_version(version)}\g<2>",
        ),
        (
            repo_root / "crates/kreuzberg/Cargo.toml",
            r'^(pdfium-render\s*=\s*\{\s*package\s*=\s*"kreuzberg-pdfium-render"\s*,\s*path\s*=\s*"[^"]+"\s*,\s*version\s*=\s*")[^"]+(")',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "crates/kreuzberg/Cargo.toml",
            r'^(kreuzberg-tesseract\s*=\s*\{\s*path\s*=\s*"[^"]+"\s*,\s*version\s*=\s*")[^"]+("\s*,\s*optional\s*=\s*true\s*\})',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "crates/kreuzberg/Cargo.toml",
            r'^(kreuzberg-paddle-ocr\s*=\s*\{\s*path\s*=\s*"[^"]+"\s*,\s*version\s*=\s*")[^"]+("\s*,\s*optional\s*=\s*true\s*\})',
            rf"\g<1>{version}\g<2>",
        ),
        # Workspace dependency versions in root Cargo.toml
        (
            repo_root / "Cargo.toml",
            r'^(kreuzberg\s*=\s*\{\s*path\s*=\s*"[^"]+"\s*,\s*version\s*=\s*")[^"]+("\s*,\s*default-features)',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "Cargo.toml",
            r'^(kreuzberg-ffi\s*=\s*\{\s*path\s*=\s*"[^"]+"\s*,\s*version\s*=\s*")[^"]+("\s*\})',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "crates/kreuzberg-ffi/kreuzberg-ffi.pc",
            r"^Version:\s*([0-9A-Za-z\.\-]+)\s*$",
            f"Version: {version}",
        ),
        (
            repo_root / "crates/kreuzberg-ffi/kreuzberg-ffi-install.pc",
            r"^Version:\s*([0-9A-Za-z\.\-]+)\s*$",
            f"Version: {version}",
        ),
        (
            repo_root / "crates/kreuzberg-node/tests/binding/cli.spec.ts",
            r'kreuzberg-cli ([0-9A-Za-z\.\-]+)',
            f'kreuzberg-cli {version}',
        ),
        (
            repo_root / "packages/java/README.md",
            r'\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?',
            version,
        ),
        (
            repo_root / "packages/java/pom.xml",
            r'(<artifactId>kreuzberg</artifactId>\s*<version>)([^<]+)(</version>)',
            rf"\g<1>{version}\g<3>",
        ),
        (
            repo_root / "packages/go/README.md",
            r'\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?',
            version,
        ),
        (
            repo_root / "packages/go/v4/doc.go",
            r'\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?',
            version,
        ),
        (
            repo_root / "e2e/java/pom.xml",
            r'(<artifactId>kreuzberg</artifactId>\s*<version>)([^<]+)(</version>)',
            rf"\g<1>{version}\g<3>",
        ),
        (
            repo_root / "tools/e2e-generator/src/java.rs",
            r'(<artifactId>kreuzberg</artifactId>\s*<version>)([^<]+)(</version>)',
            rf"\g<1>{version}\g<3>",
        ),
        (
            repo_root / "e2e/java/pom.xml",
            r'(<systemPath>\$\{project\.basedir\}/\.\./\.\./packages/java/target/kreuzberg-)[^<]+(\.jar</systemPath>)',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "tools/e2e-generator/src/java.rs",
            r'(<systemPath>\$\{project\.basedir\}/\.\./\.\./packages/java/target/kreuzberg-)[^<]+(\.jar</systemPath>)',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "packages/csharp/Kreuzberg/Kreuzberg.csproj",
            r"<Version>[^<]+</Version>",
            f"<Version>{version}</Version>",
        ),
        (
            repo_root / "packages/csharp/README.md",
            r'(PackageReference Include="Kreuzberg" Version=")([^"]+)(")',
            rf"\g<1>{version}\g<3>",
        ),
        # README config for generated READMEs
        (
            repo_root / "scripts/readme_config.yaml",
            r'^(version:\s*")[^"]+(")$',
            rf"\g<1>{version}\g<2>",
        ),
        # PHP package.xml
        (
            repo_root / "packages/php/package.xml",
            r'(<release>)[^<]+(</release>)',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "packages/php/package.xml",
            r'(<api>)[^<]+(</api>)',
            rf"\g<1>{version}\g<2>",
        ),
        # E2E Java pom.xml kreuzberg.version property
        (
            repo_root / "e2e/java/pom.xml",
            r'(<kreuzberg\.version>)[^<]+(</kreuzberg\.version>)',
            rf"\g<1>{version}\g<2>",
        ),
        # E2E generator template kreuzberg.version property
        (
            repo_root / "tools/e2e-generator/e2e/java/pom.xml",
            r'(<kreuzberg\.version>)[^<]+(</kreuzberg\.version>)',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "tools/e2e-generator/src/java.rs",
            r'(<kreuzberg\.version>)[^<]+(</kreuzberg\.version>)',
            rf"\g<1>{version}\g<2>",
        ),
        # Doc API reference version examples
        (
            repo_root / "docs/reference/api-csharp.md",
            r'(PackageReference Include="Kreuzberg" Version=")([^"]+)(")',
            rf"\g<1>{version}\g<3>",
        ),
        (
            repo_root / "docs/reference/api-go.md",
            r'\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?',
            version,
        ),
        (
            repo_root / "docs/reference/api-java.md",
            r'\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?',
            version,
        ),
        # Test app descriptions
        (
            repo_root / "tests/test_apps/rust/Cargo.toml",
            r'(description = "Comprehensive API coverage test for Kreuzberg )[\d\.\-rcRC]+( Rust library")',
            rf"\g<1>{version}\g<2>",
        ),
        (
            repo_root / "tests/test_apps/python/pyproject.toml",
            r'(description = "Comprehensive API coverage test for Kreuzberg )[\d\.\-rcRC]+( Python bindings")',
            rf"\g<1>{version}\g<2>",
        ),
        # Doc comments with version examples
        (
            repo_root / "crates/kreuzberg-php/src/lib.rs",
            r'(Version string in semver format \(e\.g\., ")([^"]+)("\))',
            rf'\g<1>{version}\g<3>',
        ),
        (
            repo_root / "packages/csharp/Kreuzberg/KreuzbergClient.cs",
            r'(Version string in format ")([^"]+)(" or similar)',
            rf'\g<1>{version}\g<3>',
        ),
        # C# PackageReleaseNotes
        (
            repo_root / "packages/csharp/Kreuzberg/Kreuzberg.csproj",
            r'(<PackageReleaseNotes>Version )[^<]+(</PackageReleaseNotes>)',
            rf'\g<1>{version}\g<2>',
        ),
        # Elixir README dependency constraint
        (
            repo_root / "packages/elixir/README.md",
            r'(kreuzberg:\s*"~>\s*)\d+\.\d+(")',
            rf'\g<1>{".".join(version.split(".")[:2])}\g<2>',
        ),
        # Go README badge filter and version references
        (
            repo_root / "packages/go/v4/README.md",
            r'(filter=v)\d+\.\d+\.\d+',
            rf'\g<1>{version}',
        ),
        # All README badge filters
        (
            repo_root / "README.md",
            r'(filter=v)\d+\.\d+\.\d+',
            rf'\g<1>{version}',
        ),
        (
            repo_root / "packages/python/README.md",
            r'(filter=v)\d+\.\d+\.\d+',
            rf'\g<1>{version}',
        ),
        (
            repo_root / "packages/ruby/README.md",
            r'(filter=v)\d+\.\d+\.\d+',
            rf'\g<1>{version}',
        ),
        (
            repo_root / "packages/php/README.md",
            r'(filter=v)\d+\.\d+\.\d+',
            rf'\g<1>{version}',
        ),
        (
            repo_root / "packages/elixir/README.md",
            r'(filter=v)\d+\.\d+\.\d+',
            rf'\g<1>{version}',
        ),
        (
            repo_root / "packages/csharp/README.md",
            r'(filter=v)\d+\.\d+\.\d+',
            rf'\g<1>{version}',
        ),
        (
            repo_root / "crates/kreuzberg-node/README.md",
            r'(filter=v)\d+\.\d+\.\d+',
            rf'\g<1>{version}',
        ),
        (
            repo_root / "crates/kreuzberg-wasm/README.md",
            r'(filter=v)\d+\.\d+\.\d+',
            rf'\g<1>{version}',
        ),
        (
            repo_root / "crates/kreuzberg/README.md",
            r'(filter=v)\d+\.\d+\.\d+',
            rf'\g<1>{version}',
        ),
        # Rust crate README version banner
        (
            repo_root / "crates/kreuzberg/README.md",
            r'(> \*\*🚀 Version )\d+\.\d+\.\d+[^*]*(\*\*)',
            rf'\g<1>{version} Release\g<2>',
        ),
        # Docker compose images
        (
            repo_root / "tests/test_apps/docker/docker-compose.yml",
            r'(image: kreuzberg-dev/kreuzberg:)\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?(-core)?',
            rf'\g<1>{version}\g<2>',
        ),
        # Docs: Installation guide Java Maven/Gradle versions
        (
            repo_root / "docs/getting-started/installation.md",
            r'(<version>)\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?(</version>)',
            rf'\g<1>{version}\g<2>',
        ),
        (
            repo_root / "docs/getting-started/installation.md",
            r"(implementation 'dev\.kreuzberg:kreuzberg:)\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?(')",
            rf"\g<1>{version}\g<2>",
        ),
        # Docs: Elixir API reference Hex dependency version
        (
            repo_root / "docs/reference/api-elixir.md",
            r'(\{:kreuzberg, "~> )\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?("\})',
            rf'\g<1>{version}\g<2>',
        ),
        # Docs: Environment variables reference header version
        (
            repo_root / "docs/reference/environment-variables.md",
            r'(This document covers all KREUZBERG_\* environment variables for version )\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?(\.)$',
            rf'\g<1>{version}\g<2>',
        ),
        # Docs: API server guide health check response version
        (
            repo_root / "docs/guides/api-server.md",
            r'("version": ")\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?(")',
            rf'\g<1>{version}\g<2>',
        ),
        # Test app source code version references
        (
            repo_root / "tests/test_apps/rust/src/main.rs",
            r'(//! Comprehensive test suite for Kreuzberg )\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?( Rust library)',
            rf'\g<1>{version}\g<2>',
        ),
        (
            repo_root / "tests/test_apps/rust/src/main.rs",
            r'(println!\("\\nVersion: kreuzberg )\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?("\);)',
            rf'\g<1>{version}\g<2>',
        ),
        (
            repo_root / "tests/test_apps/python/main.py",
            r'("""Comprehensive test suite for Kreuzberg Python bindings v)\d+\.\d+\.\d+(?:-[a-zA-Z0-9.]+)?(\.)$',
            rf'\g<1>{version}\g<2>',
        ),
    ]

    for path, pattern, repl in text_targets:
        if not path.exists():
            continue

        changed, old_ver, new_ver = update_text_file(path, pattern, repl)
        rel_path = path.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    print()
    for cargo_toml in repo_root.rglob("Cargo.toml"):
        if cargo_toml == repo_root / "Cargo.toml":
            continue
        if any(part in cargo_toml.parts for part in ["target", "tmp", "vendor", "bin", "obj", "node_modules", "stage"]):
            continue

        content = cargo_toml.read_text()
        if re.search(r'^version\s*=\s*"[^"]+"', content, re.MULTILINE):
            # Check if version is using workspace inheritance
            has_version_workspace = re.search(r'^\s*version\s*\.workspace\s*=\s*true', content, re.MULTILINE)
            has_version_in_workspace_table = re.search(r'^\[package\].*?^\s*version\s*=.*?workspace\s*=\s*true', content, re.MULTILINE | re.DOTALL)

            if not has_version_workspace and not has_version_in_workspace_table:
                changed, old_ver, new_ver = update_cargo_toml(cargo_toml, version)
                rel_path = cargo_toml.relative_to(repo_root)

                if changed:
                    print(f"✓ {rel_path}: {old_ver} → {new_ver}")
                    updated_files.append(str(rel_path))
                else:
                    unchanged_files.append(str(rel_path))

    for go_mod in repo_root.rglob("go.mod"):
        if any(part in go_mod.parts for part in ["target", "vendor", "bin", "obj", "node_modules", "tmp", "stage"]):
            continue

        changed, old_ver, new_ver = update_go_mod(go_mod, f"{version}")
        rel_path = go_mod.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        elif old_ver != "NOT FOUND":
            unchanged_files.append(str(rel_path))

    print()
    test_apps_manifests = [
        (
            repo_root / "tests/test_apps/python/pyproject.toml",
            lambda p, v: update_pyproject_toml(p, normalize_python_version(v))
        ),
        (
            repo_root / "tests/test_apps/node/package.json",
            lambda p, v: update_package_json(p, v)
        ),
        (
            repo_root / "tests/test_apps/wasm/package.json",
            lambda p, v: update_package_json(p, v)
        ),
        (
            repo_root / "tests/test_apps/ruby/Gemfile",
            lambda p, v: update_gemfile(p, v)
        ),
        (
            repo_root / "tests/test_apps/go/go.mod",
            lambda p, v: update_go_mod(p, v)
        ),
        (
            repo_root / "tests/test_apps/java/pom.xml",
            lambda p, v: update_pom_xml(p, v)
        ),
        (
            repo_root / "tests/test_apps/csharp/KreuzbergSmokeTest.csproj",
            lambda p, v: update_csproj(p, v)
        ),
        (
            repo_root / "tests/test_apps/rust/Cargo.toml",
            lambda p, v: update_cargo_toml(p, v)
        ),
        (
            repo_root / "tests/test_apps/elixir/mix.exs",
            lambda p, v: update_mix_exs_testapp(p, v)
        ),
        (
            repo_root / "tests/test_apps/minimal-test/minimal.csproj",
            lambda p, v: update_csproj(p, v)
        ),
    ]

    for manifest_path, update_func in test_apps_manifests:
        if not manifest_path.exists():
            continue

        changed, old_ver, new_ver = update_func(manifest_path, version)
        rel_path = manifest_path.relative_to(repo_root)

        if changed:
            print(f"✓ {rel_path}: {old_ver} → {new_ver}")
            updated_files.append(str(rel_path))
        else:
            unchanged_files.append(str(rel_path))

    print(f"\n📊 Summary:")
    print(f"   Updated: {len(updated_files)} files")
    print(f"   Unchanged: {len(unchanged_files)} files")

    if updated_files:
        print(f"\n✨ Version sync complete! All files now at {version}\n")
    else:
        print(f"\n✨ All files already at {version}\n")


if __name__ == "__main__":
    main()
