"""Keep xberg-tesseract's build script from reporting routine progress as Cargo warnings."""

from pathlib import Path
import re
import unittest

BUILD_SCRIPT = Path(__file__).resolve().parents[2] / "crates/xberg-tesseract/build.rs"

# Each entry reports a failure or a fallback the user should see. Routine progress goes to stderr
# (`eprintln!`), because consumers such as the alef Swift gate fail on any `warning:` line.
FAILURE_WARNINGS = {
    "Could not resolve absolute path for MinGW",
    "Failed to create cache dir",
    "All {} attempts for {} exhausted on URL",
    "Download attempt {}/{} for {} failed",
    "Tesseract WASM patch not found at",
    "Patch command failed, will apply programmatic fixups.",
    "patch command not available",
    "Library {} not found!",
}


class TesseractBuildWarnings(unittest.TestCase):
    def test_only_failures_and_fallbacks_are_cargo_warnings(self) -> None:
        messages = re.findall(r'"cargo:warning=([^"\\]*)', BUILD_SCRIPT.read_text())
        assert len(messages) == len(FAILURE_WARNINGS), messages
        unexpected = [m for m in messages if not any(m.startswith(prefix) for prefix in FAILURE_WARNINGS)]
        assert unexpected == [], unexpected


if __name__ == "__main__":
    unittest.main()
