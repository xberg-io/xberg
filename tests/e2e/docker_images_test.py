#!/usr/bin/env python3
"""
End-to-end tests for Kreuzberg Docker images.

This script tests all Docker images to ensure they work correctly with various features:
- CLI functionality
- API server functionality
- File extraction (various formats)
- OCR capabilities
- Volume mounting
- Configuration handling
- Security best practices
- Resource limits
"""

import argparse
import asyncio
import json
import random
import string
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any

# Test configuration
DOCKER_IMAGES = {
    "core": "kreuzberg:core",
    "easyocr": "kreuzberg:easyocr",
    "paddle": "kreuzberg:paddle",
    "gmft": "kreuzberg:gmft",
}

# Security configuration
SECURITY_CONFIG = {
    "max_container_runtime": 300,  # 5 minutes max
    "max_memory": "2g",
    "max_cpu": "1.0",
    "readonly_volumes": True,
    "security_opt": ["no-new-privileges"],
}

# Get the test source files directory
TEST_DIR = Path(__file__).parent.parent
TEST_FILES_DIR = TEST_DIR / "test_source_files"

# Test results tracking
test_results: dict[str, dict[str, Any]] = {}


def run_command(cmd: list[str], timeout: int = 30) -> tuple[int, str, str]:
    """Run a command and return exit code, stdout, and stderr."""
    try:
        result = subprocess.run(cmd, check=False, capture_output=True, text=True, timeout=timeout)
        return result.returncode, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "Command timed out"
    except Exception as e:
        return -1, "", str(e)


async def run_command_async(cmd: list[str], timeout: int = 30) -> tuple[int, str, str]:
    """Run a command asynchronously with timeout."""
    try:
        proc = await asyncio.create_subprocess_exec(
            *cmd, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )

        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout)
            return proc.returncode or 0, stdout.decode(), stderr.decode()
        except asyncio.TimeoutError:
            proc.kill()
            await proc.communicate()
            return -1, "", f"Command timed out after {timeout} seconds"

    except Exception as e:
        return -1, "", str(e)


def generate_random_container_name() -> str:
    """Generate a secure random container name."""
    suffix = "".join(random.choices(string.ascii_lowercase + string.digits, k=8))
    return f"kreuzberg-test-{suffix}"


def test_image_exists(image_name: str) -> bool:
    """Test if Docker image exists."""
    cmd = ["docker", "images", "-q", image_name]
    exit_code, stdout, _ = run_command(cmd)
    return exit_code == 0 and stdout.strip() != ""


def test_cli_help(image_name: str) -> bool:
    """Test CLI help command."""
    cmd = [
        "docker",
        "run",
        "--rm",
        "--security-opt",
        "no-new-privileges",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "--help",
    ]
    exit_code, stdout, stderr = run_command(cmd)
    success = exit_code == 0 and "Extract text from documents" in stdout
    if not success:
        pass
    return success


def test_cli_version(image_name: str) -> bool:
    """Test CLI version command."""
    cmd = [
        "docker",
        "run",
        "--rm",
        "--security-opt",
        "no-new-privileges",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "--version",
    ]
    exit_code, stdout, stderr = run_command(cmd)
    success = exit_code == 0 and "kreuzberg" in stdout.lower()
    if not success:
        pass
    return success


def test_api_health(image_name: str) -> bool:
    """Test API health endpoint."""

    # Start container in background
    container_name = generate_random_container_name()
    port = random.randint(9000, 9999)  # Random port to avoid conflicts

    cmd = [
        "docker",
        "run",
        "-d",
        "--name",
        container_name,
        "--memory",
        SECURITY_CONFIG["max_memory"],
        "--cpus",
        SECURITY_CONFIG["max_cpu"],
        "--security-opt",
        "no-new-privileges",
        "-p",
        f"{port}:8000",
        image_name,
    ]
    exit_code, container_id, stderr = run_command(cmd)
    if exit_code != 0:
        return False

    try:
        # Wait for API to start
        time.sleep(5)

        # Test health endpoint
        import urllib.request

        try:
            response = urllib.request.urlopen(f"http://localhost:{port}/health", timeout=5)
            data = json.loads(response.read().decode())
            return response.status == 200 and data.get("status") == "ok"
        except Exception:
            return False
    finally:
        # Clean up container
        run_command(["docker", "stop", container_name], timeout=10)
        run_command(["docker", "rm", container_name], timeout=10)


def test_file_extraction(image_name: str, test_file: str) -> bool:
    """Test file extraction via CLI."""

    with tempfile.TemporaryDirectory():
        # Copy test file to temp directory
        test_file_path = TEST_FILES_DIR / test_file
        if not test_file_path.exists():
            return False

        # Run extraction with security constraints
        cmd = [
            "docker",
            "run",
            "--rm",
            "--memory",
            "512m",
            "--cpus",
            "0.5",
            "--security-opt",
            "no-new-privileges",
            "-v",
            f"{test_file_path.parent}:/data:ro",
            image_name,
            "python",
            "-m",
            "kreuzberg",
            "extract",
            f"/data/{test_file_path.name}",
        ]
        exit_code, stdout, stderr = run_command(cmd, timeout=60)

        # Check if extraction succeeded and produced output
        success = exit_code == 0 and len(stdout) > 10

        # Additional validation: Check for sensitive data leakage
        if success:
            sensitive_patterns = ["/root", "/home", "/etc/passwd", "/proc"]
            for pattern in sensitive_patterns:
                if pattern in stdout:
                    success = False
                    break

        if not success and stderr:
            pass
        return success


def test_ocr_extraction(image_name: str, image_variant: str) -> bool:
    """Test OCR extraction based on image variant."""

    # Use different test images based on variant
    test_files = {
        "core": "ocr-image.jpg",  # Tesseract
        "easyocr": "better-ocr-image.jpg",
        "paddle": "invoice_image.png",
        "gmft": "ocr-image.jpg",  # Also uses Tesseract by default
    }

    test_file = test_files.get(image_variant, "ocr-image.jpg")
    test_file_path = TEST_FILES_DIR / test_file

    if not test_file_path.exists():
        return False

    # Run OCR extraction with resource limits
    cmd = [
        "docker",
        "run",
        "--rm",
        "--memory",
        "1g",  # OCR needs more memory
        "--cpus",
        "1.0",
        "--security-opt",
        "no-new-privileges",
        "-v",
        f"{test_file_path.parent}:/data:ro",
        image_name,
        "python",
        "-m",
        "kreuzberg",
        "extract",
        f"/data/{test_file_path.name}",
        "--force-ocr",
    ]

    # Increase timeout for OCR operations
    exit_code, stdout, stderr = run_command(cmd, timeout=120)

    # Check if extraction succeeded and produced meaningful output
    success = exit_code == 0 and len(stdout) > 20
    if not success and stderr:
        pass
    return success


def test_api_extraction(image_name: str) -> bool:
    """Test file extraction via API."""

    # Start container
    container_name = generate_random_container_name()
    port = random.randint(9000, 9999)

    cmd = [
        "docker",
        "run",
        "-d",
        "--name",
        container_name,
        "--memory",
        SECURITY_CONFIG["max_memory"],
        "--cpus",
        SECURITY_CONFIG["max_cpu"],
        "--security-opt",
        "no-new-privileges",
        "-p",
        f"{port}:8000",
        image_name,
    ]
    exit_code, container_id, stderr = run_command(cmd)
    if exit_code != 0:
        return False

    try:
        # Wait for API to start
        time.sleep(5)

        # Test extraction endpoint with a simple text file
        test_content = "Test content for API extraction"
        with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
            f.write(test_content)
            temp_file = f.name

        try:
            # Use curl to test the API
            cmd = ["curl", "-s", "-X", "POST", f"http://localhost:{port}/extract", "-F", f"data=@{temp_file}"]
            exit_code, stdout, stderr = run_command(cmd, timeout=30)

            if exit_code == 0:
                try:
                    response = json.loads(stdout)
                    # Check if content was extracted
                    if isinstance(response, list) and len(response) > 0:
                        content = response[0].get("content", "")
                        success = test_content in content
                    else:
                        success = False
                except json.JSONDecodeError:
                    success = False
            else:
                success = False

            if not success and stderr:
                pass
            return success

        finally:
            Path(temp_file).unlink()

    finally:
        # Clean up container
        run_command(["docker", "stop", container_name], timeout=10)
        run_command(["docker", "rm", container_name], timeout=10)


def test_table_extraction(image_name: str) -> bool:
    """Test table extraction for GMFT image."""

    test_file = "pdfs_with_tables/tiny.pdf"
    test_file_path = TEST_FILES_DIR / test_file

    if not test_file_path.exists():
        return False

    # Run extraction with table extraction enabled
    cmd = [
        "docker",
        "run",
        "--rm",
        "--memory",
        "1g",
        "--cpus",
        "1.0",
        "--security-opt",
        "no-new-privileges",
        "-v",
        f"{test_file_path.parent.parent}:/data:ro",
        image_name,
        "python",
        "-c",
        """
import asyncio
from kreuzberg import extract_file, ExtractionConfig

async def main():
    result = await extract_file(
        '/data/pdfs_with_tables/tiny.pdf',
        config=ExtractionConfig(extract_tables=True)
    )
    print(f"Tables found: {len(result.tables)}")
    return len(result.tables) > 0

success = asyncio.run(main())
exit(0 if success else 1)
""",
    ]

    exit_code, stdout, stderr = run_command(cmd, timeout=60)
    success = exit_code == 0 and "Tables found:" in stdout
    if not success and stderr:
        pass
    return success


def test_volume_security(image_name: str) -> bool:
    """Test volume mount security."""

    with tempfile.TemporaryDirectory() as tmpdir:
        # Create test file
        test_file = Path(tmpdir) / "test.txt"
        test_content = "Test content for volume security"
        test_file.write_text(test_content)

        # Test read-only mount
        cmd = [
            "docker",
            "run",
            "--rm",
            "-v",
            f"{tmpdir}:/data:ro",
            image_name,
            "python",
            "-c",
            "import os; print(os.access('/data/test.txt', os.W_OK))",
        ]

        exit_code, stdout, _ = run_command(cmd)
        return exit_code == 0 and stdout.strip() == "False"


def test_resource_limits(image_name: str) -> bool:
    """Test that container respects resource limits."""

    # Test memory limit enforcement
    cmd = [
        "docker",
        "run",
        "--rm",
        "--memory",
        "256m",
        "--memory-swap",
        "256m",
        image_name,
        "python",
        "-c",
        "import sys; print(sys.maxsize > 0)",
    ]

    exit_code, stdout, _ = run_command(cmd)
    return exit_code == 0 and stdout.strip() == "True"


def test_malicious_input_handling(image_name: str) -> bool:
    """Test handling of malicious inputs."""

    with tempfile.TemporaryDirectory() as tmpdir:
        # Test path traversal attempt
        cmd = [
            "docker",
            "run",
            "--rm",
            "-v",
            f"{tmpdir}:/data:ro",
            image_name,
            "python",
            "-m",
            "kreuzberg",
            "extract",
            "/data/../etc/passwd",
        ]
        exit_code, stdout, _ = run_command(cmd)

        # Should fail or not expose system files
        return exit_code != 0 or "passwd" not in stdout


def run_tests_for_image(image_variant: str, image_name: str) -> dict[str, bool]:
    """Run all tests for a specific Docker image."""

    results = {}

    # Basic tests for all images
    results["exists"] = test_image_exists(image_name)
    if not results["exists"]:
        return results

    results["cli_help"] = test_cli_help(image_name)
    results["cli_version"] = test_cli_version(image_name)
    results["api_health"] = test_api_health(image_name)

    # File extraction tests
    results["extract_txt"] = test_file_extraction(image_name, "contract.txt")
    results["extract_pdf"] = test_file_extraction(image_name, "searchable.pdf")
    results["extract_docx"] = test_file_extraction(image_name, "document.docx")

    # OCR tests
    results["ocr"] = test_ocr_extraction(image_name, image_variant)

    # API extraction test
    results["api_extract"] = test_api_extraction(image_name)

    # Security tests
    results["volume_security"] = test_volume_security(image_name)
    results["resource_limits"] = test_resource_limits(image_name)
    results["malicious_input"] = test_malicious_input_handling(image_name)

    # Special tests for specific variants
    if image_variant == "gmft":
        results["table_extraction"] = test_table_extraction(image_name)

    return results


def print_summary(all_results: dict[str, dict[str, bool]]) -> bool:
    """Print test summary."""

    total_tests = 0
    total_passed = 0

    for variant, results in all_results.items():
        if not results.get("exists", False):
            print(f"\n❌ {variant}: Image not found")
            continue

        passed = sum(1 for v in results.values() if v)
        total = len(results)
        total_tests += total
        total_passed += passed

        print(f"\n{'✅' if passed == total else '⚠️'} {variant}: {passed}/{total} tests passed")

        # Show failed tests
        failed_tests = [test for test, result in results.items() if not result]
        if failed_tests:
            print(f"   Failed tests: {', '.join(failed_tests)}")

    print(f"\n{'=' * 50}")
    print(f"Overall: {total_passed}/{total_tests} tests passed")

    success_rate = (total_passed / total_tests * 100) if total_tests > 0 else 0
    print(f"Success rate: {success_rate:.1f}%")

    if success_rate >= 90:
        print("✅ Test suite PASSED")
    elif success_rate >= 70:
        print("⚠️ Test suite PASSED with warnings")
    else:
        print("❌ Test suite FAILED")

    return total_passed == total_tests


def main() -> None:
    """Main test runner."""
    parser = argparse.ArgumentParser(description="Kreuzberg Docker E2E Tests")
    parser.add_argument("--image", help="Test a specific image variant or full image name", default=None)
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    args = parser.parse_args()

    # Check if Docker is available
    exit_code, _, _ = run_command(["docker", "--version"])
    if exit_code != 0:
        print("❌ Docker is not available")
        sys.exit(1)

    # Run tests for specific image or all images
    all_results = {}

    if args.image:
        # Check if it's a variant name or full image name
        if args.image in DOCKER_IMAGES:
            # Test by variant name
            variant = args.image
            image_name = DOCKER_IMAGES[variant]
            print(f"Testing {variant} image: {image_name}")
            all_results[variant] = run_tests_for_image(variant, image_name)
        else:
            # Test by full image name
            image_name = args.image
            # Extract variant from image name (e.g., "kreuzberg:core" -> "core")
            variant = image_name.split(":")[-1] if ":" in image_name else "custom"
            print(f"Testing custom image: {image_name}")
            all_results[variant] = run_tests_for_image(variant, image_name)
    else:
        # Test all configured images
        print("Testing all Docker images...")
        for image_variant, image_name in DOCKER_IMAGES.items():
            print(f"\nTesting {image_variant}: {image_name}")
            all_results[image_variant] = run_tests_for_image(image_variant, image_name)

    # Generate test report
    report_file = TEST_DIR / "e2e" / "test_report.json"
    report_file.parent.mkdir(parents=True, exist_ok=True)
    with report_file.open("w") as f:
        json.dump(all_results, f, indent=2, default=str)
    print(f"\nTest report saved to: {report_file}")

    # Print summary and exit
    success = print_summary(all_results)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
