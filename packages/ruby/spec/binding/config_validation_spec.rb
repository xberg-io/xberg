# frozen_string_literal: true

RSpec.describe 'Configuration Validation' do
  describe Kreuzberg::Config::Extraction do
    it 'accepts all valid parameters' do
      config = described_class.new(
        use_cache: true,
        enable_quality_processing: false,
        force_ocr: false,
        ocr: Kreuzberg::Config::OCR.new,
        chunking: Kreuzberg::Config::Chunking.new,
        language_detection: Kreuzberg::Config::LanguageDetection.new,
        pdf_options: Kreuzberg::Config::PDF.new
      )

      expect(config.use_cache).to be true
      expect(config.enable_quality_processing).to be false
      expect(config.force_ocr).to be false
      expect(config.ocr).to be_a(Kreuzberg::Config::OCR)
      expect(config.chunking).to be_a(Kreuzberg::Config::Chunking)
      expect(config.language_detection).to be_a(Kreuzberg::Config::LanguageDetection)
      expect(config.pdf_options).to be_a(Kreuzberg::Config::PDF)
    end

    it 'accepts hashes for nested configs' do
      config = described_class.new(
        ocr: { backend: 'tesseract', language: 'eng' },
        chunking: { max_chars: 500 }
      )

      expect(config.ocr).to be_a(Kreuzberg::Config::OCR)
      expect(config.ocr.backend).to eq('tesseract')
      expect(config.chunking).to be_a(Kreuzberg::Config::Chunking)
      expect(config.chunking.max_chars).to eq(500)
    end

    it 'validates ocr config type' do
      expect do
        described_class.new(ocr: 'invalid')
      end.to raise_error(ArgumentError, /Expected.*OCR/)
    end

    it 'validates chunking config type' do
      expect do
        described_class.new(chunking: 'invalid')
      end.to raise_error(ArgumentError, /Expected.*Chunking/)
    end

    it 'converts to hash correctly' do
      config = described_class.new(
        use_cache: false,
        force_ocr: true
      )
      hash = config.to_h

      expect(hash).to be_a(Hash)
      expect(hash[:use_cache]).to be false
      expect(hash[:force_ocr]).to be true
    end

    it 'omits nil values from hash' do
      config = described_class.new
      hash = config.to_h

      expect(hash[:ocr]).to be_nil
      expect(hash[:chunking]).to be_nil
    end

    it 'accepts html options hashes' do
      config = described_class.new(html_options: { heading_style: :atx, wrap: true })
      expect(config.html_options).to be_a(Kreuzberg::Config::HtmlOptions)
      expect(config.html_options.to_h[:heading_style]).to eq(:atx)
    end

    it 'accepts keyword configurations' do
      keywords = Kreuzberg::Config::Keywords.new(algorithm: :yake, max_keywords: 5)
      config = described_class.new(keywords: keywords, max_concurrent_extractions: 4)
      expect(config.keywords).to be_a(Kreuzberg::Config::Keywords)
      expect(config.max_concurrent_extractions).to eq(4)
    end
  end

  describe Kreuzberg::Config::OCR do
    it 'has sensible defaults' do
      config = described_class.new

      expect(config.backend).to eq('tesseract')
      expect(config.language).to eq('eng')
      expect(config.tesseract_config).to be_nil
    end

    it 'accepts custom values' do
      config = described_class.new(
        backend: 'easyocr',
        language: 'deu'
      )

      expect(config.backend).to eq('easyocr')
      expect(config.language).to eq('deu')
    end

    it 'coerces types correctly' do
      config = described_class.new(
        backend: :tesseract,
        language: 123
      )

      expect(config.backend).to eq('tesseract')
      expect(config.language).to eq('123')
    end

    it 'accepts tesseract config hashes' do
      config = described_class.new(
        tesseract_config: {
          psm: 6,
          enable_table_detection: true
        }
      )

      expect(config.tesseract_config).to be_a(Kreuzberg::Config::Tesseract)
      expect(config.tesseract_config.to_h[:psm]).to eq(6)
    end
  end

  describe Kreuzberg::Config::Chunking do
    it 'has sensible defaults' do
      config = described_class.new

      expect(config.max_chars).to eq(1000)
      expect(config.max_overlap).to eq(200)
      expect(config.preset).to be_nil
    end

    it 'accepts custom chunk sizes' do
      config = described_class.new(
        max_chars: 500,
        max_overlap: 100
      )

      expect(config.max_chars).to eq(500)
      expect(config.max_overlap).to eq(100)
    end

    it 'supports different strategies' do
      config = described_class.new(preset: 'fast')
      expect(config.preset).to eq('fast')
    end

    it 'accepts embedding configs' do
      embedding = { model: { type: :preset, name: 'quality' }, normalize: false }
      config = described_class.new(embedding: embedding)
      expect(config.embedding).to be_a(Kreuzberg::Config::Embedding)
      expect(config.embedding.to_h[:model]).to include(type: :preset, name: 'quality')
    end
  end

  describe Kreuzberg::Config::LanguageDetection do
    it 'has sensible defaults' do
      config = described_class.new

      expect(config.enabled).to be false
      expect(config.min_confidence).to eq(0.5)
    end

    it 'accepts custom confidence thresholds' do
      config = described_class.new(
        enabled: true,
        min_confidence: 0.9
      )

      expect(config.enabled).to be true
      expect(config.min_confidence).to eq(0.9)
    end

    it 'coerces confidence to float' do
      config = described_class.new(min_confidence: '0.75')
      expect(config.min_confidence).to eq(0.75)
    end

    it 'supports detect_multiple flag' do
      config = described_class.new(detect_multiple: true)
      expect(config.detect_multiple).to be true
      expect(config.to_h[:detect_multiple]).to be true
    end
  end

  describe Kreuzberg::Config::PDF do
    it 'has sensible defaults' do
      config = described_class.new

      expect(config.extract_images).to be false
      expect(config.passwords).to be_nil
      expect(config.extract_metadata).to be true
    end

    it 'accepts custom values' do
      config = described_class.new(
        extract_images: true,
        passwords: ['secret123']
      )

      expect(config.extract_images).to be true
      expect(config.passwords).to eq(['secret123'])
    end

    it 'converts password to string' do
      config = described_class.new(passwords: 12_345)
      expect(config.passwords).to eq(['12345'])
    end
  end

  describe Kreuzberg::Config::HtmlOptions do
    it 'normalizes preprocessing settings' do
      options = described_class.new(
        heading_style: :atx_closed,
        preprocessing: { enabled: true, preset: :standard }
      )
      hash = options.to_h
      expect(hash[:heading_style]).to eq(:atx_closed)
      expect(hash[:preprocessing]).to include(preset: :standard)
    end
  end

  describe Kreuzberg::Config::Keywords do
    it 'accepts hash arguments' do
      config = described_class.new(
        algorithm: :yake,
        max_keywords: 10,
        ngram_range: [1, 3],
        yake_params: { window_size: 4 }
      )
      expect(config.to_h[:algorithm]).to eq('yake')
      expect(config.to_h[:yake_params]).to eq(window_size: 4)
    end
  end

  describe Kreuzberg::Config::ImagePreprocessing do
    it 'has sensible defaults' do
      config = described_class.new
      expect(config.target_dpi).to eq(300)
      expect(config.auto_rotate).to be true
      expect(config.deskew).to be true
      expect(config.denoise).to be false
      expect(config.contrast_enhance).to be true
      expect(config.binarization_method).to eq('otsu')
      expect(config.invert_colors).to be false
    end

    it 'accepts custom values' do
      config = described_class.new(
        target_dpi: 600,
        auto_rotate: false,
        deskew: false,
        denoise: true,
        contrast_enhance: false,
        binarization_method: 'sauvola',
        invert_colors: true
      )
      expect(config.target_dpi).to eq(600)
      expect(config.auto_rotate).to be false
      expect(config.deskew).to be false
      expect(config.denoise).to be true
      expect(config.contrast_enhance).to be false
      expect(config.binarization_method).to eq('sauvola')
      expect(config.invert_colors).to be true
    end

    it 'validates binarization method via FFI' do
      expect { described_class.new(binarization_method: 'otsu') }.not_to raise_error
      expect { described_class.new(binarization_method: 'adaptive') }.not_to raise_error
      expect { described_class.new(binarization_method: 'sauvola') }.not_to raise_error
    end

    it 'rejects invalid binarization methods' do
      expect do
        described_class.new(binarization_method: 'invalid_method')
      end.to raise_error(ArgumentError, /Invalid binarization_method/)
    end

    it 'converts to hash correctly' do
      config = described_class.new(
        target_dpi: 500,
        binarization_method: 'adaptive'
      )
      hash = config.to_h
      expect(hash[:target_dpi]).to eq(500)
      expect(hash[:binarization_method]).to eq('adaptive')
      expect(hash[:auto_rotate]).to be true
    end
  end

  describe Kreuzberg::Config::TokenReduction do
    it 'has sensible defaults' do
      config = described_class.new
      expect(config.mode).to eq('off')
      expect(config.preserve_important_words).to be true
    end

    it 'accepts custom values' do
      config = described_class.new(
        mode: 'aggressive',
        preserve_important_words: false
      )
      expect(config.mode).to eq('aggressive')
      expect(config.preserve_important_words).to be false
    end

    it 'validates token reduction levels via FFI' do
      expect { described_class.new(mode: 'off') }.not_to raise_error
      expect { described_class.new(mode: 'light') }.not_to raise_error
      expect { described_class.new(mode: 'moderate') }.not_to raise_error
      expect { described_class.new(mode: 'aggressive') }.not_to raise_error
      expect { described_class.new(mode: 'maximum') }.not_to raise_error
    end

    it 'rejects invalid token reduction modes' do
      expect do
        described_class.new(mode: 'extreme')
      end.to raise_error(ArgumentError, /Invalid token reduction mode/)
    end

    it 'converts to hash correctly' do
      config = described_class.new(
        mode: 'light',
        preserve_important_words: true
      )
      hash = config.to_h
      expect(hash[:mode]).to eq('light')
      expect(hash[:preserve_important_words]).to be true
    end
  end

  describe 'config usage in extraction' do
    it 'works with OCR config' do
      path = create_test_file('OCR config test')
      config = Kreuzberg::Config::Extraction.new(
        ocr: Kreuzberg::Config::OCR.new(backend: 'tesseract', language: 'eng')
      )

      result = Kreuzberg.extract_file_sync(path, config: config)
      expect(result).to be_a(Kreuzberg::Result)
    end

    it 'works with chunking config' do
      path = create_test_file('Chunking config test' * 50)
      config = Kreuzberg::Config::Extraction.new(
        chunking: Kreuzberg::Config::Chunking.new(max_chars: 50)
      )

      result = Kreuzberg.extract_file_sync(path, config: config)
      expect(result).to be_a(Kreuzberg::Result)
    end

    it 'works with language detection config' do
      path = create_test_file('Language detection test')
      config = Kreuzberg::Config::Extraction.new(
        language_detection: Kreuzberg::Config::LanguageDetection.new(enabled: true)
      )

      result = Kreuzberg.extract_file_sync(path, config: config)
      expect(result).to be_a(Kreuzberg::Result)
    end

    it 'works with combined configs' do
      path = create_test_file('Combined config test')
      config = Kreuzberg::Config::Extraction.new(
        use_cache: false,
        force_ocr: false,
        ocr: { backend: 'tesseract', language: 'eng' },
        language_detection: { enabled: false }
      )

      result = Kreuzberg.extract_file_sync(path, config: config)
      expect(result).to be_a(Kreuzberg::Result)
    end
  end
end
