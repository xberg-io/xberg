```ruby title="simple_benchmark.rb"
require 'xberg'
require 'benchmark'

config = Xberg::ExtractionConfig.new(use_cache: false)
xberg = Xberg::Client.new(config)
file_path = 'document.pdf'
num_runs = 10

puts "Sync extraction (#{num_runs} runs):"
sync_time = Benchmark.realtime do
  num_runs.times do
    xberg.extract(file_path)
  end
end
avg_sync = sync_time / num_runs
puts "  - Total time: #{sync_time.round(3)}s"
puts "  - Average: #{avg_sync.round(3)}s per extraction"

puts "\nAsync extraction (#{num_runs} parallel runs):"
async_time = Benchmark.realtime do
  threads = num_runs.times.map do
    Thread.new { xberg.extract(file_path) }
  end
  threads.map(&:join)
end
puts "  - Total time: #{async_time.round(3)}s"
puts "  - Average: #{(async_time / num_runs).round(3)}s per extraction"
puts "  - Speedup: #{(sync_time / async_time).round(1)}x"

cache_config = Xberg::ExtractionConfig.new(use_cache: true)
xberg_cached = Xberg::Client.new(cache_config)

puts "\nFirst extraction (populates cache)..."
first_time = Benchmark.realtime do
  xberg_cached.extract(file_path)
end
puts "  - Time: #{first_time.round(3)}s"

puts "Second extraction (from cache)..."
cached_time = Benchmark.realtime do
  xberg_cached.extract(file_path)
end
puts "  - Time: #{cached_time.round(3)}s"
puts "  - Cache speedup: #{(first_time / cached_time).round(1)}x"
```
