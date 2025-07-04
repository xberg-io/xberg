"""Direct serialization performance comparison: JSON vs msgpack."""

import time
import statistics
from pathlib import Path

from kreuzberg._types import ExtractionResult


def benchmark_serialization() -> dict[str, object]:
    """Benchmark serialization performance with statistical rigor."""

    large_content = "This is a realistic OCR result content. " * 500
    test_result = ExtractionResult(
        content=large_content,
        mime_type="text/plain",
        metadata={  # type: ignore[typeddict-unknown-key]
            "file_path": "/some/long/path/to/document.pdf",
            "ocr_backend": "tesseract",
            "ocr_config": {"language": "eng", "psm": 3},
            "processing_time": 15.234,
            "confidence_scores": [0.95, 0.87, 0.92, 0.88, 0.94],
            "page_count": 10,
        },
        chunks=["chunk1", "chunk2", "chunk3"] * 20,
    )

    cache_data = {
        "type": "ExtractionResult",
        "data": test_result,
        "cached_at": time.time(),
    }

    print("🔬 SERIALIZATION BENCHMARK")
    print(f"Data size: ~{len(large_content) // 1024}KB content + metadata")
    print("Trials: 1000 each operation")
    print("=" * 60)

    print("\n📦 MSGPACK PERFORMANCE")
    from msgspec.msgpack import encode as msgpack_encode, decode as msgpack_decode

    msgpack_serialize_times = []
    for _ in range(1000):
        start = time.perf_counter()
        msgpack_data = msgpack_encode(cache_data)
        msgpack_serialize_times.append(time.perf_counter() - start)

    msgpack_deserialize_times = []
    for _ in range(1000):
        start = time.perf_counter()
        msgpack_decode(msgpack_data, type=dict, strict=False)
        msgpack_deserialize_times.append(time.perf_counter() - start)

    msgpack_size = len(msgpack_data)

    print("\n📄 JSON PERFORMANCE")
    from msgspec.json import encode as json_encode, decode as json_decode

    json_serialize_times = []
    for _ in range(1000):
        start = time.perf_counter()
        json_data = json_encode(cache_data)
        json_serialize_times.append(time.perf_counter() - start)

    json_deserialize_times = []
    for _ in range(1000):
        start = time.perf_counter()
        json_decode(json_data, type=dict, strict=False)
        json_deserialize_times.append(time.perf_counter() - start)

    json_size = len(json_data)

    def analyze_times(times: list[float], name: str) -> dict[str, object]:
        mean_val = statistics.mean(times)
        stdev_val = statistics.stdev(times) if len(times) > 1 else 0
        median_val = statistics.median(times)
        min_val = min(times)
        max_val = max(times)

        print(f"  {name}:")
        print(f"    Mean:   {mean_val * 1000:.3f}ms ± {stdev_val * 1000:.3f}ms")
        print(f"    Median: {median_val * 1000:.3f}ms")
        print(f"    Range:  {min_val * 1000:.3f}ms - {max_val * 1000:.3f}ms")

        return {
            "mean": mean_val,
            "stdev": stdev_val,
            "median": median_val,
            "min": min_val,
            "max": max_val,
        }

    print("\n📊 RESULTS")
    print("=" * 60)

    msgpack_serialize = analyze_times(msgpack_serialize_times, "Msgpack Serialize")
    msgpack_deserialize = analyze_times(
        msgpack_deserialize_times, "Msgpack Deserialize"
    )

    json_serialize = analyze_times(json_serialize_times, "JSON Serialize")
    json_deserialize = analyze_times(json_deserialize_times, "JSON Deserialize")

    # Type casting for arithmetic operations
    json_ser_mean = json_serialize["mean"]
    json_deser_mean = json_deserialize["mean"]
    msgpack_ser_mean = msgpack_serialize["mean"]
    msgpack_deser_mean = msgpack_deserialize["mean"]
    assert isinstance(json_ser_mean, (int, float))
    assert isinstance(json_deser_mean, (int, float))
    assert isinstance(msgpack_ser_mean, (int, float))
    assert isinstance(msgpack_deser_mean, (int, float))

    serialize_speedup = json_ser_mean / msgpack_ser_mean
    deserialize_speedup = json_deser_mean / msgpack_deser_mean
    total_speedup = (json_ser_mean + json_deser_mean) / (
        msgpack_ser_mean + msgpack_deser_mean
    )

    size_ratio = json_size / msgpack_size

    print("\n🏁 COMPARISON")
    print("=" * 60)
    print(f"Serialize speedup:   Msgpack is {serialize_speedup:.1f}x faster")
    print(f"Deserialize speedup: Msgpack is {deserialize_speedup:.1f}x faster")
    print(f"Total speedup:       Msgpack is {total_speedup:.1f}x faster")
    print(f"Size comparison:     Msgpack is {size_ratio:.1f}x smaller")
    print(f"Msgpack size:        {msgpack_size:,} bytes")
    print(f"JSON size:           {json_size:,} bytes")

    return {
        "msgpack": {
            "serialize": msgpack_serialize,
            "deserialize": msgpack_deserialize,
            "size": msgpack_size,
        },
        "json": {
            "serialize": json_serialize,
            "deserialize": json_deserialize,
            "size": json_size,
        },
        "speedup": {
            "serialize": serialize_speedup,
            "deserialize": deserialize_speedup,
            "total": total_speedup,
            "size_ratio": size_ratio,
        },
    }


if __name__ == "__main__":
    try:
        results = benchmark_serialization()

        import json

        results_file = Path("serialization_benchmark_results.json")
        with results_file.open("w") as f:
            json.dump(results, f, indent=2, default=str)

        print(f"\n💾 Results saved to {results_file}")

    except Exception as e:
        print(f"❌ Benchmark failed: {e}")
        import traceback

        traceback.print_exc()
