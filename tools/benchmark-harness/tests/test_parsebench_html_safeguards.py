"""Pre-sweep controls require no native execution or model inference."""

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

# ruff: noqa: PT027


def _safeguards():
    path = Path(__file__).parents[1] / "parsebench_html" / "safeguards.py"
    spec = importlib.util.spec_from_file_location("safeguards", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _invocation():
    return {
        "argv": ["extract", "{pdf}", "--config", "{config}", "--format", "json"],
        "content_path": ["result", "content"],
    }


def _renderer_glyph_warning():
    return {
        "source": "pdf-render",
        "message": (
            "Page 1 rendering could not paint one or more glyphs and continued anyway "
            "(advance-only, so layout is preserved but the glyph ink is missing): "
            "font 'redacted' painted nothing for 1 glyph(s) while advancing the cursor; "
            "first was code 0x1164 (glyph 4452): no outline. "
            "The page renders with a gap that reads as whitespace downstream. "
            "Reported once per font per page."
        ),
    }


def _word_geometry_fallback_warning():
    return {
        "source": "ocr",
        "message": (
            "OCR word geometry could not be mapped onto the rendered page, so layout-based "
            "table recognition was skipped for a page; its text is reported as paragraphs instead."
        ),
    }


class SafeguardsTest(unittest.TestCase):
    """Prevent mutable producers and degraded model runs from qualifying."""

    def test_supported_cli_invocation_binds_both_frozen_inputs(self):
        """The canonical Extract Clap invocation reads the frozen config and PDF."""
        assert _safeguards().validate_invocation(_invocation()) is None

    def test_model_mutation_after_preflight_is_detected(self):
        """A successful preflight cannot authorize later changed model bytes."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "model"
            source.write_bytes(b"verified model integrity bytes")
            binding = _safeguards().snapshot(source, root / "assets", "model")
            binding["file"] = str(source)
            _safeguards().verify_bound_files([binding])
            source.write_bytes(b"changed after preflight")
            with self.assertRaisesRegex(ValueError, "bound artifact changed"):
                _safeguards().verify_bound_files([binding])

    def test_runtime_symlink_retarget_is_detected(self):
        """Hash the actual runtime cache path rather than its previously resolved target."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            original = root / "original"
            replacement = root / "replacement"
            original.write_bytes(b"verified model integrity bytes")
            replacement.write_bytes(b"different runtime bytes")
            link = root / "runtime-model"
            link.symlink_to(original)
            binding = _safeguards().snapshot(original, root / "assets", "model")
            binding["file"] = str(link)
            _safeguards().verify_bound_files([binding])
            link.unlink()
            link.symlink_to(replacement)
            with self.assertRaisesRegex(ValueError, "bound artifact changed"):
                _safeguards().verify_bound_files([binding])

    def test_missing_config_placeholder_is_rejected(self):
        """A model-arm label cannot qualify CLI default extraction."""
        invocation = _invocation()
        invocation["argv"] = ["extract", "{pdf}", "--format", "json"]
        with self.assertRaisesRegex(ValueError, "frozen PDF/config"):
            _safeguards().validate_invocation(invocation)

    def test_hardcoded_pdf_is_rejected(self):
        """Every scheduled document must reach the actual CLI input argument."""
        invocation = _invocation()
        invocation["argv"][1] = "/some/other.pdf"
        with self.assertRaisesRegex(ValueError, "frozen PDF/config"):
            _safeguards().validate_invocation(invocation)

    def test_inline_override_cannot_disable_frozen_layout(self):
        """CLI config-json overrides apply after file configuration and must be forbidden."""
        invocation = _invocation()
        invocation["argv"].extend(["--config-json", '{"layout": null}'])
        with self.assertRaisesRegex(ValueError, "frozen PDF/config"):
            _safeguards().validate_invocation(invocation)

    def test_individual_flags_cannot_override_frozen_arm(self):
        """Individual extraction flags take precedence over the frozen configuration."""
        invocation = _invocation()
        invocation["argv"].extend(["--force-ocr", "true"])
        with self.assertRaisesRegex(ValueError, "frozen PDF/config"):
            _safeguards().validate_invocation(invocation)

    def test_different_content_path_cannot_hide_sdk_diagnostics(self):
        """JSON output must use the actual ExtractEnvelope result object."""
        invocation = _invocation()
        invocation["content_path"] = ["unrelated", "content"]
        with self.assertRaisesRegex(ValueError, "ExtractEnvelope"):
            _safeguards().validate_invocation(invocation)

    def test_snapshot_survives_producer_overwrite(self):
        """Later Cargo builds cannot overwrite the executable in the plan."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "producer"
            source.write_bytes(b"original executable integrity bytes")
            snapshot = _safeguards().snapshot(source, root / "assets", "baseline", executable=True)
            source.write_bytes(b"rebuilt producer")
            assert Path(snapshot["file"]).read_bytes() == b"original executable integrity bytes"
            assert Path(snapshot["file"]).stat().st_mode & 0o222 == 0

    def test_finite_metrics_and_success_are_both_required(self):
        """NaN cannot be represented as a successful official metric."""
        result = {
            "success": True,
            "metrics": [
                {"metric_name": "grits_con", "value": float("nan")},
                {"metric_name": "table_record_match", "value": 1.0},
            ],
        }
        with self.assertRaisesRegex(ValueError, "finite"):
            _safeguards().metric_applicability(result, {})

    def test_failed_evaluator_with_metric_keys_cannot_pass(self):
        """Upstream failure remains a failure even when partial metrics exist."""
        result = {
            "success": False,
            "metrics": [
                {"metric_name": "grits_con", "value": 1.0},
                {"metric_name": "table_record_match", "value": 1.0},
            ],
        }
        with self.assertRaisesRegex(ValueError, "success"):
            _safeguards().metric_applicability(result, {})

    def test_trm_unsupported_preserves_raw_score_but_not_eligibility(self):
        """Unsupported TRM records retain inference and GriTS coverage."""
        result = {
            "success": True,
            "metrics": [
                {"metric_name": "grits_con", "value": 1.0},
                {"metric_name": "table_record_match", "value": 1.0},
            ],
        }
        eligibility = _safeguards().metric_applicability(result, {"trm_unsupported": True})
        assert eligibility == {"grits_con": True, "table_record_match": False}
        assert len(result["metrics"]) == 2

    def test_structured_table_model_fallback_is_rejected(self):
        """The SDK's real skipped-table-model warning invalidates a model arm."""
        result = {
            "processing_warnings": [
                {
                    "source": "native_pdf_layout",
                    "message": "layout detection found table regions but the TATR table model is not available; table structure recognition was skipped; check that the model is downloaded",
                }
            ]
        }
        with self.assertRaisesRegex(ValueError, "model fallback"):
            _safeguards().validate_diagnostics(result, "", "layout_tatr_html")

    def test_stderr_model_unavailability_is_rejected(self):
        """A tracing-only failed model load is not hidden by valid JSON."""
        with self.assertRaisesRegex(ValueError, "model fallback"):
            _safeguards().validate_diagnostics({}, "TATR model unavailable: missing model", "forced_ocr_layout_html")

    def test_diagnostic_scope_renderer_layout_preservation_is_not_model_fallback(self):
        """Retain the actual sanitized renderer warning without relabeling its layout phrase."""
        for arm in ("layout_tatr_html", "forced_ocr_layout_html"):
            with self.subTest(arm=arm):
                result = {"processing_warnings": [_renderer_glyph_warning()]}
                assert _safeguards().validate_diagnostics(result, "", arm) is None
                assert result == {"processing_warnings": [_renderer_glyph_warning()]}

    def test_diagnostic_scope_word_geometry_table_fallback_remains_rejected(self):
        """A successful SDK payload can still skip the model arm's table-recognition route."""
        for arm in ("layout_tatr_html", "forced_ocr_layout_html"):
            with self.subTest(arm=arm):
                result = {"processing_warnings": [_word_geometry_fallback_warning()]}
                with self.assertRaisesRegex(ValueError, "model fallback diagnostic: ocr ocr word geometry"):
                    _safeguards().validate_diagnostics(result, "", arm)

    def test_diagnostic_scope_renderer_warning_cannot_hide_table_fallback(self):
        """An unrelated earlier warning must not bypass a later genuine SDK fallback."""
        for arm in ("layout_tatr_html", "forced_ocr_layout_html"):
            with self.subTest(arm=arm):
                result = {"processing_warnings": [_renderer_glyph_warning(), _word_geometry_fallback_warning()]}
                with self.assertRaisesRegex(ValueError, "model fallback diagnostic: ocr ocr word geometry"):
                    _safeguards().validate_diagnostics(result, "", arm)

    def test_diagnostic_scope_renderer_warning_cannot_hide_stderr_model_failure(self):
        """Model unavailability must be caught in stderr after unrelated SDK warnings."""
        for arm in ("layout_tatr_html", "forced_ocr_layout_html"):
            with self.subTest(arm=arm):
                result = {"processing_warnings": [_renderer_glyph_warning()]}
                with self.assertRaisesRegex(ValueError, "model fallback diagnostic: tatr model unavailable"):
                    _safeguards().validate_diagnostics(result, "TATR model unavailable: missing model", arm)

    def test_diagnostic_scope_table_model_that_could_not_be_loaded_is_rejected(self):
        """The native structure emitter reports a failed model checkout without stderr."""
        result = {
            "processing_warnings": [
                {
                    "source": "pdf_tables",
                    "message": (
                        "layout detection found table regions but the TATR table model could not be loaded for a "
                        "page; its tables were read from the page text"
                    ),
                }
            ]
        }
        for arm in ("layout_tatr_html", "forced_ocr_layout_html"):
            with self.subTest(arm=arm), self.assertRaisesRegex(ValueError, "model fallback diagnostic: pdf_tables"):
                _safeguards().validate_diagnostics(result, "", arm)

    def test_diagnostic_scope_page_not_prepared_for_layout_analysis_is_rejected(self):
        """The layout runner reports an unanalyzed placeholder page without model wording."""
        result = {
            "processing_warnings": [
                {
                    "source": "layout",
                    "message": (
                        "layout: 1 page(s) could not be prepared for layout analysis and were treated as empty: "
                        "page 2 failed to render: invalid page"
                    ),
                }
            ]
        }
        for arm in ("layout_tatr_html", "forced_ocr_layout_html"):
            with self.subTest(arm=arm), self.assertRaisesRegex(ValueError, "model fallback diagnostic: layout"):
                _safeguards().validate_diagnostics(result, "", arm)

    def test_diagnostic_scope_omitted_layout_segments_cannot_use_unsorted_fallback(self):
        """An incomplete model-layout plan is a fallback even when all segments are appended."""
        stderr = "layout region plan omitted segments; appending an unsorted fallback group"
        for arm in ("layout_tatr_html", "forced_ocr_layout_html"):
            with (
                self.subTest(arm=arm),
                self.assertRaisesRegex(ValueError, "model fallback diagnostic: layout region plan"),
            ):
                _safeguards().validate_diagnostics({}, stderr, arm)

    def test_diagnostic_scope_slanext_failure_targets_keep_previous_rejection(self):
        """Preserve existing model-target coverage beyond the frozen TATR configuration."""
        diagnostics = (
            "WARN xberg::layout: SLANeXT model unavailable: invalid model",
            (
                "WARN xberg::layout::models::slanet: SLANeXT: platform EP failed (initialization error), "
                "retrying with CPU-only"
            ),
        )
        for arm in ("layout_tatr_html", "forced_ocr_layout_html"):
            for stderr in diagnostics:
                with (
                    self.subTest(arm=arm, stderr=stderr),
                    self.assertRaisesRegex(ValueError, "model fallback diagnostic: warn xberg::layout"),
                ):
                    _safeguards().validate_diagnostics({}, stderr, arm)

    def test_diagnostic_scope_table_classifier_target_keeps_previous_rejection(self):
        """A known model component remains a failure without a TATR name in its message."""
        stderr = "WARN xberg::layout: Table classifier unavailable: invalid model"
        for arm in ("layout_tatr_html", "forced_ocr_layout_html"):
            with (
                self.subTest(arm=arm),
                self.assertRaisesRegex(ValueError, "model fallback diagnostic: warn xberg::layout"),
            ):
                _safeguards().validate_diagnostics({}, stderr, arm)

    def test_diagnostic_scope_layout_ocr_fallback_keeps_previous_rejection(self):
        """Keep the image-layout warning contract without asserting a frozen PDF observation."""
        result = {
            "processing_warnings": [
                {
                    "source": "layout-ocr",
                    "message": "Layout-region OCR failed after whole-image OCR succeeded; retained whole-image output",
                }
            ]
        }
        for arm in ("layout_tatr_html", "forced_ocr_layout_html"):
            with self.subTest(arm=arm), self.assertRaisesRegex(ValueError, "model fallback diagnostic: layout-ocr"):
                _safeguards().validate_diagnostics(result, "", arm)

    def test_wrong_feature_build_is_rejected(self):
        """A build lacking layout detection cannot qualify a layout arm."""
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "build.json"
            path.write_text(json.dumps({"commit": "a" * 40, "executable_sha256": "b" * 64, "features": ["pdf", "ocr"]}))
            with self.assertRaisesRegex(ValueError, "features"):
                _safeguards().validate_build(path, "a" * 40, "b" * 64)


if __name__ == "__main__":
    unittest.main()
