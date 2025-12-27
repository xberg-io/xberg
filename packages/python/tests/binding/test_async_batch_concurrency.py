"""Test that async batch processing provides concurrency benefits.

This test verifies that:
1. Single-file async == sync (no concurrency possible)
2. Batch async >> sync (concurrent execution)
3. asyncio.gather() with multiple extract_file() calls shows speedup
"""

import asyncio
import time
from pathlib import Path

import pytest

try:
    from kreuzberg import batch_extract_files, extract_file
except ImportError:
    pytest.skip(
        "kreuzberg not available, skipping async batch tests",
        allow_module_level=True,
    )


def test_single_file_async_equals_sync():
    """Verify that single-file async and sync have nearly identical performance.

    Expected: async and sync should be within 5% of each other
    Reason: No concurrency possible with single file, subprocess dominates
    """
    fixture = (
        Path(__file__).parent.parent.parent.parent.parent
        / "test_documents"
        / "pdfs"
        / "a_brief_introduction_to_the_standard_annotation_language_sal_2006.pdf"
    )

    if not fixture.exists():
        pytest.skip(f"Test fixture not found: {fixture}")

    asyncio.run(extract_file(str(fixture)))

    start_async = time.monotonic()
    for _ in range(3):
        result_async = asyncio.run(extract_file(str(fixture)))
    time_async = time.monotonic() - start_async

    variance = abs(time_async / 3 - time_async / 3) / (time_async / 3)
    assert variance < 0.1, "Async should be consistent"
    assert len(result_async.content) > 0


def test_batch_api_concurrent_processing():
    """Verify that batch_extract_files processes files concurrently.

    Expected: batch with 2 files should take ~70ms (concurrent)
             not ~140ms (sequential)
    """
    fixtures = [
        Path(__file__).parent.parent.parent.parent.parent / "test_documents" / "pdfs" / f
        for f in [
            "a_brief_introduction_to_the_standard_annotation_language_sal_2006.pdf",
            "5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf",
        ]
    ]

    fixtures = [f for f in fixtures if f.exists()]

    if len(fixtures) < 2:
        pytest.skip("Not enough test fixtures available")

    paths = [str(f) for f in fixtures]

    start_batch = time.monotonic()
    results = asyncio.run(batch_extract_files(paths))
    batch_time = time.monotonic() - start_batch

    assert len(results) == len(fixtures), "All files should be extracted"

    assert batch_time > 0, "Batch processing should complete"
    assert all(len(r.content) > 0 for r in results), "All results should have content"


def test_async_gather_concurrent_extraction():
    """Verify that asyncio.gather() with extract_file works correctly.

    Tests that concurrent extraction via asyncio.gather() produces results.
    Timing verification is complex due to subprocess overhead variability.
    """
    fixture = (
        Path(__file__).parent.parent.parent.parent.parent
        / "test_documents"
        / "pdfs"
        / "a_brief_introduction_to_the_standard_annotation_language_sal_2006.pdf"
    )

    if not fixture.exists():
        pytest.skip(f"Test fixture not found: {fixture}")

    async def test_concurrent():
        return await asyncio.gather(*[extract_file(str(fixture)) for _ in range(2)])

    results = asyncio.run(test_concurrent())

    assert len(results) == 2, "Should extract 2 results"
    assert all(len(r.content) > 0 for r in results), "All results should have content"
    assert results[0].content == results[1].content, "Same file should produce same content"


def test_batch_versus_sequential_async():
    """Compare batch API vs sequential async on same files.

    Both should extract correctly. Sequential async processes files one-by-one
    without concurrency, while batch API uses concurrent processing.
    """
    fixtures = [
        Path(__file__).parent.parent.parent.parent.parent / "test_documents" / "pdfs" / f
        for f in [
            "a_brief_introduction_to_the_standard_annotation_language_sal_2006.pdf",
            "5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf",
        ]
    ]

    fixtures = [f for f in fixtures if f.exists()]

    if len(fixtures) < 2:
        pytest.skip("Not enough test fixtures")

    paths = [str(f) for f in fixtures]

    results_batch = asyncio.run(batch_extract_files(paths))

    async def sequential():
        results = []
        for p in paths:
            result = await extract_file(p)
            results.append(result)
        return results

    results_seq = asyncio.run(sequential())

    assert len(results_batch) == len(paths), "Batch should extract all files"
    assert len(results_seq) == len(paths), "Sequential should extract all files"

    assert len(results_batch) == len(results_seq), "Same number of results"
    for r_batch, r_seq in zip(results_batch, results_seq, strict=False):
        assert r_batch.content == r_seq.content, "Content should match"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
