```python title="disk_cache.py"
from pathlib import Path
from xberg import ExtractInput, Xberg, ExtractionConfig, CacheConfig

cache_dir = Path.home() / ".cache" / "xberg"
cache_dir.mkdir(parents=True, exist_ok=True)

config = ExtractionConfig(
    use_cache=True,
    cache_config=CacheConfig(
        cache_path=str(cache_dir),
        max_cache_size=500 * 1024 * 1024,
        cache_ttl_seconds=7 * 86400,
        enable_compression=True,
    ),
)

xberg = Xberg(config)

print("First extraction (will be cached)...")
result1 = xberg.extract(ExtractInput.from_uri("document.pdf"), ExtractionConfig())
print(f"  - Content length: {len(result1.results[0].content)}")
print(f"  - Cached: {result1.results[0].metadata.get('was_cached', False)}")

print("\nSecond extraction (from cache)...")
result2 = xberg.extract(ExtractInput.from_uri("document.pdf"), ExtractionConfig())
print(f"  - Content length: {len(result2.results[0].content)}")
print(f"  - Cached: {result2.results[0].metadata.get('was_cached', False)}")

print(f"\nResults are identical: {result1.results[0].content == result2.results[0].content}")

cache_stats = xberg.get_cache_stats()
print(f"\nCache Statistics:")
print(f"  - Total entries: {cache_stats.get('total_entries', 0)}")
print(f"  - Cache size: {cache_stats.get('cache_size_bytes', 0) / 1024 / 1024:.1f} MB")
print(f"  - Hit rate: {cache_stats.get('hit_rate', 0):.1%}")
```
