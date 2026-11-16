"""Fail-closed controls use synthetic integrity bytes, never extraction evidence."""

import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

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


class FormulaCacheTest(unittest.TestCase):
    """Only explicitly bound formula files belong in each fresh cache."""

    def setUp(self):
        """Create a temporary model store and a synthetic producer plan."""
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        with patch.object(sys, "path", [str(Path(__file__).parents[1]), *sys.path]):
            self.runner = importlib.import_module("olmocr_bench.runner")
            contracts = importlib.import_module("olmocr_bench.contracts")

        self.model_dir = self.root / "models"
        self.model_dir.mkdir()
        names = ("image_resizer.onnx", "encoder.onnx", "decoder.onnx", "tokenizer.json")
        self.models = [self._bind(self.model_dir / name, name.encode()) for name in names]
        self.pins = {Path(row["file"]).name: row["sha256"] for row in self.models}
        self.enterContext(patch.object(contracts, "FORMULA_MODEL_SHA256", self.pins, create=True))
        self.pdf = self._bind(self.root / "synthetic.pdf", b"synthetic PDF identity")
        self.pdf["file"] = "synthetic.pdf"
        self.plan = {
            "producers": {"baseline": {"executable": {"file": "/unused/synthetic-producer"}}},
            "config": {"file": "/unused/synthetic-config"},
            "data_root": str(self.root),
            "models": [*self.models, self._bind(self.model_dir / "unrelated.onnx", b"unrelated")],
            "argv": ["extract", "{pdf}", "--config", "{config}", "--format", "json"],
            "environment": {"HF_HUB_CACHE": str(self.model_dir)},
        }

    @staticmethod
    def _bind(path, data):
        path.write_bytes(data)
        return {"file": str(path), "sha256": hashlib.sha256(data).hexdigest()}

    def _extract(self):
        return self.runner._extract(self.plan, {"role": "baseline"}, self.pdf, self.root, 1)

    def test_formula_model_links_are_isolated_per_extraction(self):
        """Separate invocations reuse only the formula files and forget result state."""
        caches = []

        def produce(*_args, env, **_kwargs):
            cache = Path(env["XBERG_CACHE_DIR"])
            assert not (cache / "previous-result").exists()
            formula = cache / "formula-recognition"
            assert not formula.is_symlink()
            assert {path.name for path in formula.iterdir()} == self.pins.keys()
            for path in formula.iterdir():
                assert path.is_symlink()
                assert path.readlink() == self.model_dir / path.name
                assert path.read_bytes() == path.name.encode()
            assert env["HF_HUB_CACHE"] == str(self.model_dir)
            (cache / "previous-result").touch()
            caches.append(cache)
            return subprocess.CompletedProcess([], 0, json.dumps({"result": {"content": "synthetic"}}).encode(), b"")

        with patch.object(self.runner.subprocess, "run", side_effect=produce) as invoke:
            for _ in range(2):
                assert self._extract()[0] == "synthetic"
            assert invoke.call_count == 2
        assert len(set(caches)) == 2
        assert not any(path.exists() for path in caches)
        assert {path.name for path in self.model_dir.iterdir()} == self.pins.keys() | {"unrelated.onnx"}

    def test_bad_formula_layout_fails_before_launching_the_producer(self):
        """A producer never starts when a formula path is invalid or ambiguous."""
        for models in (
            self.models[1:],
            [*self.models[1:], dict(self.models[0], sha256="a" * 64)],
            [*self.models, self.models[0]],
            [*self.models[1:], dict(self.models[0], file="relative/image_resizer.onnx")],
        ):
            with self.subTest(models=models), patch.object(self.runner.subprocess, "run") as invoke:
                self.plan["models"] = models
                with self.assertRaisesRegex(ValueError, "exactly one pinned RapidLaTeXOCR image_resizer.onnx"):
                    self._extract()
                invoke.assert_not_called()

        self.plan["models"] = self.models
        Path(self.models[0]["file"]).unlink()
        with patch.object(self.runner.subprocess, "run") as invoke:
            with self.assertRaisesRegex(ValueError, "missing pinned RapidLaTeXOCR file"):
                self._extract()
            invoke.assert_not_called()


if __name__ == "__main__":
    unittest.main()
