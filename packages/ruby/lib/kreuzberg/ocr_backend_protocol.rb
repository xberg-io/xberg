# frozen_string_literal: true

module Kreuzberg
  # @example Implementing a custom OCR backend
  # @example Implementing an OCR backend with initialization
  module OcrBackendProtocol
    # @return [String] Unique backend identifier
    # @example
    def name
      raise NotImplementedError, "#{self.class} must implement #name"
    end

    # Process image bytes and extract text via OCR.
    #
    # This method receives raw image data (PNG, JPEG, TIFF, etc.) and an OCR configuration
    # hash. It must return the extracted text as a string.
    #
    # The config hash contains OCR settings such as:
    # - "language" [String] - Language code (e.g., "eng", "deu", "fra")
    # - "backend" [String] - Backend name (same as #name)
    # - Additional backend-specific settings
    #
    # @param image_bytes [String] Binary image data (PNG, JPEG, TIFF, etc.)
    # @param config [Hash] OCR configuration with the following keys:
    #   - "language" [String] - Language code for OCR (e.g., "eng", "deu")
    #   - "backend" [String] - Backend name
    #
    # @return [String] Extracted text content
    #
    # @example
    #   def process_image(image_bytes, config)
    #     language = config["language"] || "eng"
    #     text = my_ocr_engine.recognize(image_bytes, language: language)
    #     text
    #   end
    def process_image(image_bytes, config)
      raise NotImplementedError, "#{self.class} must implement #process_image(image_bytes, config)"
    end
  end
end
