"""Fail-closed controls use synthetic integrity bytes, never extraction evidence."""

import hashlib
import importlib.util
import json
import re
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

    def test_formula_model_pins_match_the_native_recognizer(self):
        """The runner and the native model verifier require the same bytes."""
        native = Path(__file__).parents[3] / "crates/xberg/src/formula_recognition/mod.rs"
        table = native.read_text().split("const MODEL_FILES:", 1)[1].split("];", 1)[0]
        pins = dict(re.findall(r'"([^"]+)",\s+"([0-9a-f]{64})"', table))
        assert len(pins) == 4
        assert pins == _contracts().FORMULA_MODEL_SHA256

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

    def test_score_retains_reports_but_rejects_math_without_native_evidence(self):
        """A clean evaluator result cannot override missing native formula proof."""
        with patch.object(sys, "path", [str(Path(__file__).parents[1]), *sys.path]):
            runner = importlib.import_module("olmocr_bench.runner")
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        plan_path = root / "plan.json"
        plan_path.write_text("{}")
        generation = root / "generation"
        pdfs = [{"id": "math", "file": "bench_data/pdfs/arxiv_math/math.pdf"}]
        cases = runner.schedule(pdfs)
        rows = []
        for case in cases:
            prediction = generation / case["role"] / "arxiv_math/math_pg1_repeat1.md"
            prediction.parent.mkdir(parents=True)
            prediction.write_bytes(b"$$x^2$$")
            rows.append(
                dict(
                    case,
                    prediction={"file": str(prediction), "sha256": runner.digest(prediction)},
                    status="extracted",
                    recognized_formula=case["role"] == "baseline",
                )
            )
        receipt = {"plan_sha256": runner.digest(plan_path), "rows": rows}
        (generation / "generation.json").write_text(json.dumps(receipt))
        report_dir = root / "reports"
        plan = {
            "cases": cases,
            "catalog": {"pdfs": pdfs, "tests": [{}]},
            "evaluator_root": str(root),
            "browser_root": str(root),
        }
        with (
            patch.object(runner, "_read_plan", return_value=plan),
            patch.object(runner, "_ready"),
            patch.object(runner, "_verify"),
            patch.object(runner, "TEST_COUNT", 1),
            patch.object(runner, "evaluation_environment"),
            patch.object(runner, "load_official", return_value=([object()], [{}])),
            patch.object(runner, "score_candidate", return_value={"tests": [{}], "errors": []}) as evaluate,
        ):
            with self.assertRaisesRegex(
                ValueError, "qualification invalid: no native formula recognition in math cases for: candidate"
            ):
                runner.score(plan_path, plan_path, generation, report_dir)
            assert evaluate.call_count == 2
        coverage = json.loads((report_dir / "coverage.json").read_text())
        assert coverage["official_test_results"] == 2
        assert coverage["healthy"] is False
        assert coverage["math_coverage"]["candidate"]["arxiv_math"] == {"cases": 1, "recognized_cases": 0}
        assert all((report_dir / f"{role}.json").is_file() for role in runner.ROLES)


class ModelFallbackTest(unittest.TestCase):
    """Distinguish the optional OCR retry from unavailable benchmark models."""

    def test_tesseract_osd_retry_is_not_model_fallback(self):
        """The Full-format warning is from the retained public olmOCR pilot."""
        with patch.object(sys, "path", [str(Path(__file__).parents[1]), *sys.path]):
            runner = importlib.import_module("olmocr_bench.runner")
        warning = (
            '2026-11-17T00:14:32.190193Z  WARN xberg.ocr{xberg.ocr.backend="tesseract" xberg.ocr.language="eng"}: '
            'xberg::ocr::processor::model_context: script retry skipped or changed; bypassing result cache script="Han" '
            "error=OCR processing failed: script retry: OSD: OCR operation failed"
        )
        assert not runner._model_fallback({}, warning)
        assert not runner._model_fallback({}, warning.replace("WARN", "\x1b[33m WARN\x1b[0m"))

        for failure in (
            warning.replace("ocr::processor::model_context:", "layout::model_context:"),
            warning.replace("script retry: OSD: OCR operation failed", "Primary Tesseract models changed during OCR"),
            warning.replace("script retry: OSD: OCR operation failed", "script retry: OSD model initialization failed"),
            (
                "WARN xberg::extractors::pdf::layout_runner: layout-for-markdown: detection failed, "
                "continuing without layout hints"
            ),
            "WARN xberg::extractors::pdf: pdf formula recognition failed; keeping OCR text",
            "WARN xberg::onnx: failed to load libonnxruntime.so",
        ):
            with self.subTest(failure=failure):
                assert runner._model_fallback({}, f"{warning}\n{failure}")

        warnings = {
            "processing_warnings": [
                {
                    "source": "pdf_tables",
                    "message": "layout detection found table regions but the TATR table model is not available; "
                    "table structure recognition was skipped; check that the model is downloaded",
                }
            ]
        }
        assert runner._model_fallback(warnings, warning)

    def test_native_form_flate_decode_is_not_model_fallback(self):
        """Use the paired structured warnings and ANSI trace from baseline/863bf01719."""
        with patch.object(sys, "path", [str(Path(__file__).parents[1]), *sys.path]):
            runner = importlib.import_module("olmocr_bench.runner")
        error = (
            "Layout analysis error: incomplete Form path decode: Stream decoding error: "
            "FlateDecode: incomplete zlib stream or trailing data"
        )
        warnings = [
            {
                "source": "pdf_tables",
                "message": f"table extraction failed for page 1: {error}; tables on this page were skipped",
            },
            {
                "source": "pdf_tables",
                "message": f"bordered table extraction failed for page 1: {error}; "
                "bordered tables on this page were skipped",
            },
        ]
        trace = (
            "\x1b[2m2026-11-17T01:05:05.375834Z\x1b[0m \x1b[33m WARN\x1b[0m "
            "\x1b[2mxberg::pdf::native::table\x1b[0m\x1b[2m:\x1b[0m "
            f"xberg_native_pdf ruled table extraction failed: {error} \x1b[3mpage\x1b[0m\x1b[2m=\x1b[0m0"
        )
        assert not runner._model_fallback({"processing_warnings": warnings}, "")
        assert not runner._model_fallback({}, trace)
        assert not runner._model_fallback({"processing_warnings": warnings}, trace)

        for warning in (
            {"source": "layout", "message": warnings[0]["message"]},
            {"source": "layout", "message": "layout detection failed; source raster refused by the pixel budget"},
            {"source": "pdf_tables", "message": "TATR table model is not available; recognition was skipped"},
            {"source": "pdf_tables", "message": warnings[1]["message"] + "; ONNX model failed to load"},
        ):
            with self.subTest(warning=warning):
                assert runner._model_fallback({"processing_warnings": [*warnings, warning]}, trace)
        for failure in (
            trace.replace("xberg::pdf::native::table", "xberg::extractors::pdf::layout_runner"),
            trace + " model fallback was used",
            "WARN xberg::extractors::pdf: pdf formula recognition failed; keeping OCR text",
            "WARN xberg::onnx: failed to load libonnxruntime.so",
            "WARN xberg::hf: failed to download model from registry",
        ):
            with self.subTest(failure=failure):
                assert runner._model_fallback({"processing_warnings": warnings}, f"{trace}\n{failure}")


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
        self.enterContext(patch.object(contracts, "FORMULA_MODEL_SHA256", self.pins))
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
            assert env["RUST_LOG"] == "info,xberg::extractors::pdf::formula_evidence=debug"
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

    def test_replaced_formula_link_or_staging_file_counts_as_fallback(self):
        """A successful producer exit cannot hide a native formula download."""
        for mutation in ("replacement", "wrong_target", "staging"):

            def produce(*_args, env, mutation=mutation, **_kwargs):
                cache = Path(env["XBERG_CACHE_DIR"]) / "formula-recognition"
                link = cache / "decoder.onnx"
                if mutation == "replacement":
                    link.unlink()
                    link.write_bytes(b"downloaded")
                elif mutation == "wrong_target":
                    link.unlink()
                    link.symlink_to(self.model_dir / "encoder.onnx")
                else:
                    (cache / ".decoder.onnx.123.tmp").touch()
                return subprocess.CompletedProcess([], 0, b'{"result":{"content":"diagnostic"}}', b"")

            with self.subTest(mutation=mutation), patch.object(self.runner.subprocess, "run", side_effect=produce):
                content, details = self._extract()
                assert content == "diagnostic"
                assert details["formula_cache_intact"] is False
                assert details["model_fallback"] is True

    def test_plan_verifier_cannot_admit_a_missing_formula_row(self):
        """Plan verification must reject incomplete bindings before native calls."""
        asset = self._bind(self.root / "config.json", json.dumps(_config()).encode())
        pdfs = [{"id": str(index)} for index in range(984)]
        plan = dict(
            self.plan,
            config=asset,
            ort=asset,
            requirements=asset,
            python=asset,
            catalog={"descriptor": asset, "pdfs": pdfs, "tests": [{}] * self.runner.TEST_COUNT},
            renderer_manifest=asset,
            evaluator_assets=[],
            renderer_assets=[],
            producers={},
            cases=self.runner.schedule(pdfs),
            repeats=1,
        )
        self.runner._verify(plan)
        plan["models"] = self.models[1:]
        with patch.object(self.runner.subprocess, "run") as invoke:
            with self.assertRaisesRegex(ValueError, "exactly one pinned RapidLaTeXOCR image_resizer.onnx"):
                self.runner._verify(plan)
            invoke.assert_not_called()

    def test_freeze_cannot_admit_a_missing_formula_row(self):
        """Missing formula paths fail before other freeze work can publish a plan."""
        config = self._bind(self.root / "config.json", json.dumps(_config()).encode())
        spec = self.root / "spec.json"
        spec.write_text(
            json.dumps(
                {
                    "config": config["file"],
                    "argv": self.plan["argv"],
                    "models": [dict(row, revision="synthetic", license="synthetic") for row in self.models[1:]],
                }
            )
        )
        freeze = importlib.import_module("olmocr_bench.freeze")
        destination = self.root / "plan.json"
        with (
            patch.object(freeze, "load_catalog", return_value={}),
            patch.object(freeze, "_environment", return_value=({}, {})),
            self.assertRaisesRegex(ValueError, "exactly one pinned RapidLaTeXOCR image_resizer.onnx"),
        ):
            freeze.create_plan(self.root, spec, destination)
        assert not destination.exists()

    def test_old_plan_is_rejected_with_refreeze_instruction(self):
        """Legacy plans cannot silently skip the new qualification checks."""
        plan = self.root / "old-plan.json"
        plan.write_text('{"schema_version": 1}')
        with self.assertRaisesRegex(ValueError, "expected plan schema 2; rerun freeze"):
            self.runner._read_plan(plan)

    def test_recognition_requires_positive_native_trace_and_matching_formula_page(self):
        """Accept modeled core CLI Full-format spans for both native outcomes."""
        document = {"content": "$$x^2$$", "formulas": [{"page": 1, "latex": "x^2"}]}
        timestamp = "2026-01-01T00:00:00.000001Z"
        spans = (
            'extract_file{mime_type=None xberg.operation="extract_file" xberg.document.filename="paper {v2}.pdf"}:'
            'xberg.pipeline{xberg.pipeline.stage="extraction" xberg.extractor.name="pdf" xberg.extractor.priority=50}: '
        )
        message = (
            "xberg::extractors::pdf::formula_evidence: native PDF formula recognition applied "
            "page=1 replaced=2 synthesized=0"
        )
        trace = f"{timestamp} DEBUG {spans}{message}"
        synthesis = trace.replace("replaced=2 synthesized=0", "replaced=0 synthesized=2")
        legacy = (
            f"{timestamp}  INFO {spans}xberg::extractors::pdf: "
            "recognized formulas from layout regions with no extracted formula text page=1 formulas=1"
        )

        for evidence in (trace, synthesis, legacy, trace.replace(spans, ""), legacy.replace(spans, "")):
            with self.subTest(evidence=evidence):
                assert self.runner._recognized_formula(document, evidence)
        assert self.runner._recognized_formula(document, trace.replace("DEBUG", "\x1b[32mDEBUG\x1b[0m"))

        for evidence in (
            "",
            trace.replace("page=1", "page=2"),
            trace.replace("replaced=2", "replaced=0"),
            trace.replace("formula_evidence:", "unrelated:"),
            trace.replace("DEBUG", "INFO"),
            trace.replace("native PDF formula recognition applied", "unrelated PDF event"),
            legacy.replace("formulas=1", "formulas=0"),
        ):
            with self.subTest(evidence=evidence):
                assert not self.runner._recognized_formula(document, evidence)
        assert not self.runner._recognized_formula({"content": "$$x^2$$"}, trace)
        assert not self.runner._recognized_formula({"formulas": [{"page": 1, "latex": " "}]}, trace)

    def test_recognition_reads_actual_core_cli_event_and_rejects_it_inside_span_text(self):
        """Match the Full-format event captured from the public arXiv pilot."""
        message = (
            "xberg::extractors::pdf::formula_evidence: native PDF formula recognition applied "
            "page=1 replaced=0 synthesized=2"
        )
        trace = (
            '2026-11-16T22:40:07.780286Z DEBUG extract_file{mime_type=None xberg.operation="extract_file" '
            'xberg.document.filename="2503.04048_pg46.pdf"}:xberg.pipeline{xberg.pipeline.stage="extraction" '
            f'xberg.extractor.name="pdf-extractor" xberg.extractor.priority=50}}: {message}'
        )
        document = {"formulas": [{"page": 1, "latex": "x^2"}]}
        assert self.runner._recognized_formula(document, trace)
        decoy = f'DEBUG extraction{{detail="DEBUG {message}"}}: xberg::unrelated: extraction finished'
        assert not self.runner._recognized_formula(document, decoy)

    def test_math_coverage_does_not_count_fallback_with_formula_evidence(self):
        """A fallback cannot qualify merely because an earlier formula ran."""
        pdfs = {"math": {"file": "bench_data/pdfs/arxiv_math/math.pdf"}}
        rows = [{"role": "baseline", "pdf_id": "math", "status": "model_fallback", "recognized_formula": True}]
        coverage = self.runner._math_coverage(rows, pdfs)
        assert coverage["baseline"]["arxiv_math"] == {"cases": 1, "recognized_cases": 0}

    def test_read_plan_rejects_an_installed_dependency_change(self):
        """A previously frozen dependency version must still match at read time."""
        plan = self.root / "plan.json"
        lock = self.root / "requirements.lock"
        lock.write_text("playwright==1.2.3\n")
        catalog = {"registry": None}
        plan.write_text(
            json.dumps(
                {
                    "schema_version": 2,
                    "catalog": catalog,
                    "data_root": str(self.root),
                    "requirements": {"file": str(lock)},
                    "dependencies": {"playwright": "1.2.3"},
                }
            )
        )
        freeze = importlib.import_module("olmocr_bench.freeze")
        with (
            patch.object(self.runner, "_verify"),
            patch.object(self.runner, "load_catalog", return_value=catalog),
            patch.object(freeze.importlib.metadata, "version", side_effect=["1.2.3", "1.2.4"]),
        ):
            assert self.runner._read_plan(plan)["dependencies"] == {"playwright": "1.2.3"}
            with self.assertRaisesRegex(ValueError, "installed dependency differs from audited lock: playwright"):
                self.runner._read_plan(plan)

    def test_generate_retains_failed_math_coverage_and_preserves_prose_only_controls(self):
        """A math case requires native evidence, while a prose-only pilot has no math gate."""
        plan_path = self.root / "plan.json"
        plan_path.write_text("{}")
        pdfs = [
            {"id": "native", "file": "bench_data/pdfs/arxiv_math/native.pdf", "sha256": self.pdf["sha256"]},
            {"id": "scan", "file": "bench_data/pdfs/old_scans_math/scan.pdf", "sha256": self.pdf["sha256"]},
        ]
        with (
            patch.object(self.runner, "_ready"),
            patch.object(self.runner, "_verify"),
            patch.object(self.runner, "verify_assets"),
            patch.object(self.runner, "_read_plan") as read_plan,
            patch.object(self.runner, "_extract") as extract,
        ):
            read_plan.return_value = {
                "catalog": {"pdfs": pdfs},
                "cases": self.runner.schedule(pdfs),
                "data_root": str(self.root),
            }
            extract.side_effect = [
                ("$$x^2$$", {"model_fallback": False, "recognized_formula": value})
                for value in (True, False, False, False)
            ]
            output = self.root / "failed"
            with self.assertRaisesRegex(
                ValueError, "no native formula recognition witnessed in math cases for: candidate"
            ):
                self.runner.generate(plan_path, plan_path, output, 1)
            report = json.loads((output / "generation.json").read_text())
            assert report["healthy"] is False
            assert report["math_coverage"]["baseline"]["arxiv_math"] == {"cases": 1, "recognized_cases": 1}
            assert report["math_coverage"]["candidate"]["old_scans_math"] == {"cases": 1, "recognized_cases": 0}
            assert len(report["rows"]) == 4
            assert len((output / "generation.jsonl").read_text().splitlines()) == 4

            prose = [dict(pdfs[0], file="bench_data/pdfs/old_scans/prose.pdf")]
            read_plan.return_value = {
                "catalog": {"pdfs": prose},
                "cases": self.runner.schedule(prose),
                "data_root": str(self.root),
            }
            extract.side_effect = None
            extract.return_value = ("prose", {"model_fallback": False, "recognized_formula": False})
            output = self.root / "prose"
            self.runner.generate(plan_path, plan_path, output, 1)
            assert json.loads((output / "generation.json").read_text())["healthy"] is True


if __name__ == "__main__":
    unittest.main()
