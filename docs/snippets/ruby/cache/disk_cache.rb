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

xberg = Xberg::Client.new(config)

puts "First extraction (will be cached)..."
result1 = xberg.extract('document.pdf')
puts "  - Content length: #{result1.content.length}"
puts "  - Cached: #{result1.metadata['was_cached']}"

puts "\nSecond extraction (from cache)..."
result2 = xberg.extract('document.pdf')
puts "  - Content length: #{result2.content.length}"
puts "  - Cached: #{result2.metadata['was_cached']}"

puts "\nResults are identical: #{result1.content == result2.content}"

cache_stats = xberg.get_cache_stats
puts "\nCache Statistics:"
puts "  - Total entries: #{cache_stats['total_entries']}"
puts "  - Cache size: #{(cache_stats['cache_size_bytes'] / 1024.0 / 1024.0).round(1)} MB"
puts "  - Hit rate: #{(cache_stats['hit_rate'] * 100).round(1)}%"
```
