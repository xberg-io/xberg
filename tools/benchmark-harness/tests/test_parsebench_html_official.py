"""Official-metric controls require the separate pinned evaluator environment."""

import hashlib
import importlib.util
import json
import os
import socket
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

# ruff: noqa: PT027


def _adapter(filename="official.py"):
    path = Path(__file__).parents[1] / "parsebench_html" / filename
    spec = importlib.util.spec_from_file_location("parsebench_official", path)
    module = importlib.util.module_from_spec(spec)
    with patch.object(sys, "path", [str(path.parent), *sys.path]):
        spec.loader.exec_module(module)
    return module


@unittest.skipUnless(importlib.util.find_spec("parse_bench"), "requires the isolated pinned ParseBench environment")
class OfficialControlsTest(unittest.TestCase):
    """Use upstream pairing, title stripping and record matching for controls."""

    def _score(self, expected: str, actual: str, rule: str = "{}") -> dict:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "oracle.html").write_text(expected, encoding="utf-8")
            record = {
                "id": "control",
                "pdf": {"file": "not-extracted.pdf"},
                "oracle": {"file": "oracle.html"},
                "upstream": {"rule": rule},
            }
            with (
                patch.dict(os.environ, {"LLAMACLOUD_BENCH_LLM_NORMALIZATION": "off"}),
                patch.object(socket.socket, "connect", side_effect=AssertionError("network forbidden")),
            ):
                result = _adapter().evaluate(record, root, actual)
            return {item["metric_name"]: item["value"] for item in result["metrics"]}

    def test_frozen_model_config_is_validated_even_when_its_hash_matches(self):
        """A stale frozen configuration cannot silently run native extraction."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executable, ort, config = (root / name for name in ("producer", "runtime", "config.json"))
            executable.write_bytes(b"synthetic producer integrity control")
            ort.write_bytes(b"synthetic runtime integrity control")
            configuration = {
                "output_format": "html",
                "use_cache": False,
                "use_layout_for_markdown": True,
                "ocr": {"tesseract_config": {"use_cache": False}},
                "layout": {"strategy": "always", "table_model": "tatr", "acceleration": {"provider": "cpu"}},
                "concurrency": {"max_threads": 1},
            }
            invocation = {
                "config": str(config),
                "argv": ["extract", "{pdf}", "--config", "{config}", "--format", "json"],
                "content_path": ["result", "content"],
            }
            binary = {
                "executable": str(executable),
                "sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
                "ort": {"file": str(ort), "sha256": hashlib.sha256(ort.read_bytes()).hexdigest()},
                "arms": {"layout_tatr_html": invocation},
            }
            runner = _adapter("runner.py")
            config.write_text(json.dumps(configuration))
            invocation["config_sha256"] = hashlib.sha256(config.read_bytes()).hexdigest()
            runner._verify_producer(binary)
            for flag in (False, None):
                with self.subTest(native_layout=flag):
                    configuration.pop("use_layout_for_markdown", None)
                    if flag is not None:
                        configuration["use_layout_for_markdown"] = flag
                    config.write_text(json.dumps(configuration))
                    invocation["config_sha256"] = hashlib.sha256(config.read_bytes()).hexdigest()
                    with self.assertRaisesRegex(ValueError, "use_layout_for_markdown"):
                        runner._verify_producer(binary)

    def test_identical_table_is_positive_control(self):
        """An identical table scores exactly one in both official metrics."""
        html = "<table><tr><th>Item</th><th>Price</th></tr><tr><td>Apple</td><td>17</td></tr></table>"
        metrics = self._score(html, html)
        assert metrics["grits_con"] == 1.0
        assert metrics["table_record_match"] == 1.0

    def test_empty_prediction_is_zero_quality(self):
        """No predicted table must not disappear from the metric denominator."""
        metrics = self._score("<table><tr><td>Apple</td><td>17</td></tr></table>", "")
        assert metrics["grits_con"] == 0.0
        assert metrics["table_record_match"] == 0.0

    def test_lost_spanning_data_changes_official_score(self):
        """A missing spanning cell is detected through the official pipeline."""
        expected = "<table><tr><th>Item</th><th>Price</th></tr><tr><td rowspan='2'>Apple</td><td>17</td></tr><tr><td>23</td></tr></table>"
        actual = "<table><tr><th>Item</th><th>Price</th></tr><tr><td>Apple</td><td>17</td></tr><tr><td></td><td>23</td></tr></table>"
        metrics = self._score(expected, actual)
        assert metrics["grits_con"] < 1.0

    def test_header_flag_loss_records_actual_metric_behavior(self):
        """Measure the upstream metric's header-tag limitation explicitly."""
        html = "<table><tr><th>Item</th><th>Price</th></tr><tr><td>Apple</td><td>17</td></tr></table>"
        metrics = self._score(html, html.replace("th>", "td>"))
        assert metrics["grits_con"] == 1.0
        assert metrics["table_record_match"] == 1.0

    def test_split_join_uses_upstream_pairing(self):
        """Joining distinct tables must not silently bypass pairing penalties."""
        one = "<table><tr><td>Apple</td><td>17</td></tr></table>"
        two = "<table><tr><td>Orange</td><td>29</td></tr></table>"
        metrics = self._score(one + two, one.replace("</table>", "") + two.replace("<table>", ""))
        assert metrics["tables_expected"] == 2.0
        assert metrics["tables_actual"] == 1.0
        assert metrics["grits_con"] < 1.0


if __name__ == "__main__":
    unittest.main()
