"""Opt-in controls use the real pinned evaluator, never native extraction."""

import importlib
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

# ruff: noqa: PT027


def _module(name: str):
    with patch.object(sys, "path", [str(Path(__file__).parents[1]), *sys.path]):
        return importlib.import_module("olmocr_bench." + name)


@unittest.skipUnless(
    os.environ.get("OLMOCR_DATA_ROOT")
    and os.environ.get("OLMOCR_EVALUATOR_ROOT")
    and os.environ.get("OLMOCR_BROWSER_ROOT"),
    "requires pinned external olmOCR corpus/evaluator/browser environment",
)
class OfficialControlsTest(unittest.TestCase):
    """Require known positive, empty, missing-file and math controls."""

    def _row(self, group: str, kind: str):
        path = Path(os.environ["OLMOCR_DATA_ROOT"]) / "bench_data" / f"{group}.jsonl"
        return next(json.loads(line) for line in path.read_text().splitlines() if json.loads(line)["type"] == kind)

    def _environment(self, temporary: str):
        return _module("official").evaluation_environment(
            Path(os.environ["OLMOCR_EVALUATOR_ROOT"]), Path(os.environ["OLMOCR_BROWSER_ROOT"]), Path(temporary) / "home"
        )

    def test_full_real_catalog_freezes_984_pdfs_and_6090_tests(self):
        """Raw baselines preserve official group assignment and exact coverage."""
        catalog = _module("catalog").load_catalog(Path(os.environ["OLMOCR_DATA_ROOT"]))
        assert len(catalog["pdfs"]) == 984
        assert len(catalog["tests"]) == 6090
        assert sum(row["group"] == "baseline" for row in catalog["tests"]) == 977
        assert sum(row["group"] == "headers_footers" for row in catalog["tests"]) == 760
        registry = os.environ.get("OLMOCR_REGISTRY")
        if registry:
            public = _module("catalog").load_catalog(Path(os.environ["OLMOCR_DATA_ROOT"]), Path(registry))
            assert len(public["pdfs"]) == 984
            assert public["tests"] == catalog["tests"] or {row["id"]: row for row in public["tests"]} == {
                row["id"]: row for row in catalog["tests"]
            }

    def test_official_positive_and_empty_prediction_controls(self):
        """A real present-text rule scores one then zero through official scoring."""
        row = self._row("long_tiny_text", "present")
        with tempfile.TemporaryDirectory() as temporary, self._environment(temporary):
            test = importlib.import_module("olmocr.bench.tests").load_single_test(row)
            candidate = Path(temporary) / "candidate"
            path = candidate / f"{Path(row['pdf']).with_suffix('')}_pg1_repeat1.md"
            path.parent.mkdir(parents=True)
            path.write_text(row["text"], encoding="utf-8")
            positive = _module("official").score_group(candidate, [test])
            assert positive["score"] == 1.0 and positive["tests"] == 1
            path.write_text("", encoding="utf-8")
            empty = _module("official").score_group(candidate, [test])
            assert empty["score"] == 0.0 and empty["tests"] == 1

    def test_missing_prediction_remains_a_counted_official_error(self):
        """Diagnostic force mode retains a missing job but cannot qualify it."""
        row = self._row("long_tiny_text", "present")
        with tempfile.TemporaryDirectory() as temporary, self._environment(temporary):
            test = importlib.import_module("olmocr.bench.tests").load_single_test(row)
            result = _module("official").score_group(Path(temporary), [test])
            assert result["tests"] == 1 and len(result["rows"]) == 1
            assert len(result["errors"]) == 1 and result["score"] == 0.0

    def test_official_nonidentical_math_controls_exercise_real_comparator(self):
        """Equivalent and changed equations traverse the real rendered comparator."""
        with tempfile.TemporaryDirectory() as temporary, self._environment(temporary):
            upstream = importlib.import_module("olmocr.bench.tests")
            test = upstream.load_single_test(
                {"id": "math-comparator", "pdf": "control.pdf", "page": 1, "type": "math", "math": "x + 1"}
            )
            assert test.reference_render.error is None
            with patch.object(
                upstream, "compare_rendered_equations", wraps=upstream.compare_rendered_equations
            ) as comparator:
                assert test.run("$$x+1$$")[0] is True
                assert comparator.call_count == 1
                assert test.run("$$x+2$$")[0] is False
                assert comparator.call_count == 2
                assert all(argument.error is None for argument in comparator.call_args.args)

    def test_math_reference_render_error_is_retained_as_initialization_error(self):
        """A real KaTeX parse failure cannot make the two-row control ready."""
        bad = {"id": "bad-reference", "pdf": "control.pdf", "page": 1, "type": "math", "math": r"\notarealcommand{X}"}
        good = {"id": "good-reference", "pdf": "control.pdf", "page": 1, "type": "math", "math": "x+1"}
        with tempfile.TemporaryDirectory() as temporary, self._environment(temporary):
            upstream = importlib.import_module("olmocr.bench.tests")
            constructed = upstream.load_single_test(bad)
            assert constructed.reference_render is not None
            assert constructed.reference_render.error
            official = _module("official")
            with patch.object(official, "TEST_COUNT", 2):
                tests, rows = official.load_official([bad, good])
            assert len(rows) == 2
            by_id = {row["id"]: row for row in rows}
            assert by_id["good-reference"]["status"] == "initialized"
            assert by_id["bad-reference"]["status"] == "initialization_error"
            assert "reference render" in by_id["bad-reference"]["error"]
            assert len(tests) == 1 and tests[0].id == "good-reference"


if __name__ == "__main__":
    unittest.main()
