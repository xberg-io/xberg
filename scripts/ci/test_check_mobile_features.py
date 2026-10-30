"""Contract tests for the resolved mobile feature-graph gate."""

# ruff: noqa: PT027 -- The mobile CI gate runs with Python's standard library only.

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "check_mobile_features", Path(__file__).with_name("check-mobile-features.py")
)
assert SPEC is not None and SPEC.loader is not None
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)

SAFE_GRAPH = "xberg-dart v1.4.0 (/checkout/packages/dart/rust)\nxberg v1.4.0\ntract-onnx v0.23.9\n"


class MobileFeatureGraphTests(unittest.TestCase):
    def test_accepts_a_resolved_binding_with_core_and_tract(self) -> None:
        assert GATE.validate_graph(SAFE_GRAPH, "xberg-dart") == {"xberg-dart", "xberg", "tract-onnx"}

    def test_rejects_native_ort_even_when_positive_controls_are_present(self) -> None:
        graph = SAFE_GRAPH + "ort v2.0.0-rc.14\nort-sys v2.0.0-rc.14\n"
        with self.assertRaisesRegex(ValueError, "native ORT leaked into mobile defaults: ort, ort-sys"):
            GATE.validate_graph(graph, "xberg-dart")

    def test_rejects_empty_cargo_output(self) -> None:
        with self.assertRaisesRegex(ValueError, "returned no resolved packages"):
            GATE.validate_graph("", "xberg-dart")

    def test_rejects_an_unrelated_resolved_graph(self) -> None:
        with self.assertRaisesRegex(ValueError, "requested binding is absent"):
            GATE.validate_graph(SAFE_GRAPH, "xberg-swift")

    def test_rejects_a_graph_without_the_core_package(self) -> None:
        with self.assertRaisesRegex(ValueError, "required mobile packages are missing: xberg"):
            GATE.validate_graph(SAFE_GRAPH.replace("xberg v1.4.0\n", ""), "xberg-dart")

    def test_rejects_a_graph_without_the_tract_backend(self) -> None:
        with self.assertRaisesRegex(ValueError, "required mobile packages are missing: tract-onnx"):
            GATE.validate_graph(SAFE_GRAPH.replace("tract-onnx v0.23.9\n", ""), "xberg-dart")

    def test_rejects_non_graph_output_rather_than_passing_zero_packages(self) -> None:
        with self.assertRaisesRegex(ValueError, "returned no resolved packages"):
            GATE.validate_graph("warning: nothing to print.\n", "xberg-dart")


if __name__ == "__main__":
    unittest.main()
