"""Run the pinned upstream evaluator's shared table pipeline locally."""

import json
import os
from datetime import datetime, timezone
from pathlib import Path

from parse_bench.evaluation.evaluators.parse import ParseEvaluator
from parse_bench.schemas.parse_output import ParseOutput
from parse_bench.schemas.pipeline_io import InferenceRequest, InferenceResult
from parse_bench.test_cases.schema import ParseTestCase
from safeguards import metric_applicability


def evaluate(record: dict, root: Path, prediction: str) -> dict:
    """Evaluate genuine HTML without substituting the legacy string metric."""
    os.environ["LLAMACLOUD_BENCH_LLM_NORMALIZATION"] = "off"
    oracle = (root / record["oracle"]["file"]).read_text(encoding="utf-8")
    now = datetime.now(timezone.utc)
    output = ParseOutput(example_id=record["id"], pipeline_name="local-html", markdown=prediction)
    inference = InferenceResult(
        request=InferenceRequest(
            example_id=record["id"], source_file_path=str(root / record["pdf"]["file"]), product_type="parse"
        ),
        pipeline_name="local-html",
        product_type="parse",
        raw_output={},
        output=output,
        started_at=now,
        completed_at=now,
        latency_in_ms=0,
    )
    options = json.loads(record["upstream"]["rule"])
    allowed = {"trm_unsupported", "max_top_title_rows", "allow_splitting_ambiguous_merged_tables"}
    if not options.keys() <= allowed:
        raise ValueError("unknown upstream table evaluation options")
    case = ParseTestCase(
        test_id=record["id"], group="table", file_path=root / record["pdf"]["file"], expected_markdown=oracle, **options
    )
    evaluator = ParseEvaluator(
        enable_rule_based=False,
        enable_text_similarity=False,
        enable_teds=False,
        enable_grits=True,
        enable_header_accuracy=False,
        enable_structural_consistency=False,
        enable_table_record_match=True,
        enable_table_composite=False,
        grits_pair_workers=1,
    )
    result = evaluator.evaluate(inference, case)
    serialized = result.model_dump(mode="json")
    metric_applicability(serialized, options)
    return serialized
