from __future__ import annotations

import pytest
from PIL import Image

from kreuzberg._types import ExtractionConfig
from kreuzberg._utils._image_preprocessing import (
    calculate_optimal_dpi,
    estimate_processing_time,
    get_dpi_adjustment_heuristics,
    normalize_image_dpi,
)
from kreuzberg.exceptions import ValidationError


class TestDPIConfiguration:
    """Test DPI configuration validation and functionality."""

    def test_valid_dpi_config(self) -> None:
        """Test valid DPI configuration accepts correct values."""
        config = ExtractionConfig(
            target_dpi=150,
            max_image_dimension=25000,
            auto_adjust_dpi=True,
            min_dpi=72,
            max_dpi=600,
        )
        assert config.target_dpi == 150
        assert config.max_image_dimension == 25000
        assert config.auto_adjust_dpi is True
        assert config.min_dpi == 72
        assert config.max_dpi == 600

    def test_invalid_min_max_dpi(self) -> None:
        """Test that min_dpi >= max_dpi raises ValidationError."""
        with pytest.raises(ValidationError, match="min_dpi must be less than max_dpi"):
            ExtractionConfig(min_dpi=300, max_dpi=200)

        with pytest.raises(ValidationError, match="min_dpi must be less than max_dpi"):
            ExtractionConfig(min_dpi=150, max_dpi=150)

    def test_target_dpi_out_of_range(self) -> None:
        """Test target_dpi outside min/max bounds raises ValidationError."""
        with pytest.raises(ValidationError, match="target_dpi must be between min_dpi and max_dpi"):
            ExtractionConfig(target_dpi=50, min_dpi=72, max_dpi=600)

        with pytest.raises(ValidationError, match="target_dpi must be between min_dpi and max_dpi"):
            ExtractionConfig(target_dpi=700, min_dpi=72, max_dpi=600)

    def test_invalid_max_image_dimension(self) -> None:
        """Test max_image_dimension <= 0 raises ValidationError."""
        with pytest.raises(ValidationError, match="max_image_dimension must be positive"):
            ExtractionConfig(max_image_dimension=0)

        with pytest.raises(ValidationError, match="max_image_dimension must be positive"):
            ExtractionConfig(max_image_dimension=-1000)

    def test_edge_case_dpi_values(self) -> None:
        """Test edge case DPI values are handled correctly."""
        # Minimum possible values
        config = ExtractionConfig(
            target_dpi=1,
            min_dpi=1,
            max_dpi=2,
            max_image_dimension=1,
        )
        assert config.target_dpi == 1
        assert config.min_dpi == 1
        assert config.max_dpi == 2

        # Very high values
        config = ExtractionConfig(
            target_dpi=2000,
            min_dpi=1000,
            max_dpi=3000,
            max_image_dimension=100000,
        )
        assert config.target_dpi == 2000
        assert config.max_image_dimension == 100000


class TestDPICalculation:
    """Test DPI calculation functions."""

    def test_calculate_optimal_dpi_no_scaling_needed(self) -> None:
        """Test DPI calculation when no scaling is needed."""
        # Small page that doesn't need adjustment
        optimal_dpi = calculate_optimal_dpi(
            page_width=500,  # points
            page_height=700,  # points
            target_dpi=150,
            max_dimension=25000,
            min_dpi=72,
            max_dpi=600,
        )
        assert optimal_dpi == 150  # Should use target DPI

    def test_calculate_optimal_dpi_needs_scaling(self) -> None:
        """Test DPI calculation when scaling is needed for large pages."""
        # Very large page that needs DPI reduction
        optimal_dpi = calculate_optimal_dpi(
            page_width=2000,  # points (very wide)
            page_height=3000,  # points (very tall)
            target_dpi=300,
            max_dimension=10000,  # Restrictive limit
            min_dpi=72,
            max_dpi=600,
        )
        assert optimal_dpi < 300  # Should be reduced
        assert optimal_dpi >= 72  # Should respect min_dpi

    def test_calculate_optimal_dpi_respects_bounds(self) -> None:
        """Test DPI calculation respects min/max bounds."""
        # Test min_dpi bound
        optimal_dpi = calculate_optimal_dpi(
            page_width=100,
            page_height=100,
            target_dpi=50,  # Below min_dpi
            max_dimension=25000,
            min_dpi=72,
            max_dpi=600,
        )
        assert optimal_dpi == 72  # Should be clamped to min_dpi

        # Test max_dpi bound
        optimal_dpi = calculate_optimal_dpi(
            page_width=100,
            page_height=100,
            target_dpi=800,  # Above max_dpi
            max_dimension=25000,
            min_dpi=72,
            max_dpi=600,
        )
        assert optimal_dpi == 600  # Should be clamped to max_dpi

    def test_calculate_optimal_dpi_zero_dimensions(self) -> None:
        """Test DPI calculation with zero or invalid dimensions."""
        # Zero width
        optimal_dpi = calculate_optimal_dpi(
            page_width=0,
            page_height=100,
            target_dpi=150,
            max_dimension=25000,
            min_dpi=72,
            max_dpi=600,
        )
        assert 72 <= optimal_dpi <= 600  # Should return valid DPI

        # Zero height
        optimal_dpi = calculate_optimal_dpi(
            page_width=100,
            page_height=0,
            target_dpi=150,
            max_dimension=25000,
            min_dpi=72,
            max_dpi=600,
        )
        assert 72 <= optimal_dpi <= 600  # Should return valid DPI


class TestImageNormalization:
    """Test image normalization functionality."""

    def test_normalize_image_dpi_no_change_needed(self) -> None:
        """Test image normalization when no change is needed."""
        # Create a small test image
        image = Image.new("RGB", (100, 100), "white")
        image.info["dpi"] = (150, 150)

        config = ExtractionConfig(
            target_dpi=150,
            max_image_dimension=25000,
            auto_adjust_dpi=False,
        )

        normalized_image, metadata = normalize_image_dpi(image, config)

        # Image should be unchanged
        assert normalized_image.size == (100, 100)
        assert metadata.scale_factor == 1.0
        assert getattr(metadata, "skipped_resize", False)

    def test_normalize_image_dpi_upscaling(self) -> None:
        """Test image normalization with upscaling."""
        # Create a small test image with low DPI
        image = Image.new("RGB", (100, 100), "white")
        image.info["dpi"] = (72, 72)  # Low DPI

        config = ExtractionConfig(
            target_dpi=144,  # 2x the original DPI
            max_image_dimension=25000,
            auto_adjust_dpi=False,
        )

        normalized_image, metadata = normalize_image_dpi(image, config)

        # Image should be upscaled
        assert normalized_image.size == (200, 200)  # 2x original size
        assert abs(metadata.scale_factor - 2.0) < 0.01
        assert metadata.final_dpi == 144

    def test_normalize_image_dpi_downscaling(self) -> None:
        """Test image normalization with downscaling."""
        # Create a large test image with high DPI
        image = Image.new("RGB", (1000, 1000), "white")
        image.info["dpi"] = (300, 300)  # High DPI

        config = ExtractionConfig(
            target_dpi=150,  # Half the original DPI
            max_image_dimension=25000,
            auto_adjust_dpi=False,
        )

        normalized_image, metadata = normalize_image_dpi(image, config)

        # Image should be downscaled
        assert normalized_image.size == (500, 500)  # 0.5x original size
        assert abs(metadata.scale_factor - 0.5) < 0.01
        assert metadata.final_dpi == 150

    def test_normalize_image_dpi_auto_adjust(self) -> None:
        """Test image normalization with auto-adjustment."""
        # Create a very large image that would exceed max_dimension
        image = Image.new("RGB", (20000, 30000), "white")
        image.info["dpi"] = (300, 300)

        config = ExtractionConfig(
            target_dpi=300,
            max_image_dimension=15000,  # Force auto-adjustment
            auto_adjust_dpi=True,
            min_dpi=72,
            max_dpi=600,
        )

        normalized_image, metadata = normalize_image_dpi(image, config)

        # Image should be scaled down to fit within max_dimension
        max_dim = max(normalized_image.size)
        assert max_dim <= config.max_image_dimension
        assert metadata.auto_adjusted
        assert metadata.scale_factor < 1.0

    def test_normalize_image_dpi_no_dpi_info(self) -> None:
        """Test image normalization when image has no DPI info."""
        # Create image without DPI info
        image = Image.new("RGB", (200, 200), "white")
        # Don't set DPI info - should default to 72

        config = ExtractionConfig(
            target_dpi=144,
            max_image_dimension=25000,
            auto_adjust_dpi=False,
        )

        normalized_image, metadata = normalize_image_dpi(image, config)

        # Should assume 72 DPI and scale accordingly
        expected_scale = 144 / 72  # 2.0
        assert abs(metadata.scale_factor - expected_scale) < 0.01
        assert normalized_image.size == (400, 400)  # 2x original


class TestDPIHeuristics:
    """Test DPI adjustment heuristics."""

    def test_get_dpi_adjustment_heuristics_normal_document(self) -> None:
        """Test heuristics for normal document."""
        heuristics = get_dpi_adjustment_heuristics(
            width=1000,
            height=1400,
            current_dpi=72,
            target_dpi=150,
            max_dimension=25000,
            content_type="document",
        )

        assert heuristics["content_analysis"]["aspect_ratio"] == pytest.approx(1000 / 1400, abs=0.01)
        assert not heuristics["content_analysis"]["is_large"]
        assert heuristics["performance_impact"] in ["low", "medium", "high"]
        assert isinstance(heuristics["recommendations"], list)

    def test_get_dpi_adjustment_heuristics_large_document(self) -> None:
        """Test heuristics for large document."""
        heuristics = get_dpi_adjustment_heuristics(
            width=30000,
            height=40000,
            current_dpi=200,
            target_dpi=200,
            max_dimension=25000,
            content_type="document",
        )

        assert heuristics["content_analysis"]["is_large"]
        assert heuristics["content_analysis"]["megapixels"] > 50
        assert heuristics["performance_impact"] == "high"
        assert any("Large document" in rec for rec in heuristics["recommendations"])

    def test_get_dpi_adjustment_heuristics_narrow_document(self) -> None:
        """Test heuristics for narrow/wide documents."""
        # Very wide document
        heuristics = get_dpi_adjustment_heuristics(
            width=5000,
            height=1000,
            current_dpi=150,
            target_dpi=150,
            max_dimension=25000,
            content_type="document",
        )

        aspect_ratio = heuristics["content_analysis"]["aspect_ratio"]
        assert aspect_ratio > 2.0
        assert any("narrow" in rec.lower() for rec in heuristics["recommendations"])


class TestProcessingTimeEstimation:
    """Test processing time estimation."""

    def test_estimate_processing_time_tesseract(self) -> None:
        """Test processing time estimation for Tesseract."""
        estimate = estimate_processing_time(1000, 1000, "tesseract")

        assert estimate["megapixels"] == 1.0
        assert str(estimate["backend"]) == "tesseract"
        assert float(estimate["estimated_seconds"]) > 0
        assert float(estimate["scaling_factor"]) >= 1.0

    def test_estimate_processing_time_large_image(self) -> None:
        """Test processing time estimation for large images."""
        estimate = estimate_processing_time(5000, 5000, "tesseract")

        assert estimate["megapixels"] == 25.0
        assert float(estimate["scaling_factor"]) > 1.0  # Non-linear scaling
        large_time = estimate["estimated_seconds"]

        # Compare with smaller image
        small_estimate = estimate_processing_time(1000, 1000, "tesseract")
        small_time = small_estimate["estimated_seconds"]

        assert float(large_time) > float(small_time) * 20  # Should be much slower than linear

    def test_estimate_processing_time_different_backends(self) -> None:
        """Test processing time estimation varies by backend."""
        tesseract_est = estimate_processing_time(2000, 2000, "tesseract")
        easyocr_est = estimate_processing_time(2000, 2000, "easyocr")
        paddleocr_est = estimate_processing_time(2000, 2000, "paddleocr")

        # EasyOCR should be slower than Tesseract
        assert float(easyocr_est["estimated_seconds"]) > float(tesseract_est["estimated_seconds"])

        # All should have same megapixels
        assert (
            float(tesseract_est["megapixels"]) == float(easyocr_est["megapixels"]) == float(paddleocr_est["megapixels"])
        )
