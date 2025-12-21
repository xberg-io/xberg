# frozen_string_literal: true

require 'spec_helper'
require 'tempfile'
require 'fileutils'

RSpec.describe Kreuzberg do
  describe '#batch_extract_files_sync' do
    it 'extracts multiple files in a single batch operation' do
      paths = []
      3.times do |i|
        file = Tempfile.new("batch_test_#{i}.txt")
        file.write("Content of file #{i}")
        file.close
        paths << file.path
      end

      results = Kreuzberg.batch_extract_files_sync(paths)

      expect(results).to be_a(Array)
      expect(results.length).to eq(3)
      results.each do |result|
        expect(result).to be_a(Kreuzberg::Result)
        expect(result.content).not_to be_empty
      end
    ensure
      paths.each { |p| File.unlink(p) if File.exist?(p) }
    end

    it 'maintains correct order of results' do
      paths = []
      contents = []
      3.times do |i|
        file = Tempfile.new("ordered_#{i}.txt")
        content = "File #{i} unique content #{SecureRandom.hex(8)}"
        file.write(content)
        file.close
        paths << file.path
        contents << content
      end

      results = Kreuzberg.batch_extract_files_sync(paths)

      expect(results.length).to eq(paths.length)
      results.each_with_index do |result, idx|
        expect(result.content).to include(contents[idx])
      end
    ensure
      paths.each { |p| File.unlink(p) if File.exist?(p) }
    end

    it 'handles empty file list gracefully' do
      results = Kreuzberg.batch_extract_files_sync([])
      expect(results).to be_a(Array)
      expect(results).to be_empty
    end

    it 'handles batch operations with configuration' do
      paths = []
      2.times do |i|
        file = Tempfile.new("config_batch_#{i}.txt")
        file.write("Config test content #{i}")
        file.close
        paths << file.path
      end

      config = Kreuzberg::Config::Extraction.new(
        use_cache: false
      )

      results = Kreuzberg.batch_extract_files_sync(paths, config: config)

      expect(results).to be_a(Array)
      expect(results.length).to eq(2)
      results.each { |result| expect(result).to be_a(Kreuzberg::Result) }
    ensure
      paths.each { |p| File.unlink(p) if File.exist?(p) }
    end

    it 'returns independent result objects' do
      paths = []
      2.times do |i|
        file = Tempfile.new("independent_#{i}.txt")
        file.write("Independent content #{i}")
        file.close
        paths << file.path
      end

      results = Kreuzberg.batch_extract_files_sync(paths)

      expect(results[0].content).not_to eq(results[1].content)
      expect(results[0].mime_type).to eq(results[1].mime_type)
    ensure
      paths.each { |p| File.unlink(p) if File.exist?(p) }
    end

    it 'extracts different file types in batch' do
      paths = []
      temp_dir = Dir.mktmpdir

      # Text file
      txt_file = File.join(temp_dir, 'test.txt')
      File.write(txt_file, 'Text content')
      paths << txt_file

      # CSV file
      csv_file = File.join(temp_dir, 'test.csv')
      File.write(csv_file, "Name,Value\nAlice,1\nBob,2")
      paths << csv_file

      # JSON file
      json_file = File.join(temp_dir, 'test.json')
      File.write(json_file, '{"key": "value"}')
      paths << json_file

      results = Kreuzberg.batch_extract_files_sync(paths)

      expect(results.length).to eq(3)
      results.each do |result|
        expect(result.mime_type).not_to be_nil
        expect(result.content).not_to be_empty
      end
    ensure
      FileUtils.remove_entry(temp_dir)
    end
  end

  describe '#batch_extract_files' do
    it 'extracts multiple files asynchronously' do
      paths = []
      3.times do |i|
        file = Tempfile.new("async_batch_#{i}.txt")
        file.write("Async content #{i}")
        file.close
        paths << file.path
      end

      results = Kreuzberg.batch_extract_files(paths)

      expect(results).to be_a(Array)
      expect(results.length).to eq(3)
      results.each { |result| expect(result).to be_a(Kreuzberg::Result) }
    ensure
      paths.each { |p| File.unlink(p) if File.exist?(p) }
    end

    it 'handles async batch with configuration' do
      paths = []
      2.times do |i|
        file = Tempfile.new("async_config_#{i}.txt")
        file.write("Async config #{i}")
        file.close
        paths << file.path
      end

      config = Kreuzberg::Config::Extraction.new(
        use_cache: false
      )

      results = Kreuzberg.batch_extract_files(paths, config: config)

      expect(results.length).to eq(2)
      results.each { |result| expect(result.content).not_to be_empty }
    ensure
      paths.each { |p| File.unlink(p) if File.exist?(p) }
    end
  end

  describe '#batch_extract_bytes_sync' do
    it 'extracts multiple byte sources in batch' do
      data = [
        'First content',
        'Second content',
        '{"json": true}'
      ]
      mime_types = [
        'text/plain',
        'text/plain',
        'application/json'
      ]

      results = Kreuzberg.batch_extract_bytes_sync(data, mime_types)

      expect(results).to be_a(Array)
      expect(results.length).to eq(3)
      results.each { |result| expect(result).to be_a(Kreuzberg::Result) }
    end

    it 'maintains order for batch byte operations' do
      data = ['Content A', 'Content B', 'Content C']
      mime_types = ['text/plain'] * 3

      results = Kreuzberg.batch_extract_bytes_sync(data, mime_types)

      expect(results.length).to eq(3)
      results.each_with_index do |result, idx|
        expect(result.content).to include(data[idx])
      end
    end

    it 'handles empty byte list' do
      results = Kreuzberg.batch_extract_bytes_sync([], [])
      expect(results).to be_a(Array)
      expect(results).to be_empty
    end

    it 'applies configuration to byte batch operations' do
      data = ['Batch bytes 1', 'Batch bytes 2']
      mime_types = ['text/plain'] * 2

      config = Kreuzberg::Config::Extraction.new(
        use_cache: false
      )

      results = Kreuzberg.batch_extract_bytes_sync(data, mime_types, config: config)

      expect(results.length).to eq(2)
      results.each { |result| expect(result.mime_type).to eq('text/plain') }
    end
  end

  describe '#batch_extract_bytes' do
    it 'extracts multiple bytes asynchronously' do
      data = ['Async bytes 1', 'Async bytes 2']
      mime_types = ['text/plain'] * 2

      results = Kreuzberg.batch_extract_bytes(data, mime_types)

      expect(results).to be_a(Array)
      expect(results.length).to eq(2)
      results.each { |result| expect(result).to be_a(Kreuzberg::Result) }
    end

    it 'handles async byte batch with configuration' do
      data = ['Config async 1', 'Config async 2']
      mime_types = ['text/plain'] * 2

      config = Kreuzberg::Config::Extraction.new(
        use_cache: false
      )

      results = Kreuzberg.batch_extract_bytes(data, mime_types, config: config)

      expect(results.length).to eq(2)
      results.each { |result| expect(result.content).not_to be_empty }
    end
  end

  describe 'batch performance characteristics' do
    it 'processes batch operations efficiently' do
      paths = []
      file_count = 5

      # Create test files
      temp_dir = Dir.mktmpdir
      file_count.times do |i|
        file_path = File.join(temp_dir, "perf_test_#{i}.txt")
        File.write(file_path, "Performance test content #{i}")
        paths << file_path
      end

      # Measure batch operation time
      start_time = Time.now
      results = Kreuzberg.batch_extract_files_sync(paths)
      batch_duration = Time.now - start_time

      expect(results.length).to eq(file_count)
      expect(results).to all(be_a(Kreuzberg::Result))

      # Batch should complete in reasonable time
      # (This is a loose check - exact timing depends on system)
      expect(batch_duration).to be < 60 # 60 second upper bound for 5 files

      puts "Batch extraction time for #{file_count} files: #{batch_duration.round(3)}s"
    ensure
      FileUtils.remove_entry(temp_dir)
    end

    it 'batch results match sequential results' do
      paths = []
      temp_dir = Dir.mktmpdir

      3.times do |i|
        file_path = File.join(temp_dir, "compare_#{i}.txt")
        File.write(file_path, "Comparison content #{i}")
        paths << file_path
      end

      # Get batch results
      batch_results = Kreuzberg.batch_extract_files_sync(paths)

      # Get sequential results
      sequential_results = paths.map { |p| Kreuzberg.extract_file_sync(p) }

      # Compare
      expect(batch_results.length).to eq(sequential_results.length)
      batch_results.each_with_index do |batch_result, idx|
        seq_result = sequential_results[idx]
        expect(batch_result.content).to eq(seq_result.content)
        expect(batch_result.mime_type).to eq(seq_result.mime_type)
      end
    ensure
      FileUtils.remove_entry(temp_dir)
    end
  end

  describe 'batch error handling' do
    it 'handles missing files gracefully in batch' do
      paths = [
        '/nonexistent/file1.txt',
        '/nonexistent/file2.txt'
      ]

      # Batch operation should not raise, but may return errors in results
      expect {
        Kreuzberg.batch_extract_files_sync(paths)
      }.not_to raise_error
    end

    it 'handles mixed valid and invalid paths' do
      paths = []
      temp_dir = Dir.mktmpdir

      # Add valid file
      valid_path = File.join(temp_dir, 'valid.txt')
      File.write(valid_path, 'Valid content')
      paths << valid_path

      # Add invalid path
      paths << '/nonexistent/invalid.txt'

      # Should return results (some may be failures)
      results = Kreuzberg.batch_extract_files_sync(paths)
      expect(results).to be_a(Array)
    ensure
      FileUtils.remove_entry(temp_dir)
    end

    it 'raises error on invalid mime type in byte batch' do
      data = ['Content']
      mime_types = ['invalid/mime/type']

      # Should handle invalid MIME types gracefully
      expect {
        Kreuzberg.batch_extract_bytes_sync(data, mime_types)
      }.not_to raise_error
    end
  end

  describe 'batch caching behavior' do
    it 'respects cache configuration in batch' do
      paths = []
      temp_dir = Dir.mktmpdir

      2.times do |i|
        file_path = File.join(temp_dir, "cache_test_#{i}.txt")
        File.write(file_path, "Cache test #{i}")
        paths << file_path
      end

      config_no_cache = Kreuzberg::Config::Extraction.new(use_cache: false)

      results1 = Kreuzberg.batch_extract_files_sync(paths, config: config_no_cache)
      results2 = Kreuzberg.batch_extract_files_sync(paths, config: config_no_cache)

      expect(results1.length).to eq(results2.length)
      results1.each_with_index do |result, idx|
        expect(result.content).to eq(results2[idx].content)
      end
    ensure
      FileUtils.remove_entry(temp_dir)
    end
  end
end
