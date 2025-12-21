# frozen_string_literal: true

require 'spec_helper'

RSpec.describe 'Phase 1 FFI Config and Result Methods' do
  let(:fixture_dir) { File.expand_path('../fixtures', __dir__) }

  describe 'Kreuzberg::Config::Extraction' do
    describe '#to_json' do
      it 'serializes a basic config to JSON' do
        config = Kreuzberg::Config::Extraction.new(use_cache: true)
        json = config.to_json
        expect(json).to be_a(String)
        parsed = JSON.parse(json)
        expect(parsed['use_cache']).to be true
      end

      it 'serializes complex nested config to JSON' do
        ocr = Kreuzberg::Config::OCR.new(backend: 'tesseract', language: 'deu')
        chunking = Kreuzberg::Config::Chunking.new(max_chars: 500, max_overlap: 50)
        config = Kreuzberg::Config::Extraction.new(
          use_cache: true,
          force_ocr: false,
          ocr: ocr,
          chunking: chunking
        )

        json = config.to_json
        parsed = JSON.parse(json)

        expect(parsed['use_cache']).to be true
        expect(parsed['force_ocr']).to be false
        expect(parsed['ocr']['backend']).to eq('tesseract')
        expect(parsed['ocr']['language']).to eq('deu')
        expect(parsed['chunking']['max_chars']).to eq(500)
        expect(parsed['chunking']['max_overlap']).to eq(50)
      end

      it 'handles minimal config' do
        config = Kreuzberg::Config::Extraction.new
        json = config.to_json
        expect(json).to be_a(String)
        parsed = JSON.parse(json)
        expect(parsed).to be_a(Hash)
      end
    end

    describe '#get_field' do
      let(:config) do
        ocr = Kreuzberg::Config::OCR.new(backend: 'tesseract', language: 'eng')
        Kreuzberg::Config::Extraction.new(
          use_cache: true,
          force_ocr: false,
          ocr: ocr
        )
      end

      it 'gets a top-level field' do
        value = config.get_field('use_cache')
        expect(value).to be true
      end

      it 'gets a nested field with dot notation' do
        value = config.get_field('ocr.backend')
        expect(value).to eq('tesseract')
      end

      it 'gets another nested field' do
        value = config.get_field('ocr.language')
        expect(value).to eq('eng')
      end

      it 'returns nil for non-existent field' do
        value = config.get_field('nonexistent')
        expect(value).to be_nil
      end

      it 'returns nil for non-existent nested field' do
        value = config.get_field('ocr.nonexistent')
        expect(value).to be_nil
      end

      it 'supports symbol field names' do
        value = config.get_field(:use_cache)
        expect(value).to be true
      end

      it 'gets boolean fields correctly' do
        value = config.get_field('force_ocr')
        expect(value).to be false
      end
    end

    describe '#merge' do
      let(:base_config) do
        Kreuzberg::Config::Extraction.new(
          use_cache: true,
          force_ocr: false,
          enable_quality_processing: false
        )
      end

      let(:override_config) do
        Kreuzberg::Config::Extraction.new(
          force_ocr: true,
          enable_quality_processing: true
        )
      end

      it 'merges two configs without modifying original' do
        merged = base_config.merge(override_config)

        expect(base_config.use_cache).to be true
        expect(base_config.force_ocr).to be false
        expect(base_config.enable_quality_processing).to be false

        expect(merged.use_cache).to be true
        expect(merged.force_ocr).to be true
        expect(merged.enable_quality_processing).to be true
      end

      it 'returns a new Extraction instance' do
        merged = base_config.merge(override_config)
        expect(merged).to be_a(Kreuzberg::Config::Extraction)
        expect(merged).not_to be(base_config)
      end

      it 'merges with a Hash' do
        merged = base_config.merge(force_ocr: true)

        expect(merged.use_cache).to be true
        expect(merged.force_ocr).to be true
      end

      it 'handles nested config merging' do
        ocr1 = Kreuzberg::Config::OCR.new(backend: 'tesseract', language: 'eng')
        base = Kreuzberg::Config::Extraction.new(ocr: ocr1, use_cache: true)

        ocr2 = Kreuzberg::Config::OCR.new(backend: 'easyocr')
        override = Kreuzberg::Config::Extraction.new(ocr: ocr2)

        merged = base.merge(override)

        expect(merged.use_cache).to be true
        expect(merged.ocr.backend).to eq('easyocr')
      end
    end

    describe '#merge!' do
      let(:base_config) do
        Kreuzberg::Config::Extraction.new(
          use_cache: true,
          force_ocr: false
        )
      end

      let(:override_config) do
        Kreuzberg::Config::Extraction.new(
          force_ocr: true
        )
      end

      it 'modifies the original config in place' do
        original_object_id = base_config.object_id
        result = base_config.merge!(override_config)

        expect(result.object_id).to eq(original_object_id)
        expect(base_config.use_cache).to be true
        expect(base_config.force_ocr).to be true
      end

      it 'returns self' do
        result = base_config.merge!(override_config)
        expect(result).to be(base_config)
      end

      it 'works with Hash argument' do
        base_config.merge!(force_ocr: true, use_cache: false)

        expect(base_config.force_ocr).to be true
        expect(base_config.use_cache).to be false
      end

      it 'updates all fields correctly' do
        ocr = Kreuzberg::Config::OCR.new(backend: 'easyocr')
        base_config.merge!(ocr: ocr, enable_quality_processing: true)

        expect(base_config.ocr.backend).to eq('easyocr')
        expect(base_config.enable_quality_processing).to be true
      end
    end
  end

  describe 'Kreuzberg::Result' do
    let(:sample_result_hash) do
      {
        'content' => 'Sample document content',
        'mime_type' => 'application/pdf',
        'metadata_json' => {
          'title' => 'Test Document',
          'language' => 'en',
          'pages' => {
            'total_count' => 10,
            'unit_type' => 'Page'
          },
          'format' => {
            'name' => 'PDF',
            'pages' => 10
          }
        }.to_json,
        'tables' => [],
        'detected_languages' => %w[en de],
        'chunks' => [
          {
            'content' => 'Chunk 1',
            'byte_start' => 0,
            'byte_end' => 7,
            'token_count' => 2,
            'chunk_index' => 0,
            'total_chunks' => 2,
            'first_page' => 1,
            'last_page' => 1,
            'embedding' => nil
          },
          {
            'content' => 'Chunk 2',
            'byte_start' => 8,
            'byte_end' => 15,
            'token_count' => 2,
            'chunk_index' => 1,
            'total_chunks' => 2,
            'first_page' => 2,
            'last_page' => 2,
            'embedding' => nil
          }
        ]
      }
    end

    let(:result) { Kreuzberg::Result.new(sample_result_hash) }

    describe '#page_count' do
      it 'returns the total page count' do
        expect(result.page_count).to eq(10)
      end

      it 'returns 0 for result without page info' do
        minimal_result = Kreuzberg::Result.new(
          'content' => 'Test',
          'mime_type' => 'text/plain',
          'metadata_json' => '{}'
        )
        expect(minimal_result.page_count).to eq(0)
      end

      it 'returns 0 when metadata has no pages info' do
        result_no_pages = Kreuzberg::Result.new(
          'content' => 'Test',
          'mime_type' => 'text/plain',
          'metadata_json' => '{"title": "Test"}'
        )
        expect(result_no_pages.page_count).to eq(0)
      end
    end

    describe '#chunk_count' do
      it 'returns the total number of chunks' do
        expect(result.chunk_count).to eq(2)
      end

      it 'returns 0 for result without chunks' do
        no_chunks_result = Kreuzberg::Result.new(
          'content' => 'Test',
          'mime_type' => 'text/plain',
          'metadata_json' => '{}'
        )
        expect(no_chunks_result.chunk_count).to eq(0)
      end

      it 'returns 0 for empty chunks array' do
        empty_chunks_result = Kreuzberg::Result.new(
          'content' => 'Test',
          'mime_type' => 'text/plain',
          'metadata_json' => '{}',
          'chunks' => []
        )
        expect(empty_chunks_result.chunk_count).to eq(0)
      end
    end

    describe '#detected_language' do
      it 'returns the primary detected language from metadata' do
        expect(result.detected_language).to eq('en')
      end

      it 'returns the first detected language if metadata language is not set' do
        result_with_detected = Kreuzberg::Result.new(
          'content' => 'Test',
          'mime_type' => 'text/plain',
          'metadata_json' => '{}',
          'detected_languages' => %w[fr de]
        )
        expect(result_with_detected.detected_language).to eq('fr')
      end

      it 'returns nil when no language is detected' do
        no_lang_result = Kreuzberg::Result.new(
          'content' => 'Test',
          'mime_type' => 'text/plain',
          'metadata_json' => '{}'
        )
        expect(no_lang_result.detected_language).to be_nil
      end

      it 'returns nil for empty detected languages array' do
        empty_langs_result = Kreuzberg::Result.new(
          'content' => 'Test',
          'mime_type' => 'text/plain',
          'metadata_json' => '{}',
          'detected_languages' => []
        )
        expect(empty_langs_result.detected_language).to be_nil
      end
    end

    describe '#metadata_field' do
      it 'gets a top-level metadata field' do
        value = result.metadata_field('title')
        expect(value).to eq('Test Document')
      end

      it 'gets a nested metadata field with dot notation' do
        value = result.metadata_field('pages.total_count')
        expect(value).to eq(10)
      end

      it 'gets another nested field' do
        value = result.metadata_field('format.name')
        expect(value).to eq('PDF')
      end

      it 'returns nil for non-existent field' do
        value = result.metadata_field('nonexistent')
        expect(value).to be_nil
      end

      it 'returns nil for non-existent nested field' do
        value = result.metadata_field('format.nonexistent')
        expect(value).to be_nil
      end

      it 'supports symbol field names' do
        value = result.metadata_field(:title)
        expect(value).to eq('Test Document')
      end

      it 'returns nil when trying to access nested field on non-hash value' do
        value = result.metadata_field('title.nested')
        expect(value).to be_nil
      end

      it 'handles deeply nested fields' do
        value = result.metadata_field('format.pages')
        expect(value).to eq(10)
      end

      it 'returns nil for result without metadata' do
        no_metadata = Kreuzberg::Result.new(
          'content' => 'Test',
          'mime_type' => 'text/plain',
          'metadata_json' => 'invalid json'
        )
        expect(no_metadata.metadata_field('title')).to be_nil
      end
    end
  end
end
