# frozen_string_literal: true

require_relative 'spec_helper'

RSpec.describe 'Kreuzberg Extraction' do
  describe 'Type verification' do
    it 'Kreuzberg module exists' do
      expect(defined?(Kreuzberg)).to be_truthy
    end

    it 'Kreuzberg::Result is accessible' do
      expect(defined?(Kreuzberg::Result)).to be_truthy
    end

    it 'Kreuzberg::Config is accessible' do
      expect(defined?(Kreuzberg::Config)).to be_truthy
    end

    it 'Kreuzberg::Config::Extraction is accessible' do
      expect(defined?(Kreuzberg::Config::Extraction)).to be_truthy
    end

    it 'Kreuzberg::Config::OCR is accessible' do
      expect(defined?(Kreuzberg::Config::OCR)).to be_truthy
    end

    it 'Kreuzberg::Errors module exists' do
      expect(defined?(Kreuzberg::Errors)).to be_truthy
    end

    it 'Kreuzberg::Errors::IOError is accessible' do
      expect(defined?(Kreuzberg::Errors::IOError)).to be_truthy
    end

    it 'Kreuzberg::CLI is accessible' do
      expect(defined?(Kreuzberg::CLI)).to be_truthy
    end
  end

  describe 'Synchronous file extraction' do
    it 'extracts content from DOCX file' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to be_a(String)
      expect(result.content).not_to be_empty
      expect(result.mime_type).to include('wordprocessing')
    end

    it 'extracts content from ODT file' do
      path = test_document_path('documents/simple.odt')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
    end

    it 'returns result with proper structure' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to respond_to(:content)
      expect(result).to respond_to(:mime_type)
      expect(result).to respond_to(:metadata)
      expect(result).to respond_to(:tables)
      expect(result).to respond_to(:chunks)
    end

    it 'handles file with explicit MIME type' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path, mime_type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document')

      expect(result.content).not_to be_empty
    end
  end

  describe 'Asynchronous file extraction' do
    it 'extracts content from file asynchronously' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file(path)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
    end

    it 'returns async result with content and metadata' do
      path = test_document_path('documents/simple.odt')
      result = Kreuzberg.extract_file(path)

      expect(result.content).to be_a(String)
      expect(result.mime_type).to be_a(String)
    end
  end

  describe 'Synchronous byte extraction' do
    it 'extracts content from binary DOCX data' do
      path = test_document_path('documents/fake.docx')
      data = read_test_document('documents/fake.docx')
      result = Kreuzberg.extract_bytes_sync(
        data,
        'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
      )

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
    end

    it 'extracts content from binary ODT data' do
      data = read_test_document('documents/simple.odt')
      result = Kreuzberg.extract_bytes_sync(data, 'application/vnd.oasis.opendocument.text')

      expect(result.content).not_to be_empty
    end

    it 'requires MIME type for byte extraction' do
      data = read_test_document('documents/fake.docx')
      result = Kreuzberg.extract_bytes_sync(data, 'application/vnd.openxmlformats-officedocument.wordprocessingml.document')
      expect(result).to be_a(Kreuzberg::Result)
    end
  end

  describe 'Asynchronous byte extraction' do
    it 'extracts content from binary data asynchronously' do
      data = read_test_document('documents/fake.docx')
      result = Kreuzberg.extract_bytes(
        data,
        'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
      )

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
    end

    it 'handles async byte extraction from ODT' do
      data = read_test_document('documents/simple.odt')
      result = Kreuzberg.extract_bytes(data, 'application/vnd.oasis.opendocument.text')

      expect(result.content).not_to be_empty
    end
  end

  describe 'Batch synchronous file extraction' do
    it 'extracts multiple files in batch' do
      paths = [
        test_document_path('documents/fake.docx'),
        test_document_path('documents/simple.odt')
      ]
      results = Kreuzberg.batch_extract_files_sync(paths)

      expect(results).to be_an(Array)
      expect(results.length).to eq(2)
      expect(results.all? { |r| r.is_a?(Kreuzberg::Result) }).to be true
    end

    it 'maintains result order for batch extraction' do
      paths = [
        test_document_path('documents/fake.docx'),
        test_document_path('documents/simple.odt')
      ]
      results = Kreuzberg.batch_extract_files_sync(paths)

      expect(results[0].mime_type).to include('wordprocessing')
      expect(results[1].mime_type).to include('oasis')
    end

    it 'batch extracts with single file' do
      paths = [test_document_path('documents/fake.docx')]
      results = Kreuzberg.batch_extract_files_sync(paths)

      expect(results.length).to eq(1)
      expect(results[0].content).not_to be_empty
    end

    it 'all batch results have content' do
      paths = [
        test_document_path('documents/fake.docx'),
        test_document_path('documents/simple.odt')
      ]
      results = Kreuzberg.batch_extract_files_sync(paths)

      expect(results.all? { |r| r.content.is_a?(String) && !r.content.empty? }).to be true
    end
  end

  describe 'Batch asynchronous file extraction' do
    it 'extracts multiple files asynchronously' do
      paths = [
        test_document_path('documents/fake.docx'),
        test_document_path('documents/simple.odt')
      ]
      results = Kreuzberg.batch_extract_files(paths)

      expect(results).to be_an(Array)
      expect(results.length).to eq(2)
    end

    it 'async batch extracts with configuration' do
      paths = [test_document_path('documents/fake.docx')]
      config = Kreuzberg::Config::Extraction.new
      results = Kreuzberg.batch_extract_files(paths, config: config)

      expect(results[0]).to be_a(Kreuzberg::Result)
      expect(results[0].content).not_to be_empty
    end
  end

  describe 'Batch byte extraction' do
    it 'batch extracts multiple binary documents synchronously' do
      data_array = [
        read_test_document('documents/fake.docx'),
        read_test_document('documents/simple.odt')
      ]
      mime_types = [
        'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
        'application/vnd.oasis.opendocument.text'
      ]
      results = Kreuzberg.batch_extract_bytes_sync(data_array, mime_types)

      expect(results).to be_an(Array)
      expect(results.length).to eq(2)
      expect(results.all? { |r| r.is_a?(Kreuzberg::Result) }).to be true
    end

    it 'batch extracts multiple binary documents asynchronously' do
      data_array = [
        read_test_document('documents/fake.docx'),
        read_test_document('documents/simple.odt')
      ]
      mime_types = [
        'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
        'application/vnd.oasis.opendocument.text'
      ]
      results = Kreuzberg.batch_extract_bytes(data_array, mime_types)

      expect(results.length).to eq(2)
      expect(results.all? { |r| r.content.is_a?(String) }).to be true
    end

    it 'maintains order in batch byte extraction' do
      data_array = [
        read_test_document('documents/fake.docx'),
        read_test_document('documents/simple.odt')
      ]
      mime_types = [
        'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
        'application/vnd.oasis.opendocument.text'
      ]
      results = Kreuzberg.batch_extract_bytes_sync(data_array, mime_types)

      expect(results[0].mime_type).to include('wordprocessing')
      expect(results[1].mime_type).to include('oasis')
    end
  end

  describe 'MIME type detection' do
    it 'detects MIME type from file path' do
      path = test_document_path('documents/fake.docx')
      mime_type = Kreuzberg::CLI.detect(path)

      expect(mime_type).to be_a(String)
      expect(mime_type).not_to be_empty
      expect(mime_type).to include('wordprocessing')
    end

    it 'detects MIME type for ODT files' do
      path = test_document_path('documents/simple.odt')
      mime_type = Kreuzberg::CLI.detect(path)

      expect(mime_type).to include('oasis')
    end

    it 'extracts and provides MIME type in result' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result.mime_type).to be_a(String)
      expect(result.mime_type).not_to be_empty
    end
  end

  describe 'File type coverage' do
    it 'extracts from DOCX files' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
      expect(result.mime_type).to include('wordprocessingml')
    end

    it 'extracts from ODT files' do
      path = test_document_path('documents/simple.odt')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
    end

    it 'extracts from Markdown files' do
      path = test_document_path('extraction_test.md')
      skip 'Markdown test file required' unless File.exist?(path)
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to be_a(Kreuzberg::Result)
    end

    it 'extracts from image files - PNG' do
      path = test_document_path('images/sample.png')
      skip 'PNG test file required' unless File.exist?(path)
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to be_a(Kreuzberg::Result)
    end

    it 'extracts from image files - JPG' do
      path = test_document_path('images/example.jpg')
      skip 'JPG test file required' unless File.exist?(path)
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to be_a(Kreuzberg::Result)
    end
  end

  describe 'Configuration handling' do
    it 'creates extraction config object' do
      config = Kreuzberg::Config::Extraction.new
      expect(config).to be_a(Kreuzberg::Config::Extraction)
    end

    it 'creates OCR config object' do
      ocr_config = Kreuzberg::Config::OCR.new
      expect(ocr_config).to be_a(Kreuzberg::Config::OCR)
    end

    it 'extracts with custom config' do
      path = test_document_path('documents/fake.docx')
      config = Kreuzberg::Config::Extraction.new
      result = Kreuzberg.extract_file_sync(path, config: config)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
    end

    it 'extracts with hash config' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path, config: {})

      expect(result).to be_a(Kreuzberg::Result)
    end
  end

  describe 'Result structure and attributes' do
    it 'result has content attribute' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to respond_to(:content)
      expect(result.content).to be_a(String)
    end

    it 'result has MIME type attribute' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to respond_to(:mime_type)
      expect(result.mime_type).to be_a(String)
    end

    it 'result has metadata attribute' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to respond_to(:metadata)
    end

    it 'result has tables attribute' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to respond_to(:tables)
      expect(result.tables).to be_an(Array)
    end

    it 'result has chunks attribute' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to respond_to(:chunks)
      expect(result.chunks).to be_an(Array)
    end

    it 'result has detected_languages attribute' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to respond_to(:detected_languages)
    end

    it 'result has pages attribute' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to respond_to(:pages)
    end

    it 'result has images attribute' do
      path = test_document_path('documents/fake.docx')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to respond_to(:images)
    end
  end

  describe 'Integration tests' do
    it 'extracts and provides consistent results on repeated calls' do
      path = test_document_path('documents/fake.docx')
      result1 = Kreuzberg.extract_file_sync(path)
      result2 = Kreuzberg.extract_file_sync(path)

      expect(result1.content).to eq(result2.content)
      expect(result1.mime_type).to eq(result2.mime_type)
    end

    it 'sync and async extraction produce same content' do
      path = test_document_path('documents/fake.docx')
      sync_result = Kreuzberg.extract_file_sync(path)
      async_result = Kreuzberg.extract_file(path)

      expect(sync_result.content).to eq(async_result.content)
    end

    it 'file and bytes extraction produce same content' do
      path = test_document_path('documents/fake.docx')
      file_result = Kreuzberg.extract_file_sync(path)
      data = read_test_document('documents/fake.docx')
      bytes_result = Kreuzberg.extract_bytes_sync(
        data,
        'application/vnd.openxmlformats-officedocument.wordprocessingml.document'
      )

      expect(file_result.content).to eq(bytes_result.content)
    end

    it 'batch and individual extraction produce same results' do
      path = test_document_path('documents/fake.docx')
      individual_result = Kreuzberg.extract_file_sync(path)
      batch_results = Kreuzberg.batch_extract_files_sync([path])

      expect(individual_result.content).to eq(batch_results[0].content)
    end
  end

  describe 'CLI interface' do
    it 'CLI extract returns string output' do
      path = test_document_path('documents/fake.docx')
      output = Kreuzberg::CLI.extract(path)

      expect(output).to be_a(String)
      expect(output).not_to be_empty
    end

    it 'CLI detect returns MIME type' do
      path = test_document_path('documents/fake.docx')
      mime_type = Kreuzberg::CLI.detect(path)

      expect(mime_type).to be_a(String)
      expect(mime_type).not_to be_empty
    end

    it 'CLI version returns version string' do
      version = Kreuzberg::CLI.version
      expect(version).to be_a(String)
      expect(version).to match(/\d+\.\d+/)
    end
  end
end
