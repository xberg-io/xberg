# frozen_string_literal: true

require 'json'

module Kreuzberg
  module Config
    # @example
    class OCR
      attr_reader :backend, :language, :tesseract_config, :paddle_ocr_config, :element_config

      def initialize(
        backend: 'tesseract',
        language: 'eng',
        tesseract_config: nil,
        paddle_ocr_config: nil,
        element_config: nil
      )
        @backend = backend.to_s
        @language = language.to_s
        @tesseract_config = normalize_tesseract_config(tesseract_config)
        @paddle_ocr_config = normalize_paddle_ocr_config(paddle_ocr_config)
        @element_config = normalize_element_config(element_config)
      end

      def to_h
        {
          backend: @backend,
          language: @language,
          tesseract_config: @tesseract_config&.to_h,
          paddle_ocr_config: @paddle_ocr_config&.to_h,
          element_config: @element_config&.to_h
        }.compact
      end

      private

      def normalize_tesseract_config(value)
        return nil if value.nil?
        return value if value.is_a?(Tesseract)
        return Tesseract.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{Tesseract}, Hash, or nil, got #{value.class}"
      end

      def normalize_paddle_ocr_config(value)
        return nil if value.nil?
        return value if value.is_a?(PaddleOcr)
        return PaddleOcr.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{PaddleOcr}, Hash, or nil, got #{value.class}"
      end

      def normalize_element_config(value)
        return nil if value.nil?
        return value if value.is_a?(OcrElementConfig)
        return OcrElementConfig.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{OcrElementConfig}, Hash, or nil, got #{value.class}"
      end
    end

    # Tesseract OCR engine configuration
    class Tesseract
      attr_reader :options

      def initialize(**options)
        @options = options.transform_keys(&:to_sym)
        normalize_nested_preprocessing!
      end

      def to_h
        @options.dup
      end

      private

      def normalize_nested_preprocessing!
        preprocessing = @options[:preprocessing]
        return if preprocessing.nil?
        return if preprocessing.is_a?(ImagePreprocessing)
        return @options[:preprocessing] = ImagePreprocessing.new(**preprocessing.transform_keys(&:to_sym)) if
          preprocessing.is_a?(Hash)

        raise ArgumentError, "preprocessing must be #{ImagePreprocessing} or Hash"
      end
    end

    # PaddleOCR engine configuration
    #
    # @example Basic usage
    #   paddle = PaddleOcr.new(language: 'en', cache_dir: '/tmp/paddle')
    #
    # @example Advanced configuration
    #   paddle = PaddleOcr.new(
    #     language: 'en',
    #     cache_dir: '/tmp/paddle',
    #     use_angle_cls: true,
    #     det_db_thresh: 0.3,
    #     rec_batch_num: 32
    #   )
    #
    class PaddleOcr
      attr_reader :language, :cache_dir, :use_angle_cls, :enable_table_detection,
                  :det_db_thresh, :det_db_box_thresh, :det_db_unclip_ratio,
                  :det_limit_side_len, :rec_batch_num

      def initialize(
        language: nil,
        cache_dir: nil,
        use_angle_cls: nil,
        enable_table_detection: nil,
        det_db_thresh: nil,
        det_db_box_thresh: nil,
        det_db_unclip_ratio: nil,
        det_limit_side_len: nil,
        rec_batch_num: nil
      )
        @language = language&.to_s
        @cache_dir = cache_dir&.to_s
        @use_angle_cls = boolean_or_nil(use_angle_cls)
        @enable_table_detection = boolean_or_nil(enable_table_detection)
        @det_db_thresh = det_db_thresh&.to_f
        @det_db_box_thresh = det_db_box_thresh&.to_f
        @det_db_unclip_ratio = det_db_unclip_ratio&.to_f
        @det_limit_side_len = det_limit_side_len&.to_i
        @rec_batch_num = rec_batch_num&.to_i
      end

      def to_h
        {
          language: @language,
          cache_dir: @cache_dir,
          use_angle_cls: @use_angle_cls,
          enable_table_detection: @enable_table_detection,
          det_db_thresh: @det_db_thresh,
          det_db_box_thresh: @det_db_box_thresh,
          det_db_unclip_ratio: @det_db_unclip_ratio,
          det_limit_side_len: @det_limit_side_len,
          rec_batch_num: @rec_batch_num
        }.compact
      end

      private

      def boolean_or_nil(value)
        return nil if value.nil?

        value ? true : false
      end
    end

    # OCR element configuration for output control
    #
    # @example Basic usage
    #   config = OcrElementConfig.new(include_elements: true)
    #
    # @example Advanced configuration
    #   config = OcrElementConfig.new(
    #     include_elements: true,
    #     min_level: 'word',
    #     min_confidence: 0.7,
    #     build_hierarchy: true
    #   )
    #
    class OcrElementConfig
      attr_reader :include_elements, :min_level, :min_confidence, :build_hierarchy

      def initialize(
        include_elements: false,
        min_level: nil,
        min_confidence: nil,
        build_hierarchy: false
      )
        @include_elements = include_elements ? true : false
        @min_level = min_level&.to_s
        @min_confidence = min_confidence&.to_f
        @build_hierarchy = build_hierarchy ? true : false
      end

      def to_h
        {
          include_elements: @include_elements,
          min_level: @min_level,
          min_confidence: @min_confidence,
          build_hierarchy: @build_hierarchy
        }.compact
      end
    end

    # Chunking configuration
    #
    # @example
    #   chunking = Chunking.new(max_chars: 1000, max_overlap: 200)
    #
    class Chunking
      attr_reader :max_chars, :max_overlap, :preset, :embedding, :enabled

      def initialize(
        max_chars: nil,
        max_overlap: nil,
        preset: nil,
        embedding: nil,
        chunk_size: nil,
        chunk_overlap: nil,
        enabled: true
      )
        resolved_size = chunk_size || max_chars || 1000
        resolved_overlap = chunk_overlap || max_overlap || 200

        @max_chars = resolved_size.to_i
        @max_overlap = resolved_overlap.to_i

        # Validate positive values
        raise ArgumentError, "max_chars must be a positive integer, got #{@max_chars}" if @max_chars.negative?
        raise ArgumentError, "max_overlap must be a positive integer, got #{@max_overlap}" if @max_overlap.negative?

        @preset = preset&.to_s
        @embedding = normalize_embedding(embedding)
        @enabled = boolean_or_nil(enabled)
      end

      def to_h
        config = {
          max_chars: @max_chars,
          max_overlap: @max_overlap,
          preset: @preset,
          embedding: @embedding&.to_h
        }.compact
        # @type var config: Hash[Symbol, untyped]
        config[:enabled] = @enabled unless @enabled.nil?
        config
      end

      private

      def normalize_embedding(value)
        return nil if value.nil?
        return value if value.is_a?(Embedding)
        return Embedding.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{Embedding}, Hash, or nil, got #{value.class}"
      end

      def boolean_or_nil(value)
        return nil if value.nil?

        value ? true : false
      end
    end

    # Embedding model configuration for document chunking
    class Embedding
      attr_reader :model, :normalize, :batch_size, :show_download_progress, :cache_dir

      def initialize(
        model: { type: :preset, name: 'balanced' },
        normalize: true,
        batch_size: 32,
        show_download_progress: false,
        cache_dir: nil
      )
        @model = normalize_model(model)
        @normalize = boolean_or_nil(normalize)
        @batch_size = batch_size&.to_i
        @show_download_progress = boolean_or_nil(show_download_progress)
        @cache_dir = cache_dir&.to_s
      end

      def to_h
        {
          model: @model,
          normalize: @normalize,
          batch_size: @batch_size,
          show_download_progress: @show_download_progress,
          cache_dir: @cache_dir
        }.compact
      end

      private

      def normalize_model(model)
        normalized = if model.respond_to?(:to_h)
                       model.to_h
                     else
                       model
                     end
        raise ArgumentError, 'model must be a Hash describing the embedding model' unless normalized.is_a?(Hash)

        normalized.transform_keys(&:to_sym)
      end

      def boolean_or_nil(value)
        return nil if value.nil?

        value ? true : false
      end
    end

    # Language detection configuration
    #
    # @example
    #   lang = LanguageDetection.new(enabled: true, min_confidence: 0.8)
    #
    class LanguageDetection
      attr_reader :enabled, :min_confidence, :detect_multiple

      def initialize(enabled: false, min_confidence: 0.5, detect_multiple: false)
        @enabled = enabled ? true : false
        @min_confidence = min_confidence.to_f
        @detect_multiple = detect_multiple ? true : false
      end

      def to_h
        {
          enabled: @enabled,
          min_confidence: @min_confidence,
          detect_multiple: @detect_multiple
        }
      end
    end

    # Font configuration for PDF rendering
    #
    # @example
    #   font_config = FontConfig.new(enabled: true, custom_font_dirs: ["/usr/share/fonts"])
    #
    class FontConfig
      attr_accessor :enabled, :custom_font_dirs

      def initialize(enabled: true, custom_font_dirs: nil)
        @enabled = enabled ? true : false
        @custom_font_dirs = custom_font_dirs
      end

      def to_h
        {
          enabled: @enabled,
          custom_font_dirs: @custom_font_dirs
        }.compact
      end
    end

    # Hierarchy detection configuration
    #
    # @example
    #   hierarchy = Hierarchy.new(enabled: true, k_clusters: 6, include_bbox: true)
    #
    class Hierarchy
      attr_reader :enabled, :k_clusters, :include_bbox, :ocr_coverage_threshold

      def initialize(
        enabled: true,
        k_clusters: 6,
        include_bbox: true,
        ocr_coverage_threshold: nil
      )
        @enabled = enabled ? true : false
        @k_clusters = k_clusters&.to_i || 6
        @include_bbox = include_bbox ? true : false
        @ocr_coverage_threshold = ocr_coverage_threshold&.to_f
      end

      def to_h
        {
          enabled: @enabled,
          k_clusters: @k_clusters,
          include_bbox: @include_bbox,
          ocr_coverage_threshold: @ocr_coverage_threshold
        }.compact
      end

      def self.from_h(hash)
        return nil if hash.nil?
        return hash if hash.is_a?(self)

        new(**hash.transform_keys(&:to_sym)) if hash.is_a?(Hash)
      end
    end

    # PDF-specific options
    #
    # @example
    #   pdf = PDF.new(extract_images: true, passwords: ["secret", "backup"])
    #
    # @example With font configuration
    #   font_config = FontConfig.new(enabled: true, custom_font_dirs: ["/usr/share/fonts"])
    #   pdf = PDF.new(extract_images: true, font_config: font_config)
    #
    # @example With hierarchy configuration
    #   hierarchy = Hierarchy.new(enabled: true, k_clusters: 6)
    #   pdf = PDF.new(extract_images: true, hierarchy: hierarchy)
    #
    class PDF
      attr_reader :extract_images, :passwords, :extract_metadata, :font_config, :hierarchy,
                  :extract_annotations, :top_margin_fraction, :bottom_margin_fraction,
                  :allow_single_column_tables

      def initialize(
        extract_images: false,
        passwords: nil,
        extract_metadata: true,
        font_config: nil,
        hierarchy: nil,
        extract_annotations: false,
        top_margin_fraction: nil,
        bottom_margin_fraction: nil,
        allow_single_column_tables: false
      )
        @extract_images = extract_images ? true : false
        @passwords = if passwords.is_a?(Array)
                       passwords.map(&:to_s)
                     else
                       (passwords ? [passwords.to_s] : nil)
                     end
        @extract_metadata = extract_metadata ? true : false
        @font_config = normalize_font_config(font_config)
        @hierarchy = normalize_hierarchy(hierarchy)
        @extract_annotations = extract_annotations ? true : false
        @top_margin_fraction = top_margin_fraction&.to_f
        @bottom_margin_fraction = bottom_margin_fraction&.to_f
        @allow_single_column_tables = allow_single_column_tables ? true : false
      end

      def to_h
        {
          extract_images: @extract_images,
          passwords: @passwords,
          extract_metadata: @extract_metadata,
          font_config: @font_config&.to_h,
          hierarchy: @hierarchy&.to_h,
          extract_annotations: @extract_annotations,
          top_margin_fraction: @top_margin_fraction,
          bottom_margin_fraction: @bottom_margin_fraction,
          allow_single_column_tables: @allow_single_column_tables
        }.compact
      end

      def font_config=(value)
        @font_config = normalize_font_config(value)
      end

      def hierarchy=(value)
        @hierarchy = normalize_hierarchy(value)
      end

      private

      def normalize_font_config(value)
        return nil if value.nil?
        return value if value.is_a?(FontConfig)
        return FontConfig.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{FontConfig}, Hash, or nil, got #{value.class}"
      end

      def normalize_hierarchy(value)
        return nil if value.nil?
        return value if value.is_a?(Hierarchy)
        return Hierarchy.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{Hierarchy}, Hash, or nil, got #{value.class}"
      end
    end

    # Image extraction configuration
    #
    # @example
    #   image = ImageExtraction.new(extract_images: true, target_dpi: 300)
    #
    # @example With auto-adjust DPI
    #   image = ImageExtraction.new(
    #     extract_images: true,
    #     auto_adjust_dpi: true,
    #     min_dpi: 150,
    #     max_dpi: 600
    #   )
    #
    class ImageExtraction
      attr_reader :extract_images, :target_dpi, :max_image_dimension,
                  :auto_adjust_dpi, :min_dpi, :max_dpi

      def initialize(
        extract_images: true,
        target_dpi: 300,
        max_image_dimension: 2000,
        auto_adjust_dpi: true,
        min_dpi: 150,
        max_dpi: 600
      )
        @extract_images = extract_images ? true : false
        @target_dpi = target_dpi.to_i
        @max_image_dimension = max_image_dimension.to_i
        @auto_adjust_dpi = auto_adjust_dpi ? true : false
        @min_dpi = min_dpi.to_i
        @max_dpi = max_dpi.to_i
      end

      def to_h
        {
          extract_images: @extract_images,
          target_dpi: @target_dpi,
          max_image_dimension: @max_image_dimension,
          auto_adjust_dpi: @auto_adjust_dpi,
          min_dpi: @min_dpi,
          max_dpi: @max_dpi
        }
      end
    end

    # Image preprocessing configuration for OCR
    #
    # @example Basic preprocessing
    #   preprocessing = ImagePreprocessing.new(
    #     binarization_method: "otsu",
    #     denoise: true
    #   )
    #
    # @example Advanced preprocessing
    #   preprocessing = ImagePreprocessing.new(
    #     target_dpi: 600,
    #     auto_rotate: true,
    #     deskew: true,
    #     denoise: true,
    #     contrast_enhance: true,
    #     binarization_method: "sauvola",
    #     invert_colors: false
    #   )
    #
    class ImagePreprocessing
      attr_reader :target_dpi, :auto_rotate, :deskew, :denoise,
                  :contrast_enhance, :binarization_method, :invert_colors

      VALID_BINARIZATION_METHODS = %w[otsu sauvola niblack wolf bradley adaptive].freeze

      def initialize(
        target_dpi: 300,
        auto_rotate: true,
        deskew: true,
        denoise: false,
        contrast_enhance: true,
        binarization_method: 'otsu',
        invert_colors: false
      )
        @target_dpi = target_dpi.to_i
        @auto_rotate = auto_rotate ? true : false
        @deskew = deskew ? true : false
        @denoise = denoise ? true : false
        @contrast_enhance = contrast_enhance ? true : false
        @binarization_method = binarization_method.to_s
        @invert_colors = invert_colors ? true : false

        # Validate binarization method
        return if VALID_BINARIZATION_METHODS.include?(@binarization_method)

        valid_methods = VALID_BINARIZATION_METHODS.join(', ')
        raise ArgumentError,
              "Invalid binarization_method: #{@binarization_method}. Valid methods are: #{valid_methods}"
      end

      def to_h
        {
          target_dpi: @target_dpi,
          auto_rotate: @auto_rotate,
          deskew: @deskew,
          denoise: @denoise,
          contrast_enhance: @contrast_enhance,
          binarization_method: @binarization_method,
          invert_colors: @invert_colors
        }
      end
    end

    # Token reduction configuration
    #
    # @example Disable token reduction
    #   token = TokenReduction.new(mode: "off")
    #
    # @example Light reduction
    #   token = TokenReduction.new(mode: "light", preserve_important_words: true)
    #
    # @example Aggressive reduction
    #   token = TokenReduction.new(mode: "aggressive", preserve_important_words: false)
    #
    class TokenReduction
      attr_reader :mode, :preserve_important_words

      VALID_MODES = %w[off light moderate aggressive maximum].freeze

      def initialize(mode: 'off', preserve_important_words: true)
        @mode = mode.to_s
        @preserve_important_words = preserve_important_words ? true : false

        # Validate mode against known valid modes
        return if VALID_MODES.include?(@mode)

        raise ArgumentError, "Invalid token reduction mode: #{@mode}. Valid modes are: #{VALID_MODES.join(', ')}"
      end

      def to_h
        {
          mode: @mode,
          preserve_important_words: @preserve_important_words
        }
      end
    end

    # HTML preprocessing configuration for content extraction
    class HtmlPreprocessing
      attr_reader :enabled, :preset, :remove_navigation, :remove_forms

      def initialize(enabled: nil, preset: nil, remove_navigation: nil, remove_forms: nil)
        @enabled = boolean_or_nil(enabled)
        @preset = preset&.to_sym
        @remove_navigation = boolean_or_nil(remove_navigation)
        @remove_forms = boolean_or_nil(remove_forms)
      end

      def to_h
        {
          enabled: @enabled,
          preset: @preset,
          remove_navigation: @remove_navigation,
          remove_forms: @remove_forms
        }.compact
      end

      private

      def boolean_or_nil(value)
        return nil if value.nil?

        value ? true : false
      end
    end

    # HTML rendering options for document conversion
    class HtmlOptions
      attr_reader :options

      def initialize(**options)
        normalized = options.transform_keys(&:to_sym)
        symbol_keys = %i[
          heading_style
          code_block_style
          highlight_style
          list_indent_type
          newline_style
          whitespace_mode
        ]
        symbol_keys.each do |key|
          normalized[key] = normalized[key]&.to_sym if normalized.key?(key)
        end
        if normalized[:preprocessing].is_a?(Hash)
          normalized[:preprocessing] = HtmlPreprocessing.new(**normalized[:preprocessing])
        end
        @options = normalized
      end

      def to_h
        @options.transform_values { |value| value.respond_to?(:to_h) ? value.to_h : value }
      end
    end

    # YAKE keyword extraction parameters
    class KeywordYakeParams
      attr_reader :window_size

      def initialize(window_size: 2)
        @window_size = window_size.to_i
      end

      def to_h
        { window_size: @window_size }
      end
    end

    # RAKE keyword extraction parameters
    class KeywordRakeParams
      attr_reader :min_word_length, :max_words_per_phrase

      def initialize(min_word_length: 1, max_words_per_phrase: 3)
        @min_word_length = min_word_length.to_i
        @max_words_per_phrase = max_words_per_phrase.to_i
      end

      def to_h
        {
          min_word_length: @min_word_length,
          max_words_per_phrase: @max_words_per_phrase
        }
      end
    end

    # Keyword extraction configuration for document analysis
    class Keywords
      attr_reader :algorithm, :max_keywords, :min_score, :ngram_range,
                  :language, :yake_params, :rake_params

      def initialize(
        algorithm: nil,
        max_keywords: nil,
        min_score: nil,
        ngram_range: nil,
        language: nil,
        yake_params: nil,
        rake_params: nil
      )
        @algorithm = algorithm&.to_s
        @max_keywords = max_keywords&.to_i
        @min_score = min_score&.to_f
        @ngram_range = ngram_range&.map(&:to_i)
        @language = language&.to_s
        @yake_params = normalize_nested(yake_params, KeywordYakeParams)
        @rake_params = normalize_nested(rake_params, KeywordRakeParams)
      end

      def to_h
        {
          algorithm: @algorithm,
          max_keywords: @max_keywords,
          min_score: @min_score,
          ngram_range: @ngram_range,
          language: @language,
          yake_params: @yake_params&.to_h,
          rake_params: @rake_params&.to_h
        }.compact
      end

      private

      def normalize_nested(value, klass)
        return nil if value.nil?
        return value if value.is_a?(klass)
        return klass.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{klass}, Hash, or nil, got #{value.class}"
      end
    end

    # Page tracking configuration for multi-page documents
    #
    # @example Enable page extraction
    #   pages = PageConfig.new(extract_pages: true)
    #
    # @example Enable page markers in content
    #   pages = PageConfig.new(insert_page_markers: true, marker_format: "--- PAGE {page_num} ---")
    #
    class PageConfig
      attr_reader :extract_pages, :insert_page_markers, :marker_format

      def initialize(
        extract_pages: false,
        insert_page_markers: false,
        marker_format: "\n\n<!-- PAGE {page_num} -->\n\n"
      )
        # Handle boolean conversion: treat 0 as false (like in C/FFI), but other truthy values as true
        @extract_pages = !extract_pages.nil? && extract_pages != false && extract_pages != 0
        @insert_page_markers = !insert_page_markers.nil? && insert_page_markers != false && insert_page_markers != 0
        @marker_format = marker_format.to_s
      end

      def to_h
        {
          extract_pages: @extract_pages,
          insert_page_markers: @insert_page_markers,
          marker_format: @marker_format
        }
      end
    end

    # Post-processor configuration
    #
    # @example Enable all post-processors
    #   postprocessor = PostProcessor.new(enabled: true)
    #
    # @example Enable specific processors
    #   postprocessor = PostProcessor.new(
    #     enabled: true,
    #     enabled_processors: ["quality", "formatting"]
    #   )
    #
    # @example Disable specific processors
    #   postprocessor = PostProcessor.new(
    #     enabled: true,
    #     disabled_processors: ["token_reduction"]
    #   )
    #
    class PostProcessor
      attr_reader :enabled, :enabled_processors, :disabled_processors

      def initialize(
        enabled: true,
        enabled_processors: nil,
        disabled_processors: nil
      )
        @enabled = enabled ? true : false
        @enabled_processors = enabled_processors&.map(&:to_s)
        @disabled_processors = disabled_processors&.map(&:to_s)
      end

      def to_h
        {
          enabled: @enabled,
          enabled_processors: @enabled_processors,
          disabled_processors: @disabled_processors
        }.compact
      end
    end

    # Hardware acceleration configuration for ONNX Runtime
    #
    # Controls which execution provider (CPU, CoreML, CUDA, TensorRT) is used
    # for inference in layout detection and embedding generation.
    #
    # @example Auto-select provider (CoreML on macOS, CUDA on Linux, CPU elsewhere)
    #   acceleration = Acceleration.new
    #
    # @example Force CPU only
    #   acceleration = Acceleration.new(provider: 'cpu')
    #
    # @example Use CUDA with specific device
    #   acceleration = Acceleration.new(provider: 'cuda', device_id: 0)
    #
    class Acceleration
      attr_reader :provider, :device_id

      def initialize(provider: 'auto', device_id: 0)
        @provider = provider.to_s
        @device_id = device_id.to_i
      end

      def to_h
        {
          provider: @provider,
          device_id: @device_id
        }
      end
    end

    # Email extraction configuration
    #
    # @example With fallback codepage
    #   email = Email.new(msg_fallback_codepage: 1251)
    #
    class Email
      attr_reader :msg_fallback_codepage

      def initialize(msg_fallback_codepage: nil)
        @msg_fallback_codepage = msg_fallback_codepage&.to_i
      end

      def to_h
        h = {}
        h[:msg_fallback_codepage] = @msg_fallback_codepage unless @msg_fallback_codepage.nil?
        h
      end
    end

    # Layout detection configuration
    #
    # @example Basic usage with fast preset
    #   layout = LayoutDetection.new(preset: "fast")
    #
    # @example Accurate preset with custom threshold
    #   layout = LayoutDetection.new(
    #     preset: "accurate",
    #     confidence_threshold: 0.5,
    #     apply_heuristics: true
    #   )
    #
    class LayoutDetection
      attr_reader :preset, :confidence_threshold, :apply_heuristics, :table_model

      def initialize(preset: 'fast', confidence_threshold: nil, apply_heuristics: true, table_model: nil)
        @preset = preset.to_s
        @confidence_threshold = confidence_threshold&.to_f
        @apply_heuristics = apply_heuristics ? true : false
        @table_model = table_model&.to_s
      end

      def to_h
        {
          preset: @preset,
          confidence_threshold: @confidence_threshold,
          apply_heuristics: @apply_heuristics,
          table_model: @table_model
        }.compact
      end
    end

    # Concurrency configuration for thread pool management
    #
    # @example Limit max threads
    #   concurrency = Concurrency.new(max_threads: 4)
    #
    class Concurrency
      attr_reader :max_threads

      def initialize(max_threads: nil)
        @max_threads = max_threads&.to_i
      end

      def to_h
        h = {}
        h[:max_threads] = @max_threads unless @max_threads.nil?
        h
      end
    end

    # Main extraction configuration
    #
    # @example Basic usage
    #   config = Extraction.new(use_cache: true, force_ocr: true)
    #
    # @example With OCR
    #   ocr = Config::OCR.new(backend: "tesseract", language: "eng")
    #   config = Extraction.new(ocr: ocr)
    #
    # @example With image extraction
    #   image = Config::ImageExtraction.new(extract_images: true, target_dpi: 600)
    #   config = Extraction.new(image_extraction: image)
    #
    # @example With post-processing
    #   postprocessor = Config::PostProcessor.new(
    #     enabled: true,
    #     enabled_processors: ["quality"]
    #   )
    #   config = Extraction.new(postprocessor: postprocessor)
    #
    # @example With document structure
    #   config = Extraction.new(include_document_structure: true)
    #
    # @example With all options
    #   config = Extraction.new(
    #     use_cache: true,
    #     enable_quality_processing: true,
    #     force_ocr: false,
    #     include_document_structure: true,
    #     ocr: Config::OCR.new(language: "deu"),
    #     chunking: Config::Chunking.new(max_chars: 500),
    #     language_detection: Config::LanguageDetection.new(enabled: true),
    #     pdf_options: Config::PDF.new(extract_images: true, passwords: ["secret"]),
    #     image_extraction: Config::ImageExtraction.new(target_dpi: 600),
    #     postprocessor: Config::PostProcessor.new(enabled: true)
    #   )
    #
    class Extraction
      attr_reader :use_cache, :enable_quality_processing, :force_ocr, :disable_ocr, :force_ocr_pages,
                  :include_document_structure,
                  :ocr, :chunking, :language_detection, :pdf_options,
                  :images, :postprocessor,
                  :token_reduction, :keywords, :html_options, :pages,
                  :max_concurrent_extractions, :output_format, :result_format,
                  :security_limits, :layout, :concurrency,
                  :cache_namespace, :cache_ttl_secs, :extraction_timeout_secs,
                  :max_archive_depth, :acceleration, :email

      # Alias for backward compatibility - image_extraction is the canonical name
      alias image_extraction images

      # Load configuration from a file.
      #
      # Detects the file format from the extension (.toml, .yaml, .json)
      # and loads the configuration accordingly.
      #
      # @param path [String] Path to the configuration file
      # @return [Kreuzberg::Config::Extraction] Loaded configuration object
      #
      # @example Load from TOML
      #   config = Kreuzberg::Config::Extraction.from_file("config.toml")
      #
      # @example Load from YAML
      #   config = Kreuzberg::Config::Extraction.from_file("config.yaml")
      #
      # Keys that are allowed in the Extraction config
      ALLOWED_KEYS = %i[
        use_cache enable_quality_processing force_ocr disable_ocr force_ocr_pages
        include_document_structure ocr chunking language_detection pdf_options image_extraction
        postprocessor token_reduction keywords html_options pages
        max_concurrent_extractions output_format result_format
        security_limits layout concurrency cache_namespace cache_ttl_secs extraction_timeout_secs
        max_archive_depth acceleration email
      ].freeze

      # Aliases for backward compatibility
      KEY_ALIASES = {
        images: :image_extraction
      }.freeze

      # Valid output format values (case-insensitive, normalized internally)
      VALID_OUTPUT_FORMATS = %w[plain markdown html djot].freeze

      # Valid result format values (case-insensitive, normalized internally)
      VALID_RESULT_FORMATS = %w[unified elements element_based].freeze

      def self.from_file(path)
        hash = Kreuzberg._config_from_file_native(path)
        new(**normalize_hash_keys(hash))
      end

      # Normalize hash keys from native function
      # - Converts string keys to symbols
      # - Maps aliased keys to their canonical names
      # - Filters out unknown keys
      def self.normalize_hash_keys(hash)
        symbolized = hash.transform_keys(&:to_sym)

        # Apply key aliases
        KEY_ALIASES.each do |from, to|
          symbolized[to] = symbolized.delete(from) if symbolized.key?(from) && !symbolized.key?(to)
        end

        # Filter to only allowed keys
        symbolized.slice(*ALLOWED_KEYS)
      end

      private_class_method :normalize_hash_keys

      # Discover configuration file in current or parent directories.
      #
      # Searches for kreuzberg.toml, kreuzberg.yaml, or kreuzberg.json in the current
      # directory and parent directories.
      #
      # @return [Kreuzberg::Config::Extraction, nil] Loaded configuration object or nil if not found
      #
      # @example
      #   config = Kreuzberg::Config::Extraction.discover
      #   if config
      #     # Use discovered config
      #   end
      #
      def self.discover
        hash = Kreuzberg._config_discover_native
        return nil if hash.nil?

        new(**normalize_hash_keys(hash))
      end

      def initialize(hash = nil, # rubocop:disable Metrics/MethodLength
                     use_cache: true,
                     enable_quality_processing: true,
                     force_ocr: false,
                     disable_ocr: false,
                     force_ocr_pages: nil,
                     include_document_structure: false,
                     ocr: nil,
                     chunking: nil,
                     language_detection: nil,
                     pdf_options: nil,
                     image_extraction: nil,
                     postprocessor: nil,
                     token_reduction: nil,
                     keywords: nil,
                     html_options: nil,
                     pages: nil,
                     max_concurrent_extractions: nil,
                     output_format: nil,
                     result_format: nil,
                     security_limits: nil,
                     layout: nil,
                     concurrency: nil,
                     cache_namespace: nil,
                     cache_ttl_secs: nil,
                     extraction_timeout_secs: nil,
                     max_archive_depth: 3,
                     acceleration: nil,
                     email: nil)
        kwargs = {
          use_cache: use_cache, enable_quality_processing: enable_quality_processing,
          force_ocr: force_ocr, disable_ocr: disable_ocr, force_ocr_pages: force_ocr_pages,
          include_document_structure: include_document_structure,
          ocr: ocr, chunking: chunking, language_detection: language_detection,
          pdf_options: pdf_options, image_extraction: image_extraction,
          postprocessor: postprocessor,
          token_reduction: token_reduction, keywords: keywords, html_options: html_options,
          pages: pages, max_concurrent_extractions: max_concurrent_extractions,
          output_format: output_format, result_format: result_format,
          security_limits: security_limits, layout: layout,
          concurrency: concurrency,
          cache_namespace: cache_namespace,
          cache_ttl_secs: cache_ttl_secs,
          extraction_timeout_secs: extraction_timeout_secs,
          max_archive_depth: max_archive_depth,
          acceleration: acceleration,
          email: email
        }
        extracted = extract_from_hash(hash, kwargs)

        assign_attributes(extracted)
      end

      def extract_from_hash(hash, defaults)
        return defaults unless hash.is_a?(Hash)

        hash = hash.transform_keys(&:to_sym)
        defaults.merge(hash.slice(*defaults.keys))
      end

      def assign_attributes(params) # rubocop:disable Metrics/MethodLength
        @use_cache = params[:use_cache] ? true : false
        @enable_quality_processing = params[:enable_quality_processing] ? true : false
        @force_ocr = params[:force_ocr] ? true : false
        @disable_ocr = params[:disable_ocr] ? true : false
        @force_ocr_pages = params[:force_ocr_pages]
        @include_document_structure = params[:include_document_structure] ? true : false
        @ocr = normalize_config(params[:ocr], OCR)
        @chunking = normalize_config(params[:chunking], Chunking)
        @language_detection = normalize_config(params[:language_detection], LanguageDetection)
        @pdf_options = normalize_config(params[:pdf_options], PDF)
        @images = normalize_config(params[:image_extraction], ImageExtraction)
        @postprocessor = normalize_config(params[:postprocessor], PostProcessor)
        @token_reduction = normalize_config(params[:token_reduction], TokenReduction)
        @keywords = normalize_config(params[:keywords], Keywords)
        @html_options = normalize_config(params[:html_options], HtmlOptions)
        @pages = normalize_config(params[:pages], PageConfig)
        @layout = normalize_config(params[:layout], LayoutDetection)
        @concurrency = normalize_config(params[:concurrency], Concurrency)
        @acceleration = normalize_config(params[:acceleration], Acceleration)
        @email = normalize_config(params[:email], Email)
        @max_concurrent_extractions = params[:max_concurrent_extractions]&.to_i
        @max_archive_depth = params[:max_archive_depth]&.to_i || 3
        @output_format = validate_output_format(params[:output_format])
        @result_format = validate_result_format(params[:result_format])
        @cache_namespace = params[:cache_namespace]
        @cache_ttl_secs = params[:cache_ttl_secs]&.to_i
        @extraction_timeout_secs = params[:extraction_timeout_secs]&.to_i
        @security_limits = params[:security_limits]
      end

      def validate_output_format(value)
        return nil if value.nil?

        str_value = value.to_s.downcase
        return str_value if VALID_OUTPUT_FORMATS.include?(str_value)

        raise ArgumentError,
              "Invalid output_format: #{value}. Valid values: #{VALID_OUTPUT_FORMATS.join(', ')}"
      end

      def validate_result_format(value)
        return nil if value.nil?

        str_value = value.to_s.downcase
        return str_value if VALID_RESULT_FORMATS.include?(str_value)

        raise ArgumentError,
              "Invalid result_format: #{value}. Valid values: #{VALID_RESULT_FORMATS.join(', ')}"
      end

      def to_h
        core_config_hash.merge(sub_config_hash).compact
      end

      def core_config_hash
        {
          use_cache: @use_cache,
          enable_quality_processing: @enable_quality_processing,
          force_ocr: @force_ocr,
          disable_ocr: @disable_ocr,
          force_ocr_pages: @force_ocr_pages,
          include_document_structure: @include_document_structure,
          max_concurrent_extractions: @max_concurrent_extractions,
          max_archive_depth: @max_archive_depth,
          output_format: @output_format,
          result_format: @result_format,
          cache_namespace: @cache_namespace,
          cache_ttl_secs: @cache_ttl_secs,
          extraction_timeout_secs: @extraction_timeout_secs
        }
      end

      def sub_config_hash
        {
          ocr: @ocr&.to_h, chunking: @chunking&.to_h,
          language_detection: @language_detection&.to_h, pdf_options: @pdf_options&.to_h,
          image_extraction: @images&.to_h, postprocessor: @postprocessor&.to_h,
          token_reduction: @token_reduction&.to_h, keywords: @keywords&.to_h,
          html_options: @html_options&.to_h, pages: @pages&.to_h,
          layout: @layout&.to_h, concurrency: @concurrency&.to_h,
          acceleration: @acceleration&.to_h, email: @email&.to_h
        }
      end

      # Serialize configuration to JSON string
      #
      # @return [String] JSON representation of the configuration
      #
      # @example
      #   config = Extraction.new(use_cache: true)
      #   json = config.to_json
      #   puts json  # => "{\"use_cache\":true,...}"
      #
      def to_json(*_args)
        json_hash = to_h
        # Convert to JSON directly - the native function has issues
        JSON.generate(json_hash)
      end

      # Get a field from the configuration
      #
      # Supports dot notation for nested fields (e.g., "ocr.backend")
      #
      # @param field_name [String, Symbol] Field name to retrieve
      # @return [Object, nil] Parsed field value, or nil if field doesn't exist
      #
      # @example Get a top-level field
      #   config = Extraction.new(use_cache: true)
      #   config.get_field("use_cache")  # => true
      #
      # @example Get a nested field
      #   config = Extraction.new(ocr: OCR.new(backend: "tesseract"))
      #   config.get_field("ocr.backend")  # => "tesseract"
      #
      def get_field(field_name)
        json_hash = to_h
        field_path = field_name.to_s.split('.')

        # Navigate the nested hash using the field path
        field_path.reduce(json_hash) do |current, key|
          case current
          when Hash
            # Check both symbol and string keys, prefer symbol if exists
            if current.key?(key.to_sym)
              current[key.to_sym]
            elsif current.key?(key.to_s)
              current[key.to_s]
            end
          end
        end
      end

      # Merge another configuration into this one
      #
      # Returns a new configuration with fields from the other config overriding
      # fields from this config (shallow merge).
      #
      # @param other [Extraction, Hash] Configuration to merge
      # @return [Extraction] New merged configuration
      #
      # @example
      #   base = Extraction.new(use_cache: true, force_ocr: false)
      #   override = Extraction.new(force_ocr: true)
      #   merged = base.merge(override)
      #   merged.use_cache   # => true
      #   merged.force_ocr   # => true
      #
      def merge(other)
        other_config = other.is_a?(Extraction) ? other : Extraction.new(**other)
        # Merge the two config hashes
        merged_hash = to_h.merge(other_config.to_h)
        Extraction.new(**merged_hash)
      end

      # Merge another configuration into this one (mutating)
      #
      # Modifies this configuration in-place by merging fields from another config.
      #
      # @param other [Extraction, Hash] Configuration to merge
      # @return [self]
      #
      # @example
      #   base = Extraction.new(use_cache: true, force_ocr: false)
      #   override = Extraction.new(force_ocr: true)
      #   base.merge!(override)
      #   base.use_cache   # => true
      #   base.force_ocr   # => true
      #
      def merge!(other)
        other_config = other.is_a?(Extraction) ? other : Extraction.new(**other)
        merged = merge(other_config)
        update_from_merged(merged)
        self
      end

      # Set a configuration field using hash-like syntax
      #
      # @param key [Symbol, String] Field name to set
      # @param value [Object] Value to set
      # @return [Object] The value that was set
      #
      # @example
      #   config = Extraction.new(use_cache: true)
      #   config[:use_cache] = false
      #   config[:force_ocr] = true
      #
      # rubocop:disable Metrics/MethodLength
      def []=(key, value)
        key_sym = key.to_sym
        case key_sym
        when :use_cache
          @use_cache = value ? true : false
        when :enable_quality_processing
          @enable_quality_processing = value ? true : false
        when :force_ocr
          @force_ocr = value ? true : false
        when :disable_ocr
          @disable_ocr = value ? true : false
        when :force_ocr_pages
          @force_ocr_pages = value
        when :include_document_structure
          @include_document_structure = value ? true : false
        when :ocr
          @ocr = normalize_config(value, OCR)
        when :chunking
          @chunking = normalize_config(value, Chunking)
        when :language_detection
          @language_detection = normalize_config(value, LanguageDetection)
        when :pdf_options
          @pdf_options = normalize_config(value, PDF)
        when :image_extraction
          @images = normalize_config(value, ImageExtraction)
        when :postprocessor
          @postprocessor = normalize_config(value, PostProcessor)
        when :token_reduction
          @token_reduction = normalize_config(value, TokenReduction)
        when :keywords
          @keywords = normalize_config(value, Keywords)
        when :html_options
          @html_options = normalize_config(value, HtmlOptions)
        when :pages
          @pages = normalize_config(value, PageConfig)
        when :layout
          @layout = normalize_config(value, LayoutDetection)
        when :concurrency
          @concurrency = normalize_config(value, Concurrency)
        when :acceleration
          @acceleration = normalize_config(value, Acceleration)
        when :email
          @email = normalize_config(value, Email)
        when :max_archive_depth
          @max_archive_depth = value&.to_i || 3
        when :max_concurrent_extractions
          @max_concurrent_extractions = value&.to_i
        when :output_format
          @output_format = validate_output_format(value)
        when :result_format
          @result_format = validate_result_format(value)
        when :cache_namespace
          @cache_namespace = value
        when :cache_ttl_secs
          @cache_ttl_secs = value&.to_i
        when :extraction_timeout_secs
          @extraction_timeout_secs = value&.to_i
        else
          raise ArgumentError, "Unknown configuration key: #{key}"
        end
      end
      # rubocop:enable Metrics/MethodLength

      # Get a configuration field using hash-like syntax
      #
      # @param key [Symbol, String] Field name to get
      # @return [Object, nil] The field value
      #
      # @example
      #   config = Extraction.new(use_cache: true)
      #   config[:use_cache]  # => true
      #
      def [](key)
        send(key.to_sym)
      rescue NoMethodError
        nil
      end

      # Set output_format attribute
      #
      # @param value [String, nil] Output format value
      # @return [String, nil] The value that was set
      #
      def output_format=(value)
        @output_format = validate_output_format(value)
      end

      # Set result_format attribute
      #
      # @param value [String, nil] Result format value
      # @return [String, nil] The value that was set
      #
      def result_format=(value)
        @result_format = validate_result_format(value)
      end

      private

      def normalize_config(value, klass)
        return nil if value.nil?
        return value if value.is_a?(klass)
        return klass.new(**value.transform_keys(&:to_sym)) if value.is_a?(Hash)

        raise ArgumentError, "Expected #{klass}, Hash, or nil, got #{value.class}"
      end

      def update_from_merged(merged)
        update_core_options(merged)
        update_processing_options(merged)
        update_output_options(merged)
      end

      def update_core_options(merged)
        @use_cache = merged.use_cache
        @enable_quality_processing = merged.enable_quality_processing
        @force_ocr = merged.force_ocr
        @disable_ocr = merged.disable_ocr
        @force_ocr_pages = merged.force_ocr_pages
        @include_document_structure = merged.include_document_structure
        @ocr = merged.ocr
        @chunking = merged.chunking
        @language_detection = merged.language_detection
      end

      def update_processing_options(merged)
        @pdf_options = merged.pdf_options
        @images = merged.image_extraction
        @postprocessor = merged.postprocessor
        @token_reduction = merged.token_reduction
        @keywords = merged.keywords
        @html_options = merged.html_options
        @pages = merged.pages
        @layout = merged.layout
        @acceleration = merged.acceleration
        @email = merged.email
        @max_archive_depth = merged.max_archive_depth
      end

      def update_output_options(merged)
        @concurrency = merged.concurrency
        @max_concurrent_extractions = merged.max_concurrent_extractions
        @output_format = merged.output_format
        @result_format = merged.result_format
        @cache_namespace = merged.cache_namespace
        @cache_ttl_secs = merged.cache_ttl_secs
        @extraction_timeout_secs = merged.extraction_timeout_secs
      end
    end
  end
end
