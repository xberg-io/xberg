import inspect

from xberg._xberg import (
    LayoutDetectionConfig,
    LayoutDetectionModel,
    LayoutStrategy,
    TableModel,
    TableOverlapPreference,
)

RELEASED_ORDER = [
    "strategy",
    "apply_heuristics",
    "table_model",
    "table_overlap_preference",
    "enable_chart_understanding",
    "confidence_threshold",
    "formula_model",
    "acceleration",
]


def test_layout_config_accepts_the_positional_order_released_before_the_model_field() -> None:
    config = LayoutDetectionConfig(
        LayoutStrategy.ALWAYS,
        True,
        TableModel.TATR,
        TableOverlapPreference.CONTENT,
        False,
        0.5,
    )

    assert config.confidence_threshold == 0.5
    assert config.model == LayoutDetectionModel.RT_DETR


def test_layout_config_keeps_model_after_every_released_parameter() -> None:
    names = list(inspect.signature(LayoutDetectionConfig).parameters)

    assert names[: len(RELEASED_ORDER)] == RELEASED_ORDER
    assert names[len(RELEASED_ORDER) :] == ["model"]
