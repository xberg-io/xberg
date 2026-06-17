#!/usr/bin/env ruby
# frozen_string_literal: true


require 'json'
require 'time'

DEBUG = ENV.fetch('KREUZBERG_BENCHMARK_DEBUG', 'false') == 'true'

def debug_log(message)
  return unless DEBUG
  warn "[DEBUG] #{Time.now.iso8601(3)} - #{message}"
end

def peak_memory_bytes
  if File.exist?('/proc/self/status')
    match = File.read('/proc/self/status').match(/VmRSS:\s+(\d+)/)
    (match[1].to_i * 1024) if match
  else
    `ps -o rss= -p #{Process.pid}`.strip.to_i * 1024
  end
rescue StandardError
  0
end

debug_log "=== Gem Initialization Debug Info ==="
debug_log "RUBY_PLATFORM: #{RUBY_PLATFORM}"
debug_log "RUBY_VERSION: #{RUBY_VERSION}"
debug_log "LD_LIBRARY_PATH: #{ENV['LD_LIBRARY_PATH'] || 'NOT SET'}"
debug_log "DYLD_LIBRARY_PATH: #{ENV['DYLD_LIBRARY_PATH'] || 'NOT SET'}"
debug_log "LD_LIBRARY_PATH entries:"
(ENV['LD_LIBRARY_PATH'] || '').split(':').filter_map { |p| p if File.directory?(p) }.each do |dir|
  debug_log "  [OK] #{dir}"
end
(ENV['LD_LIBRARY_PATH'] || '').split(':').filter_map { |p| p unless File.directory?(p) || p.empty? }.each do |dir|
  debug_log "  [MISSING] #{dir}"
end

begin
  debug_log "Loading kreuzberg gem..."
  require 'kreuzberg'
  debug_log "Successfully loaded kreuzberg gem"
rescue LoadError => e
  debug_log "FAILED to load kreuzberg gem: #{e.class} - #{e.message}"
  debug_log "Backtrace:\n#{e.backtrace.join("\n")}"

  debug_log "Attempting to find kreuzberg library files:"
  require 'rbconfig'
  gem_root = Gem.loaded_specs['kreuzberg_rb']&.gem_root
  debug_log "Gem root: #{gem_root || 'NOT FOUND'}"

  if gem_root
    lib_dir = File.join(gem_root, 'lib')
    debug_log "Lib directory: #{lib_dir} (exists: #{File.directory?(lib_dir)})"
    if File.directory?(lib_dir)
      debug_log "Contents:"
      Dir.glob("#{lib_dir}/**/*").each { |f| debug_log "  - #{f}" }
    end
  end

  raise
end
debug_log "=== Initialization Complete ===" if DEBUG

# Determine if OCR was actually used based on extraction result metadata.
# Mirrors the native Rust adapter logic: OCR is used when format_type is "ocr",
# or when format_type is "image"/"pdf" and OCR was enabled in config.
def determine_ocr_used(metadata, ocr_enabled)
  format_type = metadata&.dig('format_type') || metadata&.dig(:format_type) || ''
  return true if format_type == 'ocr'
  return true if (format_type == 'image' || format_type == 'pdf') && ocr_enabled

  false
end

def parse_request(line)
  stripped = line.strip
  if stripped.start_with?('{')
    begin
      req = JSON.parse(stripped)
      return [req['path'] || '', req['force_ocr'] || false]
    rescue JSON::ParserError
      # Fall through to plain path
    end
  end
  [stripped, false]
end

def extract_sync(file_path, config = {})
  debug_log "=== SYNC EXTRACTION START ==="
  debug_log "Input: file_path=#{file_path}"
  debug_log "File exists: #{File.exist?(file_path)}"
  debug_log "File size: #{File.size(file_path)} bytes" if File.exist?(file_path)

  start_monotonic = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  start_wall = Time.now
  debug_log "Timing start (monotonic): #{start_monotonic.round(6)}, wall: #{start_wall.iso8601(6)}"

  result = Kreuzberg.extract_file(path: file_path, config: config)

  end_monotonic = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  end_wall = Time.now
  duration_s = end_monotonic - start_monotonic
  duration_ms = duration_s * 1000.0

  debug_log "Timing end (monotonic): #{end_monotonic.round(6)}, wall: #{end_wall.iso8601(6)}"
  debug_log "Duration (seconds): #{duration_s.round(6)}"
  debug_log "Duration (milliseconds): #{duration_ms.round(3)}"
  debug_log "Result class: #{result.class}"
  debug_log "Result has content: #{!result.content.nil?}"
  debug_log "Content length: #{result.content&.length || 'nil'} characters"
  debug_log "Result has metadata: #{!result.metadata.nil?}"
  debug_log "Metadata type: #{result.metadata&.class || 'nil'}"

  metadata = result.metadata || {}
  ocr_enabled = config.dig(:ocr, :enabled) || false
  payload = {
    content: result.content,
    metadata: metadata,
    _extraction_time_ms: duration_ms,
    _ocr_used: determine_ocr_used(metadata, ocr_enabled),
    _peak_memory_bytes: peak_memory_bytes
  }

  debug_log "Output JSON size: #{JSON.generate(payload).bytesize} bytes"
  debug_log "=== SYNC EXTRACTION END ==="

  payload
rescue StandardError => e
  debug_log "ERROR during sync extraction: #{e.class} - #{e.message}"
  debug_log "Backtrace:\n#{e.backtrace.join("\n")}"
  raise
end

def extract_batch(file_paths, config = {})
  debug_log "=== BATCH EXTRACTION START ==="
  debug_log "Input: #{file_paths.length} files"
  file_paths.each_with_index do |path, idx|
    debug_log "  [#{idx}] #{path} (exists: #{File.exist?(path)}, size: #{File.exist?(path) ? File.size(path) : 'N/A'} bytes)"
  end

  start_monotonic = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  start_wall = Time.now
  debug_log "Timing start (monotonic): #{start_monotonic.round(6)}, wall: #{start_wall.iso8601(6)}"

  results = Kreuzberg.batch_extract_files_sync(paths: file_paths, config: config)

  end_monotonic = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  end_wall = Time.now
  total_duration_s = end_monotonic - start_monotonic
  total_duration_ms = total_duration_s * 1000.0

  debug_log "Timing end (monotonic): #{end_monotonic.round(6)}, wall: #{end_wall.iso8601(6)}"
  debug_log "Total duration (seconds): #{total_duration_s.round(6)}"
  debug_log "Total duration (milliseconds): #{total_duration_ms.round(3)}"
  debug_log "Results count: #{results.length}"

  per_file_duration_ms = file_paths.length.positive? ? total_duration_ms / file_paths.length : 0
  debug_log "Per-file average duration (milliseconds): #{per_file_duration_ms.round(3)}"

  ocr_enabled = config.dig(:ocr, :enabled) || false
  peak_mem = peak_memory_bytes
  results_with_timing = results.map.with_index do |result, idx|
    debug_log "  Result[#{idx}] - content length: #{result.content&.length || 'nil'}, has metadata: #{!result.metadata.nil?}"
    metadata = result.metadata || {}
    {
      content: result.content,
      metadata: metadata,
      _extraction_time_ms: per_file_duration_ms,
      _batch_total_ms: total_duration_ms,
      _ocr_used: determine_ocr_used(metadata, ocr_enabled),
      _peak_memory_bytes: peak_mem
    }
  end

  debug_log "=== BATCH EXTRACTION END ==="

  results_with_timing
rescue StandardError => e
  debug_log "ERROR during batch extraction: #{e.class} - #{e.message}"
  debug_log "Backtrace:\n#{e.backtrace.join("\n")}"
  raise
end

def extract_server(ocr_enabled)
  debug_log "=== SERVER MODE START ==="

  # Signal readiness after Ruby + native extension initialization
  puts "READY"
  $stdout.flush

  STDIN.each_line do |line|
    file_path, force_ocr = parse_request(line)
    next if file_path.empty?

    debug_log "Processing file: #{file_path}, force_ocr: #{force_ocr}"
    begin
      config = {
        use_cache: false
      }
      config[:ocr] = { enabled: true } if (ocr_enabled || force_ocr)

      start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
      result = Kreuzberg.extract_file(path: file_path, config: config)
      duration_ms = (Process.clock_gettime(Process::CLOCK_MONOTONIC) - start) * 1000.0

      metadata = result.metadata || {}
      payload = {
        content: result.content,
        metadata: metadata,
        _extraction_time_ms: duration_ms,
        _ocr_used: determine_ocr_used(metadata, (ocr_enabled || force_ocr)),
        _peak_memory_bytes: peak_memory_bytes
      }

      puts JSON.generate(payload)
      $stdout.flush
    rescue StandardError => e
      error_payload = {
        error: e.message,
        _extraction_time_ms: 0,
        _ocr_used: false
      }
      puts JSON.generate(error_payload)
      $stdout.flush
    end
  end

  debug_log "=== SERVER MODE END ==="
end

def main
  debug_log "Ruby script started"
  debug_log "ARGV: #{ARGV.inspect}"
  debug_log "ARGV length: #{ARGV.length}"

  ocr_enabled = false
  args = []

  ARGV.each do |arg|
    if arg == '--ocr'
      ocr_enabled = true
    elsif arg == '--no-ocr'
      ocr_enabled = false
    else
      args << arg
    end
  end

  if args.length < 1
    warn 'Usage: kreuzberg_extract.rb [--ocr|--no-ocr] <mode> <file_path> [additional_files...]'
    warn 'Modes: sync, batch, server'
    warn 'Debug mode: set KREUZBERG_BENCHMARK_DEBUG=true to enable debug logging to stderr'
    exit 1
  end

  mode = args[0]
  file_paths = args[1..]

  debug_log "Mode: #{mode}"
  debug_log "OCR enabled: #{ocr_enabled}"
  debug_log "File paths (#{file_paths.length}): #{file_paths.inspect}"

  case mode
  when 'server'
    debug_log "Executing server mode"
    extract_server(ocr_enabled)

  when 'sync'
    if file_paths.length != 1
      warn 'Error: sync mode requires exactly one file'
      exit 1
    end
    debug_log "Executing sync mode with file: #{file_paths[0]}"

    config = {
      use_cache: false
    }
    config[:ocr] = { enabled: true } if ocr_enabled

    payload = extract_sync(file_paths[0], config)
    output = JSON.generate(payload)
    debug_log "Output JSON: #{output}"
    puts output

  when 'batch'
    if file_paths.empty?
      warn 'Error: batch mode requires at least one file'
      exit 1
    end
    debug_log "Executing batch mode with #{file_paths.length} files"

    config = {
      use_cache: false
    }
    config[:ocr] = { enabled: true } if ocr_enabled

    results = extract_batch(file_paths, config)

    if file_paths.length == 1
      output = JSON.generate(results[0])
      debug_log "Output JSON (single file): #{output}"
      puts output
    else
      output = JSON.generate(results)
      debug_log "Output JSON (multiple files): #{output[0..200]}..." if output.length > 200
      puts output
    end

  else
    warn "Error: Unknown mode '#{mode}'. Use sync, batch, or server"
    exit 1
  end

  debug_log "Script completed successfully"
rescue StandardError => e
  debug_log "FATAL ERROR: #{e.class} - #{e.message}"
  debug_log "Backtrace:\n#{e.backtrace.join("\n")}"
  warn "Error extracting with Kreuzberg: #{e.message}"
  exit 1
end

main if __FILE__ == $PROGRAM_NAME
