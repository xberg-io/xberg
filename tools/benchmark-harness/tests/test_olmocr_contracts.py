"""Fail-closed controls use synthetic integrity bytes, never extraction evidence."""

import hashlib
import importlib.util
import tempfile
import unittest
from pathlib import Path

# ruff: noqa: PT027


def _contracts():
    path = Path(__file__).parents[1] / "olmocr_bench" / "contracts.py"
    spec = importlib.util.spec_from_file_location("olmocr_contracts", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _config():
    return {
        "output_format": "markdown",
        "use_cache": False,
        "disable_ocr": False,
        "use_layout_for_markdown": True,
        "layout": {"strategy": "always", "formula_model": "latex_ocr", "acceleration": {"provider": "cpu"}},
        "concurrency": {"max_threads": 1},
        "ocr": {"tesseract_config": {"language": "eng", "use_cache": False}},
    }


class ContractsTest(unittest.TestCase):
    """Count actual jobs and preserve the official baseline grouping."""

    def test_enabled_layout_formula_configuration_is_supported(self):
        """The negative configuration controls have a valid positive counterpart."""
        assert _contracts().validate_config(_config()) is None

    def test_extra_prediction_repeat_cannot_change_frozen_scoring(self):
        """The official glob must not silently pick up an unplanned second repeat."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first = root / "page_pg1_repeat1.md"
            first.write_text("synthetic integrity control")
            assets = [{"file": str(first), "sha256": hashlib.sha256(first.read_bytes()).hexdigest()}]
            _contracts().verify_predictions(root, assets)
            (root / "page_pg1_repeat2.md").write_text("unexpected repeat")
            with self.assertRaisesRegex(ValueError, "prediction inventory"):
                _contracts().verify_predictions(root, assets)

    def test_generated_baselines_do_not_reclassify_explicit_header_baselines(self):
        """Existing baselines stay in their original JSONL reporting group."""
        rows = [{"id": "existing", "pdf": "headers/a.pdf", "page": 1, "type": "baseline", "group": "headers_footers"}]
        result = _contracts().inject_baselines(rows, ["headers/a.pdf", "math/b.pdf"])
        assert len(result) == 2
        assert result[0]["group"] == "headers_footers"
        assert result[1] == {
            "id": "math/b.pdf_baseline",
            "pdf": "math/b.pdf",
            "page": 1,
            "type": "baseline",
            "group": "baseline",
        }

    def test_duplicate_test_ids_cannot_replace_missing_tests(self):
        """Identical IDs are invalid even across separate groups."""
        row = {"id": "duplicate", "pdf": "a.pdf", "page": 1, "type": "present"}
        with self.assertRaisesRegex(ValueError, "duplicate test"):
            _contracts().inject_baselines([row, row], ["a.pdf"])

    def test_unknown_pdf_reference_is_not_filtered(self):
        """An unresolved public test reference must stop qualification."""
        with self.assertRaisesRegex(ValueError, "PDF reference"):
            _contracts().inject_baselines([{"id": "bad", "pdf": "absent.pdf"}], ["a.pdf"])

    def test_complete_schedule_has_1968_unique_generation_cases(self):
        """Every one of 984 PDFs runs for both producers."""
        result = _contracts().schedule([{"id": str(index)} for index in range(984)])
        assert len(result) == 1968
        assert len({row["id"] for row in result}) == 1968

    def test_empty_schedule_cannot_pass(self):
        """No data must not produce a successful empty sweep."""
        with self.assertRaisesRegex(ValueError, "empty"):
            _contracts().schedule([])

    def test_missing_native_markdown_layout_switch_is_rejected(self):
        """Some(layout) alone does not activate native-PDF Markdown layout."""
        config = _config()
        config["use_layout_for_markdown"] = False
        with self.assertRaisesRegex(ValueError, "use_layout_for_markdown"):
            _contracts().validate_config(config)

    def test_disabled_formula_model_is_rejected(self):
        """LaTeX math measurements require the actual formula model setting."""
        config = _config()
        config["layout"]["formula_model"] = None
        with self.assertRaisesRegex(ValueError, "latex_ocr"):
            _contracts().validate_config(config)

    def test_nested_ocr_cache_cannot_be_implicit(self):
        """Both cache levels must be explicitly disabled."""
        config = _config()
        config["ocr"]["tesseract_config"].pop("use_cache")
        with self.assertRaisesRegex(ValueError, "OCR cache"):
            _contracts().validate_config(config)

    def test_canonical_cli_invocation_is_supported(self):
        """Canonical extraction binds one config and the actual scheduled input."""
        assert _contracts().validate_argv(["extract", "{pdf}", "--config", "{config}", "--format", "json"]) is None

    def test_cli_overrides_cannot_disable_frozen_settings(self):
        """Inline config overrides take precedence over the frozen file in the CLI."""
        with self.assertRaisesRegex(ValueError, "canonical"):
            _contracts().validate_argv(["extract", "{pdf}", "--config-json", '{"layout":null}'])

    def test_missing_result_cannot_look_complete(self):
        """Missing extraction rows remain a failed coverage gate."""
        with self.assertRaisesRegex(ValueError, "missing"):
            _contracts().validate_results([{"id": "a"}, {"id": "b"}], [{"id": "a"}])

    def test_duplicate_result_cannot_fill_missing_case(self):
        """Repeated rows cannot inflate the generation count."""
        with self.assertRaisesRegex(ValueError, "duplicate"):
            _contracts().validate_results([{"id": "a"}, {"id": "b"}], [{"id": "a"}, {"id": "a"}])

    def test_model_mutation_after_preflight_is_detected(self):
        """Models remain bound throughout generation and evaluation."""
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "model"
            path.write_bytes(b"original synthetic integrity bytes")
            asset = {"file": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
            _contracts().verify_assets([asset])
            path.write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "artifact changed"):
                _contracts().verify_assets([asset])

    def test_snapshot_is_independent_of_mutable_producer(self):
        """Later Cargo builds cannot change a frozen executable."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "producer"
            path.write_bytes(b"original integrity bytes")
            result = _contracts().snapshot(path, root / "assets", "baseline", executable=True)
            path.write_bytes(b"rebuilt")
            frozen = Path(result["file"])
            assert frozen.read_bytes() == b"original integrity bytes"
            assert frozen.stat().st_mode & 0o222 == 0


if __name__ == "__main__":
    unittest.main()
