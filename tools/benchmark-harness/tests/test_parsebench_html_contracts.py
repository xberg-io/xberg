"""Fail-closed controls; synthetic assets never establish extraction quality."""

import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

# ruff: noqa: PT027


def _load_contracts():
    path = Path(__file__).parents[1] / "parsebench_html" / "contracts.py"
    spec = importlib.util.spec_from_file_location("parsebench_contracts", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


CONTRACTS = _load_contracts()


def _asset(root: Path, name: str, content: bytes) -> dict:
    (root / name).write_bytes(content)
    return {"file": name, "bytes": len(content), "sha256": hashlib.sha256(content).hexdigest()}


def _stage(root: Path) -> dict:
    document = {
        "id": "opaque-1",
        "pdf": _asset(root, "input.pdf", b"%PDF-1.4\nsynthetic integrity payload"),
        "oracle": _asset(root, "oracle.html", b"<table><tr><th>Key</th></tr><tr><td>Value</td></tr></table>"),
        "upstream": {"id": "synthetic-1", "tags": []},
        "curation": {"gate_verdict": "ACCEPT"},
    }
    value = {
        "dataset_revision": CONTRACTS.DATASET_REVISION,
        "evaluator_revision": CONTRACTS.EVALUATOR_REVISION,
        "records": [document],
    }
    (root / "stage.json").write_text(json.dumps(value), encoding="utf-8")
    return value


def _cases() -> list[dict]:
    return [{"id": "baseline/native_html/opaque-1"}, {"id": "candidate/native_html/opaque-1"}]


def _layout_config() -> dict:
    return {
        "output_format": "html",
        "use_cache": False,
        "layout": {"table_model": "tatr", "strategy": "always", "acceleration": {"provider": "cpu"}},
        "concurrency": {"max_threads": 1},
        "ocr": {"tesseract_config": {"use_cache": False}},
    }


class ContractsTest(unittest.TestCase):
    """Prove that zero-work and corrupt-artifact sweeps cannot pass."""

    def test_valid_stage_retains_original_oracle_bytes(self) -> None:
        """Valid stage retains original oracle bytes."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            expected = _stage(root)
            assert CONTRACTS.read_stage(root, 1) == expected

    def test_changed_pdf_cannot_pass_a_saved_stage(self) -> None:
        """Changed pdf cannot pass a saved stage."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _stage(root)
            (root / "input.pdf").write_bytes(b"changed bytes")
            with self.assertRaisesRegex(ValueError, "digest mismatch"):
                CONTRACTS.read_stage(root, 1)

    def test_duplicate_records_do_not_reduce_work(self) -> None:
        """Duplicate records do not reduce work."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            value = _stage(root)
            value["records"] *= 2
            (root / "stage.json").write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "duplicate record"):
                CONTRACTS.read_stage(root, 2)

    def test_missing_oracle_is_not_an_evaluable_record(self) -> None:
        """Missing oracle bytes fail before any case can be scheduled."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _stage(root)
            (root / "oracle.html").unlink()
            with self.assertRaises(FileNotFoundError):
                CONTRACTS.read_stage(root, 1)

    def test_empty_oracle_cannot_pass_even_with_a_matching_hash(self) -> None:
        """An intact but empty oracle is not a quality denominator."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            value = _stage(root)
            value["records"][0]["oracle"] = _asset(root, "oracle.html", b"")
            (root / "stage.json").write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "missing raw HTML table oracle"):
                CONTRACTS.read_stage(root, 1)

    def test_empty_corpus_fails_before_scheduling(self) -> None:
        """Empty corpus fails before scheduling."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            value = _stage(root)
            value["records"] = []
            (root / "stage.json").write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "record count"):
                CONTRACTS.read_stage(root, 503)

    def test_whole_comparison_has_exactly_3018_unique_cases(self) -> None:
        """Whole comparison has exactly 3018 unique cases."""
        records = [{"id": f"opaque-{index:03}"} for index in range(503)]
        result = CONTRACTS.schedule(records, ["baseline", "candidate"], list(CONTRACTS.ARMS))
        assert len(result) == 3018
        assert len({row["id"] for row in result}) == 3018

    def test_unknown_arm_is_not_silently_skipped(self) -> None:
        """Unknown arm is not silently skipped."""
        with self.assertRaisesRegex(ValueError, "arms"):
            CONTRACTS.schedule([{"id": "opaque-1"}], ["baseline", "candidate"], ["unknown"])

    def test_missing_result_cannot_look_like_a_complete_run(self) -> None:
        """Missing result cannot look like a complete run."""
        with self.assertRaisesRegex(ValueError, "missing result"):
            CONTRACTS.validate_rows(_cases(), [{"id": _cases()[0]["id"], "status": "ok"}])

    def test_duplicate_result_cannot_fill_a_missing_case(self) -> None:
        """Duplicate result cannot fill a missing case."""
        row = {"id": _cases()[0]["id"], "status": "ok"}
        with self.assertRaisesRegex(ValueError, "duplicate result"):
            CONTRACTS.validate_rows(_cases(), [row, row])

    def test_unknown_result_is_rejected(self) -> None:
        """Unknown result is rejected."""
        with self.assertRaisesRegex(ValueError, "unexpected result"):
            CONTRACTS.validate_rows(_cases(), [{"id": "unexpected", "status": "ok"}])

    def test_outside_artifact_paths_are_rejected(self) -> None:
        """Outside artifact paths are rejected."""
        with tempfile.TemporaryDirectory() as temporary, self.assertRaisesRegex(ValueError, "relative"):
            CONTRACTS.verified_file(Path(temporary), {"file": "../outside"})

    def test_success_label_without_artifacts_cannot_pass(self) -> None:
        """A status label cannot replace genuine prediction and metric artifacts."""
        rows = [{"id": case["id"], "status": "ok"} for case in _cases()]
        with self.assertRaisesRegex(ValueError, "invalid result status"):
            CONTRACTS.validate_rows(_cases(), rows)

    def test_evaluated_result_requires_both_official_metrics(self) -> None:
        """Missing TRM cannot disappear behind a successful evaluation label."""
        rows = [
            {
                "id": case["id"],
                "status": "predicted",
                "prediction": {"file": "real.html"},
                "evaluation_status": "evaluated",
                "metrics": {"grits_con": 1.0},
            }
            for case in _cases()
        ]
        with self.assertRaisesRegex(ValueError, "missing official metrics"):
            CONTRACTS.validate_rows(_cases(), rows)

    def test_official_failure_cannot_be_relabeled_as_evaluated(self) -> None:
        """A complete result census cannot turn failed evaluation into success."""
        rows = [
            {
                "id": case["id"],
                "status": "predicted",
                "prediction": {"file": "real.html"},
                "evaluation_status": "evaluated",
                "official_success": False,
                "metrics": {"grits_con": 1.0, "table_record_match": 1.0},
            }
            for case in _cases()
        ]
        with self.assertRaisesRegex(ValueError, "official success"):
            CONTRACTS.validate_rows(_cases(), rows)

    def test_html_arm_requires_both_cache_layers_disabled(self) -> None:
        """Html arm requires both cache layers disabled."""
        config = {"output_format": "html", "use_cache": False, "ocr": {"tesseract_config": {"language": "eng"}}}
        with self.assertRaisesRegex(ValueError, "OCR cache"):
            CONTRACTS.validate_config(config, "native_html")

    def test_valid_html_configuration_is_accepted(self) -> None:
        """Valid html configuration is accepted."""
        config = {
            "output_format": "html",
            "use_cache": False,
            "disable_ocr": True,
            "ocr": {"tesseract_config": {"language": "eng", "use_cache": False}},
        }
        assert CONTRACTS.validate_config(config, "native_html") is None

    def test_layout_arm_cannot_run_without_explicit_tatr(self) -> None:
        """A mislabeled native configuration must not count as a model arm."""
        config = {"output_format": "html", "use_cache": False, "ocr": {"tesseract_config": {"use_cache": False}}}
        with self.assertRaisesRegex(ValueError, "TATR"):
            CONTRACTS.validate_config(config, "layout_tatr_html")

    def test_forced_ocr_arm_requires_force_ocr(self) -> None:
        """The OCR comparison cannot silently use native text extraction."""
        config = {
            "output_format": "html",
            "use_cache": False,
            "layout": {"table_model": "tatr", "strategy": "always", "acceleration": {"provider": "cpu"}},
            "concurrency": {"max_threads": 1},
            "ocr": {"tesseract_config": {"use_cache": False}},
        }
        with self.assertRaisesRegex(ValueError, "force_ocr"):
            CONTRACTS.validate_config(config, "forced_ocr_layout_html")

    def test_native_layout_arm_rejects_missing_native_layout_opt_in(self) -> None:
        """An omitted opt-in leaves the native PDF layout runner inactive."""
        with self.assertRaisesRegex(ValueError, "use_layout_for_markdown"):
            CONTRACTS.validate_config(_layout_config(), "layout_tatr_html")

    def test_native_layout_arm_rejects_disabled_native_layout_opt_in(self) -> None:
        """A false opt-in cannot qualify an otherwise valid TATR configuration."""
        config = _layout_config()
        config["use_layout_for_markdown"] = False
        with self.assertRaisesRegex(ValueError, "use_layout_for_markdown"):
            CONTRACTS.validate_config(config, "layout_tatr_html")

    def test_native_layout_arm_accepts_explicit_native_layout_opt_in(self) -> None:
        """The native model arm must explicitly activate its layout route."""
        config = _layout_config()
        config["use_layout_for_markdown"] = True
        assert CONTRACTS.validate_config(config, "layout_tatr_html") is None

    def test_forced_ocr_layout_arm_does_not_require_native_layout_opt_in(self) -> None:
        """Forced OCR reaches layout through its independent OCR route."""
        for opt_in in (None, False, True):
            with self.subTest(opt_in=opt_in):
                config = _layout_config()
                config["force_ocr"] = True
                if opt_in is not None:
                    config["use_layout_for_markdown"] = opt_in
                assert CONTRACTS.validate_config(config, "forced_ocr_layout_html") is None


if __name__ == "__main__":
    unittest.main()
