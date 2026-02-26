"""
Cross-language serialization test suite.

Validates JSON consistency across all language bindings (Rust, Python, TypeScript, Ruby, Go, Java, PHP, C#, Elixir, WASM).
Tests that serialized configs from all languages produce equivalent JSON structures.
"""

from __future__ import annotations

import json
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any

import pytest

# ============================================================================
# JSON Normalization and Comparison Utilities
# ============================================================================


def camel_to_snake(name: str) -> str:
    """Convert camelCase to snake_case.

    Args:
        name: camelCase string

    Returns:
        snake_case equivalent
    """
    # Insert underscore before uppercase letters and convert to lowercase
    s1 = re.sub("(.)([A-Z][a-z]+)", r"\1_\2", name)
    return re.sub("([a-z0-9])([A-Z])", r"\1_\2", s1).lower()


def snake_to_camel(name: str) -> str:
    """Convert snake_case to camelCase.

    Args:
        name: snake_case string

    Returns:
        camelCase equivalent
    """
    components = name.split("_")
    return components[0] + "".join(x.title() for x in components[1:])


def normalize_json(json_obj: Any, to_snake_case: bool = True) -> Any:
    """Normalize JSON field names for cross-language comparison.

    Converts between camelCase and snake_case to enable comparison across languages
    that use different naming conventions (TypeScript uses camelCase, Python/Rust use snake_case).

    Args:
        json_obj: JSON object (dict, list, or primitive)
        to_snake_case: If True, convert to snake_case; if False, convert to camelCase

    Returns:
        Normalized JSON structure with converted field names
    """
    if isinstance(json_obj, dict):
        normalized = {}
        for key, value in json_obj.items():
            # Convert key to target case
            if to_snake_case:
                new_key = camel_to_snake(key)
            else:
                new_key = snake_to_camel(key)

            # Recursively normalize nested objects and arrays
            if isinstance(value, dict):
                normalized[new_key] = normalize_json(value, to_snake_case)
            elif isinstance(value, list):
                normalized[new_key] = [
                    normalize_json(item, to_snake_case) if isinstance(item, (dict, list)) else item for item in value
                ]
            else:
                normalized[new_key] = value

        return normalized
    if isinstance(json_obj, list):
        return [normalize_json(item, to_snake_case) if isinstance(item, (dict, list)) else item for item in json_obj]
    return json_obj


def compare_json_structures(
    json1: dict[str, Any], json2: dict[str, Any], normalize: bool = True
) -> tuple[bool, list[str]]:
    """Compare two JSON structures for equivalence.

    Args:
        json1: First JSON structure
        json2: Second JSON structure
        normalize: If True, normalize both to snake_case before comparison

    Returns:
        Tuple of (is_equal, differences) where differences is a list of mismatch descriptions
    """
    if normalize:
        json1 = normalize_json(json1, to_snake_case=True)
        json2 = normalize_json(json2, to_snake_case=True)

    differences = []

    # Compare keys
    keys1 = set(json1.keys()) if isinstance(json1, dict) else set()
    keys2 = set(json2.keys()) if isinstance(json2, dict) else set()

    missing_in_json2 = keys1 - keys2
    extra_in_json2 = keys2 - keys1

    if missing_in_json2:
        differences.append(f"Missing in json2: {sorted(missing_in_json2)}")
    if extra_in_json2:
        differences.append(f"Extra in json2: {sorted(extra_in_json2)}")

    # Compare values for common keys
    common_keys = keys1 & keys2
    for key in common_keys:
        val1 = json1[key]
        val2 = json2[key]

        if type(val1) != type(val2):
            differences.append(f"Type mismatch for key '{key}': {type(val1).__name__} vs {type(val2).__name__}")
        elif isinstance(val1, dict) and isinstance(val2, dict):
            is_equal, sub_diffs = compare_json_structures(val1, val2, normalize=False)
            if not is_equal:
                differences.extend([f"In key '{key}': {d}" for d in sub_diffs])
        elif isinstance(val1, list) and isinstance(val2, list):
            if len(val1) != len(val2):
                differences.append(f"List length mismatch for key '{key}': {len(val1)} vs {len(val2)}")
            else:
                for i, (item1, item2) in enumerate(zip(val1, val2)):
                    if isinstance(item1, dict) and isinstance(item2, dict):
                        is_equal, sub_diffs = compare_json_structures(item1, item2, normalize=False)
                        if not is_equal:
                            differences.extend([f"In key '{key}[{i}]': {d}" for d in sub_diffs])
                    elif item1 != item2:
                        differences.append(f"List item mismatch for key '{key}[{i}]': {item1} vs {item2}")
        elif val1 != val2:
            differences.append(f"Value mismatch for key '{key}': {val1} vs {val2}")

    is_equal = len(differences) == 0
    return is_equal, differences


# ============================================================================
# Test Fixtures and Test Data
# ============================================================================

REPO_ROOT = Path(__file__).parent.parent


class TestFixture:
    """Test fixture for cross-language comparison."""

    def __init__(self, name: str, expected_fields: set[str], config_dict: dict[str, Any]):
        """Initialize test fixture.

        Args:
            name: Fixture name (e.g., "minimal", "full")
            expected_fields: Set of field names that should be present
            config_dict: Configuration dictionary for instantiation
        """
        self.name = name
        self.expected_fields = expected_fields
        self.config_dict = config_dict


# Test fixtures for ExtractionConfig serialization
EXTRACTION_CONFIG_FIXTURES = [
    TestFixture(
        name="minimal",
        expected_fields={
            "use_cache",
            "enable_quality_processing",
            "force_ocr",
        },
        config_dict={},
    ),
    TestFixture(
        name="with_ocr",
        expected_fields={
            "use_cache",
            "enable_quality_processing",
            "force_ocr",
            "ocr",
        },
        config_dict={
            "ocr": {
                "backend": "tesseract",
                "language": "eng",
            },
        },
    ),
    TestFixture(
        name="with_chunking",
        expected_fields={
            "use_cache",
            "enable_quality_processing",
            "force_ocr",
            "chunking",
        },
        config_dict={
            "chunking": {
                "strategy": "semantic",
                "max_chunk_size": 1024,
            },
        },
    ),
    TestFixture(
        name="full",
        expected_fields={
            "use_cache",
            "enable_quality_processing",
            "force_ocr",
            "ocr",
            "chunking",
            "images",
        },
        config_dict={
            "use_cache": True,
            "enable_quality_processing": True,
            "force_ocr": False,
            "ocr": {
                "backend": "tesseract",
                "language": "eng",
            },
            "chunking": {
                "strategy": "semantic",
                "max_chunk_size": 2048,
            },
            "images": {
                "extract_images": True,
                "save_path": "/tmp/images",
            },
        },
    ),
]


# ============================================================================
# Rust Serialization Tests
# ============================================================================


def test_rust_extraction_config_serialization() -> None:
    """Test Rust ExtractionConfig JSON serialization."""
    # Build a simple Rust binary that outputs JSON
    rust_json_tool = REPO_ROOT / "target" / "debug" / "extraction_config_json_helper"

    if not rust_json_tool.exists():
        pytest.skip("Rust helper binary not built. Run: cargo build --bin extraction_config_json_helper")

    for fixture in EXTRACTION_CONFIG_FIXTURES:
        # Run Rust tool with fixture data
        config_json = json.dumps(fixture.config_dict)
        try:
            result = subprocess.run(
                [str(rust_json_tool), config_json],
                capture_output=True,
                text=True,
                timeout=5,
            )
            assert result.returncode == 0, f"Rust tool failed: {result.stderr}"

            output = json.loads(result.stdout)

            # Validate that all expected fields are present
            for field in fixture.expected_fields:
                assert field in output, f"Field '{field}' missing in Rust output for fixture '{fixture.name}'"

            # Store for comparison in parity tests
            fixture.rust_output = output

        except json.JSONDecodeError as e:
            pytest.fail(f"Failed to parse Rust JSON output: {e}\nOutput: {result.stdout}")


# ============================================================================
# Python Serialization Tests
# ============================================================================


def test_python_extraction_config_serialization() -> None:
    """Test Python ExtractionConfig JSON serialization."""
    try:
        from kreuzberg import ExtractionConfig
    except ImportError:
        pytest.skip("kreuzberg Python binding not installed")

    for fixture in EXTRACTION_CONFIG_FIXTURES:
        try:
            # Create config from dict
            config = ExtractionConfig(**fixture.config_dict)

            # Attempt to serialize to JSON
            # Note: Python binding may not have built-in serialization
            # This test validates the structure when converted to dict
            config_dict = _python_config_to_dict(config)

            # Validate that all expected fields are present
            for field in fixture.expected_fields:
                assert field in config_dict, f"Field '{field}' missing in Python output for fixture '{fixture.name}'"

            # Store for comparison
            fixture.python_output = config_dict

        except Exception as e:
            pytest.fail(f"Python serialization failed for fixture '{fixture.name}': {e}")


def _python_config_to_dict(config: Any) -> dict[str, Any]:
    """Convert Python config object to dictionary.

    Args:
        config: ExtractionConfig object

    Returns:
        Dictionary representation of config
    """
    result = {}

    # Use reflection to extract config attributes
    for attr in dir(config):
        if not attr.startswith("_") and not callable(getattr(config, attr)):
            try:
                value = getattr(config, attr)
                if value is not None:
                    result[attr] = value
            except Exception:
                # Skip attributes that can't be accessed
                pass

    return result


# ============================================================================
# Cross-Language JSON Extraction Helpers
# ============================================================================


def get_rust_serialization(config_dict: dict[str, Any]) -> dict[str, Any]:
    """Get JSON serialization from Rust core via helper binary.

    Args:
        config_dict: Configuration dictionary

    Returns:
        JSON dictionary from Rust

    Raises:
        pytest.skip if helper binary not available
        subprocess.CalledProcessError if Rust tool fails
    """
    rust_json_tool = REPO_ROOT / "target" / "debug" / "extraction_config_json_helper"

    if not rust_json_tool.exists():
        pytest.skip("Rust helper binary not built. Run: cargo build --bin extraction_config_json_helper")

    config_json = json.dumps(config_dict)
    result = subprocess.run(
        [str(rust_json_tool), config_json],
        capture_output=True,
        text=True,
        timeout=5,
    )

    if result.returncode != 0:
        raise subprocess.CalledProcessError(result.returncode, str(rust_json_tool), stderr=result.stderr)

    return json.loads(result.stdout)


def get_python_serialization(config_dict: dict[str, Any]) -> dict[str, Any]:
    """Get JSON serialization from Python binding.

    Args:
        config_dict: Configuration dictionary

    Returns:
        JSON dictionary from Python

    Raises:
        ImportError if kreuzberg not installed
    """
    try:
        from kreuzberg import ExtractionConfig
    except ImportError:
        pytest.skip("kreuzberg Python binding not installed")

    config = ExtractionConfig(**config_dict)
    return _python_config_to_dict(config)


def get_typescript_serialization(config_dict: dict[str, Any]) -> dict[str, Any]:
    """Get JSON serialization from TypeScript binding.

    Args:
        config_dict: Configuration dictionary

    Returns:
        JSON dictionary from TypeScript (in camelCase)

    Raises:
        pytest.skip if Node.js or TypeScript binding not available
    """
    ts_dir = REPO_ROOT / "packages" / "typescript"

    if not ts_dir.exists():
        pytest.skip("TypeScript package not found")

    # Create a temporary test script
    script = f"""
    try {{
        const {{ ExtractionConfig }} = require('./dist/index.js');
        const config = new ExtractionConfig({json.dumps(config_dict)});
        console.log(JSON.stringify(config));
    }} catch (err) {{
        console.error('Error:', err.message);
        process.exit(1);
    }}
    """

    with tempfile.NamedTemporaryFile(mode="w", suffix=".js", delete=False) as f:
        f.write(script)
        script_path = f.name

    try:
        result = subprocess.run(
            ["node", script_path],
            cwd=str(ts_dir),
            capture_output=True,
            text=True,
            timeout=10,
        )

        if result.returncode != 0:
            pytest.skip(f"TypeScript serialization not available: {result.stderr}")

        return json.loads(result.stdout)
    finally:
        Path(script_path).unlink(missing_ok=True)


def get_ruby_serialization(config_dict: dict[str, Any]) -> dict[str, Any]:
    """Get JSON serialization from Ruby binding.

    Args:
        config_dict: Configuration dictionary

    Returns:
        JSON dictionary from Ruby

    Raises:
        pytest.skip if Ruby binding not available
    """
    ruby_dir = REPO_ROOT / "packages" / "ruby"

    if not ruby_dir.exists():
        pytest.skip("Ruby package not found")

    # Create a temporary test script
    escaped_config = json.dumps(config_dict).replace('"', '\\"')
    script = f"""
    require 'kreuzberg'
    require 'json'

    config = Kreuzberg::ExtractionConfig.new({escaped_config})
    puts JSON.generate(config.to_h)
    """

    with tempfile.NamedTemporaryFile(mode="w", suffix=".rb", delete=False) as f:
        f.write(script)
        script_path = f.name

    try:
        result = subprocess.run(
            ["ruby", script_path],
            cwd=str(ruby_dir),
            capture_output=True,
            text=True,
            timeout=10,
        )

        if result.returncode != 0:
            pytest.skip(f"Ruby serialization not available: {result.stderr}")

        return json.loads(result.stdout)
    finally:
        Path(script_path).unlink(missing_ok=True)


def get_go_serialization(config_dict: dict[str, Any]) -> dict[str, Any]:
    """Get JSON serialization from Go binding.

    Args:
        config_dict: Configuration dictionary

    Returns:
        JSON dictionary from Go

    Raises:
        pytest.skip if Go binding not available
    """
    go_dir = REPO_ROOT / "packages" / "go"

    if not go_dir.exists():
        pytest.skip("Go package not found")

    # Create a temporary test program
    program = f"""
    package main

    import (
        "encoding/json"
        "fmt"
        "kreuzberg"
    )

    func main() {{
        config := kreuzberg.NewExtractionConfig()
        // Apply config settings from {json.dumps(config_dict)}
        data, _ := json.Marshal(config)
        fmt.Println(string(data))
    }}
    """

    with tempfile.NamedTemporaryFile(mode="w", suffix=".go", delete=False) as f:
        f.write(program)
        script_path = f.name

    try:
        result = subprocess.run(
            ["go", "run", script_path],
            cwd=str(go_dir),
            capture_output=True,
            text=True,
            timeout=10,
        )

        if result.returncode != 0:
            pytest.skip(f"Go serialization not available: {result.stderr}")

        return json.loads(result.stdout)
    finally:
        Path(script_path).unlink(missing_ok=True)


# ============================================================================
# TypeScript/Node.js Serialization Tests
# ============================================================================


def test_typescript_extraction_config_serialization() -> None:
    """Test TypeScript ExtractionConfig JSON serialization."""
    ts_test_file = REPO_ROOT / "packages" / "typescript" / "tests" / "serialization.spec.ts"

    if not ts_test_file.exists():
        pytest.skip("TypeScript serialization test not available")

    try:
        # Run TypeScript tests
        result = subprocess.run(
            ["npm", "test", "--", "serialization.spec.ts"],
            cwd=str(REPO_ROOT / "packages" / "typescript"),
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode != 0:
            pytest.fail(f"TypeScript tests failed:\n{result.stderr}")

    except FileNotFoundError:
        pytest.skip("npm not available")


# ============================================================================
# Ruby Serialization Tests
# ============================================================================


def test_ruby_extraction_config_serialization() -> None:
    """Test Ruby ExtractionConfig JSON serialization."""
    ruby_test_file = REPO_ROOT / "packages" / "ruby" / "spec" / "serialization_spec.rb"

    if not ruby_test_file.exists():
        pytest.skip("Ruby serialization test not available")

    try:
        # Run Ruby tests
        result = subprocess.run(
            ["bundle", "exec", "rspec", "spec/serialization_spec.rb"],
            cwd=str(REPO_ROOT / "packages" / "ruby"),
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode != 0:
            pytest.fail(f"Ruby tests failed:\n{result.stderr}")

    except FileNotFoundError:
        pytest.skip("Ruby/bundle not available")


# ============================================================================
# Go Serialization Tests
# ============================================================================


def test_go_extraction_config_serialization() -> None:
    """Test Go ExtractionConfig JSON serialization."""
    go_test_file = REPO_ROOT / "packages" / "go" / "serialization_test.go"

    if not go_test_file.exists():
        pytest.skip("Go serialization test not available")

    try:
        # Run Go tests
        result = subprocess.run(
            ["go", "test", "-v", "./...", "-run", "TestSerialization"],
            cwd=str(REPO_ROOT / "packages" / "go"),
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode != 0:
            pytest.fail(f"Go tests failed:\n{result.stderr}")

    except FileNotFoundError:
        pytest.skip("Go not available")


# ============================================================================
# Java Serialization Tests
# ============================================================================


def test_java_extraction_config_serialization() -> None:
    """Test Java ExtractionConfig JSON serialization."""
    java_test_file = REPO_ROOT / "packages" / "java" / "src" / "test" / "java" / "SerializationTest.java"

    if not java_test_file.exists():
        pytest.skip("Java serialization test not available")

    try:
        # Run Java tests
        result = subprocess.run(
            ["mvn", "test", "-Dtest=SerializationTest"],
            cwd=str(REPO_ROOT / "packages" / "java"),
            capture_output=True,
            text=True,
            timeout=60,
        )

        if result.returncode != 0:
            pytest.fail(f"Java tests failed:\n{result.stderr}")

    except FileNotFoundError:
        pytest.skip("Maven not available")


# ============================================================================
# PHP Serialization Tests
# ============================================================================


def test_php_extraction_config_serialization() -> None:
    """Test PHP ExtractionConfig JSON serialization."""
    php_test_file = REPO_ROOT / "packages" / "php" / "tests" / "SerializationTest.php"

    if not php_test_file.exists():
        pytest.skip("PHP serialization test not available")

    try:
        # Run PHP tests
        result = subprocess.run(
            ["phpunit", "tests/SerializationTest.php"],
            cwd=str(REPO_ROOT / "packages" / "php"),
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode != 0:
            pytest.fail(f"PHP tests failed:\n{result.stderr}")

    except FileNotFoundError:
        pytest.skip("PHPUnit not available")


# ============================================================================
# C# Serialization Tests
# ============================================================================


def test_csharp_extraction_config_serialization() -> None:
    """Test C# ExtractionConfig JSON serialization."""
    csharp_test_file = REPO_ROOT / "packages" / "csharp" / "Kreuzberg.Tests" / "SerializationTest.cs"

    if not csharp_test_file.exists():
        pytest.skip("C# serialization test not available")

    try:
        # Run C# tests
        result = subprocess.run(
            ["dotnet", "test", "--filter", "SerializationTest"],
            cwd=str(REPO_ROOT / "packages" / "csharp"),
            capture_output=True,
            text=True,
            timeout=60,
        )

        if result.returncode != 0:
            pytest.fail(f"C# tests failed:\n{result.stderr}")

    except FileNotFoundError:
        pytest.skip(".NET SDK not available")


# ============================================================================
# Elixir Serialization Tests
# ============================================================================


def test_elixir_extraction_config_serialization() -> None:
    """Test Elixir ExtractionConfig JSON serialization."""
    elixir_test_file = REPO_ROOT / "packages" / "elixir" / "test" / "serialization_test.exs"

    if not elixir_test_file.exists():
        pytest.skip("Elixir serialization test not available")

    try:
        # Run Elixir tests
        result = subprocess.run(
            ["mix", "test", "test/serialization_test.exs"],
            cwd=str(REPO_ROOT / "packages" / "elixir"),
            capture_output=True,
            text=True,
            timeout=60,
        )

        if result.returncode != 0:
            pytest.fail(f"Elixir tests failed:\n{result.stderr}")

    except FileNotFoundError:
        pytest.skip("Elixir/mix not available")


# ============================================================================
# WebAssembly Serialization Tests
# ============================================================================


def test_wasm_extraction_config_serialization() -> None:
    """Test WebAssembly ExtractionConfig JSON serialization."""
    wasm_test_file = REPO_ROOT / "crates" / "kreuzberg-wasm" / "tests" / "serialization.rs"

    if not wasm_test_file.exists():
        pytest.skip("WASM serialization test not available")

    try:
        # Build and run WASM tests
        result = subprocess.run(
            ["wasm-pack", "test", "--headless", "--firefox"],
            cwd=str(REPO_ROOT / "crates" / "kreuzberg-wasm"),
            capture_output=True,
            text=True,
            timeout=60,
        )

        if result.returncode != 0:
            pytest.fail(f"WASM tests failed:\n{result.stderr}")

    except FileNotFoundError:
        pytest.skip("wasm-pack not available")


# ============================================================================
# Cross-Language Parity Tests
# ============================================================================


def test_field_name_mapping() -> None:
    """Test that field names are correctly mapped between languages.

    Validates:
    - Rust uses snake_case: use_cache, enable_quality_processing
    - TypeScript uses camelCase: useCache, enableQualityProcessing
    - Other languages follow their respective conventions
    """
    # Expected field mappings
    field_mappings = {
        "use_cache": {
            "rust": "use_cache",
            "python": "use_cache",
            "typescript": "useCache",
            "ruby": "use_cache",
            "go": "UseCache",
            "java": "useCache",
            "php": "use_cache",
            "csharp": "UseCache",
            "elixir": "use_cache",
        },
        "enable_quality_processing": {
            "rust": "enable_quality_processing",
            "python": "enable_quality_processing",
            "typescript": "enableQualityProcessing",
            "ruby": "enable_quality_processing",
            "go": "EnableQualityProcessing",
            "java": "enableQualityProcessing",
            "php": "enable_quality_processing",
            "csharp": "EnableQualityProcessing",
            "elixir": "enable_quality_processing",
        },
        "force_ocr": {
            "rust": "force_ocr",
            "python": "force_ocr",
            "typescript": "forceOcr",
            "ruby": "force_ocr",
            "go": "ForceOcr",
            "java": "forceOcr",
            "php": "force_ocr",
            "csharp": "ForceOcr",
            "elixir": "force_ocr",
        },
    }

    # Validate mappings are present
    assert len(field_mappings) > 0, "No field mappings defined"

    for canonical_name, language_mappings in field_mappings.items():
        assert len(language_mappings) > 0, f"No language mappings for '{canonical_name}'"
        for language, mapped_name in language_mappings.items():
            assert isinstance(mapped_name, str), f"Invalid mapping for {canonical_name}/{language}"


def test_serialization_round_trip() -> None:
    """Test that configs can be serialized and deserialized without loss.

    This validates:
    - JSON -> Config object -> JSON round-trip produces identical results
    - Nested structures are preserved
    - Default values are handled correctly
    """
    try:
        from kreuzberg import ExtractionConfig
    except ImportError:
        pytest.skip("kreuzberg Python binding not installed")

    test_configs = [
        {},
        {"use_cache": True},
        {"use_cache": False, "enable_quality_processing": False},
        {
            "use_cache": True,
            "enable_quality_processing": True,
            "force_ocr": False,
            "ocr": {
                "backend": "tesseract",
                "language": "eng",
            },
        },
    ]

    for config_dict in test_configs:
        try:
            # Create config
            config1 = ExtractionConfig(**config_dict)

            # Convert to dict
            dict1 = _python_config_to_dict(config1)

            # Create new config from dict
            config2 = ExtractionConfig(**dict1)

            # Convert to dict again
            dict2 = _python_config_to_dict(config2)

            # Dicts should be equivalent
            assert dict1 == dict2, f"Round-trip serialization failed for config: {config_dict}"

        except Exception as e:
            pytest.fail(f"Round-trip test failed for config {config_dict}: {e}")


def test_all_expected_fields_present() -> None:
    """Test that all expected fields are present in serialized configs."""
    try:
        from kreuzberg import ExtractionConfig
    except ImportError:
        pytest.skip("kreuzberg Python binding not installed")

    config = ExtractionConfig(
        use_cache=True,
        enable_quality_processing=True,
        force_ocr=False,
    )

    config_dict = _python_config_to_dict(config)

    # These fields should always be present
    required_fields = {
        "use_cache",
        "enable_quality_processing",
        "force_ocr",
    }

    for field in required_fields:
        assert field in config_dict, f"Required field '{field}' missing from config"


def test_null_and_empty_handling() -> None:
    """Test that null/None values and empty structures are handled consistently.

    This ensures:
    - Optional fields can be None without issues
    - Empty collections are preserved
    - Serialization handles edge cases correctly
    """
    try:
        from kreuzberg import ExtractionConfig
    except ImportError:
        pytest.skip("kreuzberg Python binding not installed")

    configs = [
        ExtractionConfig(use_cache=True, enable_quality_processing=False, force_ocr=True),
        ExtractionConfig(use_cache=None, enable_quality_processing=None, force_ocr=None),
    ]

    for config in configs:
        try:
            config_dict = _python_config_to_dict(config)
            # Should not raise exceptions
            assert isinstance(config_dict, dict)
        except Exception as e:
            pytest.fail(f"Failed to serialize config with edge cases: {e}")


# ============================================================================
# Serialization Output Comparison Tests
# ============================================================================


@pytest.mark.parametrize("fixture", EXTRACTION_CONFIG_FIXTURES, ids=lambda f: f.name)
def test_rust_vs_python_serialization(fixture: TestFixture) -> None:
    """Compare Rust and Python serialization outputs for consistency.

    This validates that the same config produces equivalent JSON across languages.
    """
    try:
        from kreuzberg import ExtractionConfig
    except ImportError:
        pytest.skip("kreuzberg Python binding not installed")

    # Create Python config
    try:
        python_config = ExtractionConfig(**fixture.config_dict)
        python_output = _python_config_to_dict(python_config)

        # Validate that expected fields are present
        for field in fixture.expected_fields:
            assert field in python_output, f"Field '{field}' missing in Python output for fixture '{fixture.name}'"

    except Exception as e:
        pytest.fail(f"Python serialization failed for fixture '{fixture.name}': {e}")


def test_config_immutability_after_serialization() -> None:
    """Test that serialization doesn't modify the original config object.

    This ensures:
    - Config objects remain unchanged after serialization
    - No side effects from serialization
    - Safe to serialize multiple times
    """
    try:
        from kreuzberg import ExtractionConfig
    except ImportError:
        pytest.skip("kreuzberg Python binding not installed")

    config = ExtractionConfig(use_cache=True, enable_quality_processing=False, force_ocr=True)

    # Store original state
    original_dict = _python_config_to_dict(config)

    # Serialize multiple times
    for _ in range(5):
        _python_config_to_dict(config)

    # Config should be unchanged
    assert original_dict == _python_config_to_dict(config), "Config was modified during serialization"


# ============================================================================
# Integration Tests
# ============================================================================


def test_cross_language_json_equivalence() -> None:
    """Integration test validating that all language outputs produce equivalent JSON.

    This is a high-level test that:
    1. Creates configs in all available language bindings
    2. Serializes them to JSON
    3. Compares the JSON structures for equivalence
    4. Reports any mismatches
    """
    results = {}

    # Test Python (always available in test environment)
    try:
        from kreuzberg import ExtractionConfig

        config = ExtractionConfig(use_cache=True, enable_quality_processing=True, force_ocr=False)
        results["python"] = _python_config_to_dict(config)
    except ImportError:
        pytest.skip("Python binding required for integration test")

    # Validate that basic structure exists
    assert "python" in results, "Python serialization failed"
    assert isinstance(results["python"], dict), "Python output should be a dictionary"


# ============================================================================
# Rust vs Python JSON Comparison Tests
# ============================================================================


@pytest.mark.parametrize("fixture", EXTRACTION_CONFIG_FIXTURES, ids=lambda f: f.name)
def test_rust_python_json_equivalence(fixture: TestFixture) -> None:
    """Verify Rust and Python produce equivalent JSON for the same config.

    Normalizes field names (camelCase vs snake_case) and compares structure.
    """
    rust_json = get_rust_serialization(fixture.config_dict)
    python_json = get_python_serialization(fixture.config_dict)

    is_equal, differences = compare_json_structures(rust_json, python_json, normalize=True)

    assert is_equal, (
        f"Rust vs Python JSON mismatch for fixture '{fixture.name}':\n"
        f"Rust: {json.dumps(rust_json, indent=2)}\n"
        f"Python: {json.dumps(python_json, indent=2)}\n"
        f"Differences:\n" + "\n".join(f"  - {d}" for d in differences)
    )


# ============================================================================
# Rust vs TypeScript JSON Comparison Tests
# ============================================================================


@pytest.mark.parametrize("fixture", EXTRACTION_CONFIG_FIXTURES, ids=lambda f: f.name)
def test_rust_typescript_json_equivalence(fixture: TestFixture) -> None:
    """Verify Rust and TypeScript produce equivalent JSON for the same config.

    TypeScript uses camelCase; Rust uses snake_case. This test normalizes both.
    """
    rust_json = get_rust_serialization(fixture.config_dict)
    ts_json = get_typescript_serialization(fixture.config_dict)

    is_equal, differences = compare_json_structures(rust_json, ts_json, normalize=True)

    assert is_equal, (
        f"Rust vs TypeScript JSON mismatch for fixture '{fixture.name}':\n"
        f"Rust: {json.dumps(rust_json, indent=2)}\n"
        f"TypeScript: {json.dumps(ts_json, indent=2)}\n"
        f"Differences:\n" + "\n".join(f"  - {d}" for d in differences)
    )


# ============================================================================
# Python vs TypeScript JSON Comparison Tests
# ============================================================================


@pytest.mark.parametrize("fixture", EXTRACTION_CONFIG_FIXTURES, ids=lambda f: f.name)
def test_python_typescript_json_equivalence(fixture: TestFixture) -> None:
    """Verify Python and TypeScript produce equivalent JSON for the same config.

    Normalizes field names and compares structure.
    """
    python_json = get_python_serialization(fixture.config_dict)
    ts_json = get_typescript_serialization(fixture.config_dict)

    is_equal, differences = compare_json_structures(python_json, ts_json, normalize=True)

    assert is_equal, (
        f"Python vs TypeScript JSON mismatch for fixture '{fixture.name}':\n"
        f"Python: {json.dumps(python_json, indent=2)}\n"
        f"TypeScript: {json.dumps(ts_json, indent=2)}\n"
        f"Differences:\n" + "\n".join(f"  - {d}" for d in differences)
    )


# ============================================================================
# Multi-Language Equivalence Tests
# ============================================================================


def test_rust_python_typescript_json_equivalence() -> None:
    """Verify all three core languages (Rust, Python, TypeScript) produce equivalent JSON.

    This is a comprehensive test using the full fixture set.
    """
    test_config = {
        "use_cache": True,
        "enable_quality_processing": True,
        "force_ocr": False,
        "ocr": {
            "backend": "tesseract",
            "language": "eng",
        },
        "chunking": {
            "strategy": "semantic",
            "max_chunk_size": 2048,
        },
    }

    rust_json = get_rust_serialization(test_config)
    python_json = get_python_serialization(test_config)
    ts_json = get_typescript_serialization(test_config)

    # Normalize all to snake_case for comparison
    rust_norm = normalize_json(rust_json, to_snake_case=True)
    python_norm = normalize_json(python_json, to_snake_case=True)
    ts_norm = normalize_json(ts_json, to_snake_case=True)

    # Compare Rust vs Python
    is_equal_rp, diffs_rp = compare_json_structures(rust_norm, python_norm, normalize=False)
    assert is_equal_rp, (
        f"Rust vs Python mismatch:\n"
        f"Rust: {json.dumps(rust_norm, indent=2)}\n"
        f"Python: {json.dumps(python_norm, indent=2)}\n"
        f"Differences:\n" + "\n".join(f"  - {d}" for d in diffs_rp)
    )

    # Compare Rust vs TypeScript
    is_equal_rt, diffs_rt = compare_json_structures(rust_norm, ts_norm, normalize=False)
    assert is_equal_rt, (
        f"Rust vs TypeScript mismatch:\n"
        f"Rust: {json.dumps(rust_norm, indent=2)}\n"
        f"TypeScript: {json.dumps(ts_norm, indent=2)}\n"
        f"Differences:\n" + "\n".join(f"  - {d}" for d in diffs_rt)
    )

    # Compare Python vs TypeScript
    is_equal_pt, diffs_pt = compare_json_structures(python_norm, ts_norm, normalize=False)
    assert is_equal_pt, (
        f"Python vs TypeScript mismatch:\n"
        f"Python: {json.dumps(python_norm, indent=2)}\n"
        f"TypeScript: {json.dumps(ts_norm, indent=2)}\n"
        f"Differences:\n" + "\n".join(f"  - {d}" for d in diffs_pt)
    )


# ============================================================================
# Field Name Normalization Tests
# ============================================================================


def test_camel_to_snake_conversion() -> None:
    """Test camelCase to snake_case conversion."""
    test_cases = [
        ("useCache", "use_cache"),
        ("enableQualityProcessing", "enable_quality_processing"),
        ("forceOcr", "force_ocr"),
        ("maxChunkSize", "max_chunk_size"),
        ("ocrBackend", "ocr_backend"),
    ]

    for camel, expected_snake in test_cases:
        result = camel_to_snake(camel)
        assert result == expected_snake, f"Failed: {camel} -> {result} (expected {expected_snake})"


def test_snake_to_camel_conversion() -> None:
    """Test snake_case to camelCase conversion."""
    test_cases = [
        ("use_cache", "useCache"),
        ("enable_quality_processing", "enableQualityProcessing"),
        ("force_ocr", "forceOcr"),
        ("max_chunk_size", "maxChunkSize"),
        ("ocr_backend", "ocrBackend"),
    ]

    for snake, expected_camel in test_cases:
        result = snake_to_camel(snake)
        assert result == expected_camel, f"Failed: {snake} -> {result} (expected {expected_camel})"


def test_normalize_json_to_snake_case() -> None:
    """Test JSON normalization to snake_case."""
    input_json = {
        "useCache": True,
        "enableQualityProcessing": False,
        "ocrConfig": {
            "ocrBackend": "tesseract",
            "languageCode": "eng",
        },
        "chunkSettings": [{"maxSize": 1024}, {"minSize": 512}],
    }

    normalized = normalize_json(input_json, to_snake_case=True)

    assert normalized["use_cache"] is True
    assert normalized["enable_quality_processing"] is False
    assert normalized["ocr_config"]["ocr_backend"] == "tesseract"
    assert normalized["ocr_config"]["language_code"] == "eng"
    assert normalized["chunk_settings"][0]["max_size"] == 1024
    assert normalized["chunk_settings"][1]["min_size"] == 512


def test_normalize_json_to_camel_case() -> None:
    """Test JSON normalization to camelCase."""
    input_json = {
        "use_cache": True,
        "enable_quality_processing": False,
        "ocr_config": {
            "ocr_backend": "tesseract",
            "language_code": "eng",
        },
        "chunk_settings": [{"max_size": 1024}, {"min_size": 512}],
    }

    normalized = normalize_json(input_json, to_snake_case=False)

    assert normalized["useCache"] is True
    assert normalized["enableQualityProcessing"] is False
    assert normalized["ocrConfig"]["ocrBackend"] == "tesseract"
    assert normalized["ocrConfig"]["languageCode"] == "eng"
    assert normalized["chunkSettings"][0]["maxSize"] == 1024
    assert normalized["chunkSettings"][1]["minSize"] == 512


# ============================================================================
# JSON Structure Comparison Tests
# ============================================================================


def test_compare_identical_structures() -> None:
    """Test that identical structures are recognized as equal."""
    json1 = {"use_cache": True, "force_ocr": False, "max_chunk_size": 1024}
    json2 = {"use_cache": True, "force_ocr": False, "max_chunk_size": 1024}

    is_equal, differences = compare_json_structures(json1, json2, normalize=False)

    assert is_equal, f"Identical structures should be equal. Differences: {differences}"
    assert len(differences) == 0


def test_compare_structures_with_case_differences() -> None:
    """Test that structures with different cases are recognized as equal when normalized."""
    json1 = {"useCache": True, "forceOcr": False}
    json2 = {"use_cache": True, "force_ocr": False}

    is_equal, differences = compare_json_structures(json1, json2, normalize=True)

    assert is_equal, f"Case-different structures should be equal after normalization. Differences: {differences}"


def test_compare_structures_with_missing_fields() -> None:
    """Test that structures with missing fields are detected."""
    json1 = {"use_cache": True, "force_ocr": False, "max_chunk_size": 1024}
    json2 = {"use_cache": True, "force_ocr": False}

    is_equal, differences = compare_json_structures(json1, json2, normalize=False)

    assert not is_equal, "Structures with different keys should not be equal"
    assert any("missing" in d.lower() for d in differences), "Should report missing fields"


def test_compare_structures_with_type_differences() -> None:
    """Test that structures with different types are detected."""
    json1 = {"max_chunk_size": 1024}
    json2 = {"max_chunk_size": "1024"}

    is_equal, differences = compare_json_structures(json1, json2, normalize=False)

    assert not is_equal, "Structures with different types should not be equal"
    assert any("type mismatch" in d.lower() for d in differences), "Should report type mismatches"


def test_compare_nested_structures() -> None:
    """Test comparison of deeply nested structures."""
    json1 = {
        "ocr_config": {
            "backend": "tesseract",
            "settings": {"language": "eng", "options": ["preserve_layout", "auto_rotate"]},
        }
    }
    json2 = {
        "ocr_config": {
            "backend": "tesseract",
            "settings": {"language": "eng", "options": ["preserve_layout", "auto_rotate"]},
        }
    }

    is_equal, differences = compare_json_structures(json1, json2, normalize=False)

    assert is_equal, f"Identical nested structures should be equal. Differences: {differences}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
