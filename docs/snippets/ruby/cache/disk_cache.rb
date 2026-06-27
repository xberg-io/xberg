```ruby title="disk_cache.rb"
require 'xberg'
require 'fileutils'

cache_dir = File.expand_path('~/.cache/xberg')
FileUtils.mkdir_p(cache_dir)

config = Xberg::ExtractionConfig.new(
  use_cache: true,
  cache_config: Xberg::CacheConfig.new(
    cache_path: cache_dir,
    max_cache_size: 500 * 1024 * 1024,
    cache_ttl_seconds: 7 * 86400,
    enable_compression: true,
  )
)

puts "First extraction (will be cached)..."
input1 = Xberg::ExtractInput.new(uri: 'document.pdf')
result1 = Xberg.extract(input1, config)
doc1 = result1.results.first
puts "  - Content length: #{doc1.content.length}"
puts "  - Cached: #{doc1.metadata['was_cached']}"

puts "\nSecond extraction (from cache)..."
input2 = Xberg::ExtractInput.new(uri: 'document.pdf')
result2 = Xberg.extract(input2, config)
doc2 = result2.results.first
puts "  - Content length: #{doc2.content.length}"
puts "  - Cached: #{doc2.metadata['was_cached']}"

puts "\nResults are identical: #{doc1.content == doc2.content}"

cache_stats = Xberg.get_cache_stats
puts "\nCache Statistics:"
puts "  - Total entries: #{cache_stats['total_entries']}"
puts "  - Cache size: #{(cache_stats['cache_size_bytes'] / 1024.0 / 1024.0).round(1)} MB"
puts "  - Hit rate: #{(cache_stats['hit_rate'] * 100).round(1)}%"
```
