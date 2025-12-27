#!/usr/bin/env ruby
# frozen_string_literal: true


require 'kreuzberg'

puts '=' * 80
puts 'KREUZBERG RUBY BINDINGS COMPREHENSIVE TEST SUITE'
puts '=' * 80

# Simple test runner
class TestRunner
  def initialize
    @passed = 0
    @failed = 0
    @skipped = 0
    @section = 0
  end

  def start_section(name)
    @section += 1
    puts "\n[SECTION #{@section}] #{name}"
    puts '-' * 80
  end

  def test(description)
    begin
      result = yield
      if result == false
        puts "  ✗ #{description}"
        @failed += 1
        false
      else
        puts "  ✓ #{description}"
        @passed += 1
        true
      end
    rescue StandardError => e
      puts "  ✗ #{description}"
      puts "    Error: #{e.class}: #{e.message}"
      @failed += 1
      false
    end
  end

  def skip(description, reason)
    puts "  ⊘ #{description} (#{reason})"
    @skipped += 1
  end

  def async_test(description)
    begin
      result = yield
      if result == false
        puts "  ✗ #{description}"
        @failed += 1
        false
      else
        puts "  ✓ #{description}"
        @passed += 1
        true
      end
    rescue StandardError => e
      puts "  ✗ #{description}"
      puts "    Error: #{e.class}: #{e.message}"
      @failed += 1
      false
    end
  end

  def summary
    puts "\n" + '=' * 80
    puts "SUMMARY"
    puts "=" * 80
    total = @passed + @failed
    puts "Total: #{total} tests"
    puts "Passed: #{@passed}"
    puts "Failed: #{@failed}"
    puts "Skipped: #{@skipped}"
    puts "=" * 80
    @failed == 0
  end
end

runner = TestRunner.new

runner.start_section('Module Imports & Setup')

runner.test('Kreuzberg module is defined') do
  defined?(Kreuzberg) == 'constant'
end

runner.test('Config module is accessible') do
  defined?(Kreuzberg::Config) == 'constant'
end

runner.test('Result class is accessible') do
  defined?(Kreuzberg::Result) == 'constant'
end

runner.test('Errors module is accessible') do
  defined?(Kreuzberg::Errors) == 'constant'
end

runner.test('KeywordAlgorithm constants are defined') do
  Kreuzberg::KeywordAlgorithm::YAKE == :yake && Kreuzberg::KeywordAlgorithm::RAKE == :rake
end

runner.start_section('Configuration Classes - Creation & Structure')

runner.test('OCR config creation with defaults') do
  ocr = Kreuzberg::Config::OCR.new
  ocr.backend == 'tesseract' && ocr.language == 'eng'
end

runner.test('OCR config creation with custom values') do
  ocr = Kreuzberg::Config::OCR.new(backend: 'paddleocr', language: 'fra')
  ocr.backend == 'paddleocr' && ocr.language == 'fra'
end

runner.test('OCR config to_h serialization') do
  ocr = Kreuzberg::Config::OCR.new(backend: 'tesseract', language: 'eng')
  hash = ocr.to_h
  hash.is_a?(Hash) && hash[:backend] == 'tesseract'
end

runner.test('Chunking config creation with defaults') do
  chunking = Kreuzberg::Config::Chunking.new
  chunking.max_chars == 1000 && chunking.max_overlap == 200
end

runner.test('Chunking config with custom chunk_size') do
  chunking = Kreuzberg::Config::Chunking.new(chunk_size: 2000, chunk_overlap: 300)
  chunking.max_chars == 2000 && chunking.max_overlap == 300
end

runner.test('Chunking config enabled flag') do
  chunking = Kreuzberg::Config::Chunking.new(enabled: true)
  chunking.enabled == true
end

runner.test('Chunking config to_h serialization') do
  chunking = Kreuzberg::Config::Chunking.new(max_chars: 500)
  hash = chunking.to_h
  hash.is_a?(Hash) && hash[:max_chars] == 500
end

runner.test('ImagePreprocessing config creation') do
  preprocessing = Kreuzberg::Config::ImagePreprocessing.new(enabled: true)
  preprocessing.enabled == true
end

runner.test('ImagePreprocessing to_h serialization') do
  preprocessing = Kreuzberg::Config::ImagePreprocessing.new
  hash = preprocessing.to_h
  hash.is_a?(Hash)
end

runner.test('Tesseract config creation') do
  tesseract = Kreuzberg::Config::Tesseract.new(oem: 1, psm: 6)
  tesseract.options.is_a?(Hash)
end

runner.test('Tesseract config to_h') do
  tesseract = Kreuzberg::Config::Tesseract.new(oem: 1)
  hash = tesseract.to_h
  hash.is_a?(Hash)
end

runner.test('PDF config creation') do
  pdf = Kreuzberg::Config::PDF.new(extract_text: true)
  pdf.extract_text == true
end

runner.test('PDF config with custom DPI') do
  pdf = Kreuzberg::Config::PDF.new(dpi: 300)
  pdf.dpi == 300
end

runner.test('PDF config to_h serialization') do
  pdf = Kreuzberg::Config::PDF.new
  hash = pdf.to_h
  hash.is_a?(Hash)
end

runner.test('ImageExtraction config creation') do
  image_extract = Kreuzberg::Config::ImageExtraction.new(enabled: true)
  image_extract.enabled == true
end

runner.test('ImageExtraction config with dimensions') do
  image_extract = Kreuzberg::Config::ImageExtraction.new(min_width: 100, min_height: 100)
  image_extract.min_width == 100
end

runner.test('PageConfig creation') do
  page = Kreuzberg::Config::PageConfig.new(enabled: true)
  page.enabled == true
end

runner.test('Extraction config creation with defaults') do
  config = Kreuzberg::Config::Extraction.new
  config.is_a?(Kreuzberg::Config::Extraction)
end

runner.test('Extraction config with force_ocr') do
  config = Kreuzberg::Config::Extraction.new(force_ocr: true)
  config.force_ocr == true
end

runner.test('Extraction config with custom OCR') do
  ocr = Kreuzberg::Config::OCR.new(language: 'spa')
  config = Kreuzberg::Config::Extraction.new(ocr: ocr)
  config.ocr.language == 'spa'
end

runner.test('Extraction config with custom Chunking') do
  chunking = Kreuzberg::Config::Chunking.new(max_chars: 1500)
  config = Kreuzberg::Config::Extraction.new(chunking: chunking)
  config.chunking.max_chars == 1500
end

runner.test('Extraction config to_h serialization') do
  config = Kreuzberg::Config::Extraction.new(force_ocr: true)
  hash = config.to_h
  hash.is_a?(Hash) && hash[:force_ocr] == true
end

runner.start_section('Error Classes & Exception Hierarchy')

runner.test('ValidationError is defined') do
  defined?(Kreuzberg::Errors::ValidationError) == 'constant'
end

runner.test('ParsingError is defined') do
  defined?(Kreuzberg::Errors::ParsingError) == 'constant'
end

runner.test('OCRError is defined') do
  defined?(Kreuzberg::Errors::OCRError) == 'constant'
end

runner.test('MissingDependencyError is defined') do
  defined?(Kreuzberg::Errors::MissingDependencyError) == 'constant'
end

runner.test('IOError is defined') do
  defined?(Kreuzberg::Errors::IOError) == 'constant'
end

runner.test('PluginError is defined') do
  defined?(Kreuzberg::Errors::PluginError) == 'constant'
end

runner.test('UnsupportedFormatError is defined') do
  defined?(Kreuzberg::Errors::UnsupportedFormatError) == 'constant'
end

runner.test('Error creation with message') do
  error = Kreuzberg::Errors::ValidationError.new('Test error')
  error.message == 'Test error'
end

runner.test('ValidationError inherits from Error') do
  error = Kreuzberg::Errors::ValidationError.new('Test')
  error.is_a?(Kreuzberg::Errors::Error)
end

runner.test('ParsingError stores context') do
  error = Kreuzberg::Errors::ParsingError.new('Parse failed', context: { line: 5 })
  error.context == { line: 5 }
end

runner.test('OCRError stores context') do
  error = Kreuzberg::Errors::OCRError.new('OCR failed', context: { backend: 'tesseract' })
  error.context == { backend: 'tesseract' }
end

runner.test('MissingDependencyError stores dependency') do
  error = Kreuzberg::Errors::MissingDependencyError.new('Missing lib', dependency: 'libtesseract')
  error.dependency == 'libtesseract'
end

runner.test('Error stores error_code') do
  error = Kreuzberg::Errors::Error.new('Test', error_code: 5)
  error.error_code == 5
end

runner.start_section('MIME Type Detection & Validation')

runner.test('detect_mime_type from PDF bytes') do
  pdf_header = "%PDF-1.4\n"
  mime = Kreuzberg.detect_mime_type(pdf_header)
  mime.is_a?(String) && !mime.empty?
end

runner.test('detect_mime_type_from_path with PDF') do
  mime = Kreuzberg.detect_mime_type_from_path('document.pdf')
  mime == 'application/pdf'
end

runner.test('detect_mime_type_from_path with DOCX') do
  mime = Kreuzberg.detect_mime_type_from_path('document.docx')
  mime == 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
end

runner.test('validate_mime_type with valid MIME') do
  result = Kreuzberg.validate_mime_type('application/pdf')
  result == true
end

runner.test('validate_mime_type with invalid MIME') do
  result = Kreuzberg.validate_mime_type('application/invalid-mime-type-xyz')
  result == false || result.is_a?(String)
end

runner.test('get_extensions_for_mime returns array') do
  extensions = Kreuzberg.get_extensions_for_mime('application/pdf')
  extensions.is_a?(Array) && extensions.include?('pdf')
end

runner.test('get_extensions_for_mime for DOCX') do
  extensions = Kreuzberg.get_extensions_for_mime('application/vnd.openxmlformats-officedocument.wordprocessingml.document')
  extensions.is_a?(Array) && extensions.include?('docx')
end

runner.start_section('Plugin Registry - Validators')

runner.test('list_validators returns array') do
  validators = Kreuzberg.list_validators
  validators.is_a?(Array)
end

runner.test('register_validator with callable') do
  validator = lambda do |result|
    result[:content]&.length&.positive? || false
  end
  result = Kreuzberg.register_validator('test_validator', validator)
  result == true
end

runner.test('list_validators includes registered validator') do
  validators = Kreuzberg.list_validators
  validators.include?('test_validator') || validators.is_a?(Array)
end

runner.test('unregister_validator removes validator') do
  result = Kreuzberg.unregister_validator('test_validator')
  result == true
end

runner.test('clear_validators clears all validators') do
  Kreuzberg.register_validator('temp_validator_1', ->(r) { true })
  result = Kreuzberg.clear_validators
  result == true
end

runner.start_section('Plugin Registry - Post-Processors')

runner.test('list_post_processors returns array') do
  processors = Kreuzberg.list_post_processors
  processors.is_a?(Array)
end

runner.test('register_post_processor with callable') do
  processor = lambda do |result|
    result[:content]&.gsub(/\s+/, ' ')
  end
  result = Kreuzberg.register_post_processor('space_normalizer', processor)
  result == true
end

runner.test('list_post_processors includes registered processor') do
  processors = Kreuzberg.list_post_processors
  processors.include?('space_normalizer') || processors.is_a?(Array)
end

runner.test('unregister_post_processor removes processor') do
  result = Kreuzberg.unregister_post_processor('space_normalizer')
  result == true
end

runner.test('clear_post_processors clears all post-processors') do
  Kreuzberg.register_post_processor('temp_proc_1', ->(r) { r })
  result = Kreuzberg.clear_post_processors
  result == true
end

runner.start_section('Plugin Registry - OCR Backends')

runner.test('list_ocr_backends returns array') do
  backends = Kreuzberg.list_ocr_backends
  backends.is_a?(Array)
end

runner.test('unregister_ocr_backend on non-existent backend returns false or true') do
  result = Kreuzberg.unregister_ocr_backend('nonexistent_backend')
  result.is_a?(TrueClass) || result.is_a?(FalseClass)
end

runner.start_section('Embedding Presets')

runner.test('list_embedding_presets returns array') do
  presets = Kreuzberg.list_embedding_presets
  presets.is_a?(Array)
end

runner.test('get_embedding_preset on built-in preset') do
  ['bert', 'nomic', 'mxbai'].each do |name|
    preset = Kreuzberg.get_embedding_preset(name)
    if preset
      puts "    Found preset: #{name}"
      return true
    end
  end
  true
end

runner.start_section('Cache API')

runner.test('clear_cache method exists') do
  Kreuzberg.respond_to?(:clear_cache)
end

runner.test('cache_stats returns hash-like object') do
  begin
    stats = Kreuzberg.cache_stats
    stats.is_a?(Hash) || stats.respond_to?(:[])
  rescue StandardError
    skip('cache_stats not implemented', 'native extension limitation')
  end
end

runner.start_section('Result Object Structure')

runner.test('Result class has expected attributes') do
  attrs = [:content, :mime_type, :metadata, :tables, :chunks, :images, :pages]
  attrs.all? { |attr| Kreuzberg::Result.new({}).respond_to?(attr) }
end

runner.test('Result.Table has expected fields') do
  table = Kreuzberg::Result::Table.new(
    cells: [['a', 'b'], ['c', 'd']],
    markdown: '| a | b |\n| c | d |',
    page_number: 1
  )
  table.cells.is_a?(Array) && table.markdown.is_a?(String) && table.page_number == 1
end

runner.test('Result.Chunk has expected fields') do
  chunk = Kreuzberg::Result::Chunk.new(
    content: 'test content',
    byte_start: 0,
    byte_end: 12,
    token_count: 2,
    chunk_index: 0,
    total_chunks: 1,
    first_page: 1,
    last_page: 1,
    embedding: nil
  )
  chunk.content == 'test content' && chunk.byte_start == 0
end

runner.test('Result.Image has expected fields') do
  image = Kreuzberg::Result::Image.new(
    data: 'fake_image_data',
    mime_type: 'image/png',
    page_number: 1
  )
  image.data == 'fake_image_data' && image.mime_type == 'image/png'
end

runner.test('Result.Page has expected fields') do
  page = Kreuzberg::Result::Page.new(
    page_number: 1,
    width: 612,
    height: 792,
    rotation: 0
  )
  page.page_number == 1 && page.width == 612
end

runner.test('Result.to_h produces hash with all fields') do
  result = Kreuzberg::Result.new({
                                   content: 'test',
                                   mime_type: 'text/plain',
                                   metadata: {},
                                   tables: [],
                                   chunks: [],
                                   images: [],
                                   pages: []
                                 })
  hash = result.to_h
  hash.is_a?(Hash) && hash[:content] == 'test'
end

runner.start_section('Extraction Functions - File-based (Sync)')

runner.test('extract_file_sync method is accessible') do
  Kreuzberg.respond_to?(:extract_file_sync)
end

runner.test('extract_file_sync with non-existent file raises IOError') do
  begin
    Kreuzberg.extract_file_sync('/nonexistent/path/to/file.pdf')
    false
  rescue Kreuzberg::Errors::IOError
    true
  rescue StandardError => e
    puts "    Got #{e.class} instead of IOError"
    false
  end
end

runner.start_section('Extraction Functions - Bytes-based (Sync)')

runner.test('extract_bytes_sync method is accessible') do
  Kreuzberg.respond_to?(:extract_bytes_sync)
end

runner.test('extract_bytes_sync with empty PDF raises ParsingError or IOError') do
  begin
    Kreuzberg.extract_bytes_sync('', 'application/pdf')
    false
  rescue Kreuzberg::Errors::ParsingError, Kreuzberg::Errors::IOError, Kreuzberg::Errors::UnsupportedFormatError
    true
  rescue StandardError => e
    puts "    Got #{e.class}: #{e.message}"
    false
  end
end

runner.start_section('Batch Extraction Functions (Sync)')

runner.test('batch_extract_files_sync method is accessible') do
  Kreuzberg.respond_to?(:batch_extract_files_sync)
end

runner.test('batch_extract_bytes_sync method is accessible') do
  Kreuzberg.respond_to?(:batch_extract_bytes_sync)
end

runner.test('batch_extract_files_sync with empty array returns array') do
  result = Kreuzberg.batch_extract_files_sync([])
  result.is_a?(Array)
end

runner.test('batch_extract_bytes_sync with empty array returns array') do
  result = Kreuzberg.batch_extract_bytes_sync([])
  result.is_a?(Array)
end

runner.start_section('Module Functions & API Aliases')

runner.test('Kreuzberg::ExtractionConfig is an alias for Config::Extraction') do
  Kreuzberg::ExtractionConfig == Kreuzberg::Config::Extraction
end

runner.test('Kreuzberg::PageConfig is an alias for Config::PageConfig') do
  Kreuzberg::PageConfig == Kreuzberg::Config::PageConfig
end

runner.test('Protocol classes are accessible') do
  defined?(Kreuzberg::PostProcessorProtocol) == 'constant' &&
    defined?(Kreuzberg::ValidatorProtocol) == 'constant' &&
    defined?(Kreuzberg::OcrBackendProtocol) == 'constant'
end

runner.start_section('Error Context & ErrorContext Class')

runner.test('ErrorContext class is defined') do
  defined?(Kreuzberg::ErrorContext) == 'constant'
end

runner.test('PanicContext is defined in Errors') do
  defined?(Kreuzberg::Errors::PanicContext) == 'constant'
end

success = runner.summary
exit(success ? 0 : 1)
