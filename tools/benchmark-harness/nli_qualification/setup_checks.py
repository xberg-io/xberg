"""Check the NLI native setup contract without provisioning or loading a model."""

import re
import unittest
from pathlib import Path

PACKAGE = Path(__file__).resolve().parent
ROOT = PACKAGE.parents[2]


def version_sources(action: str, workflow: str) -> str:
    """Require the action default as the sole version source for main consumers."""
    default = re.search(r'(?ms)^  ort-version:\n(?:(?!^  [\w-]+:).)*?^    default: "([^"\n]+)"', action)
    steps = [
        step for step in re.split(r"(?m)^      - ", workflow) if "uses: ./.github/actions/setup-onnx-runtime" in step
    ]
    overrides = any(re.search(r"(?m)^\s+ort-version:", step) for step in steps)
    if len(steps) != 6:
        raise ValueError(f"expected exactly six main ORT action consumers, found {len(steps)}")
    if default is None or overrides or "ORT_VERSION" in workflow:
        raise ValueError("main ORT consumers must use the single native action default")
    return default[1]


def main() -> None:
    """Collect exactly six non-optimized setup checks and propagate failure."""
    if not __debug__:
        raise RuntimeError("optimized Python would remove setup test assertions")
    suite = unittest.defaultTestLoader.discover(str(PACKAGE / "tests"), pattern="test_setup_contract.py")
    collected = suite.countTestCases()
    if collected != 6:
        raise RuntimeError(f"expected exactly six NLI native setup tests, collected {collected}")
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful():
        raise SystemExit(1)


if __name__ == "__main__":
    main()
