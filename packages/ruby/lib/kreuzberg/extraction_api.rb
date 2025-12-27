# frozen_string_literal: true

module Kreuzberg
  module ExtractionAPI
    # @param path [String, Pathname] Path to the document file to extract
    # @param mime_type [String, nil] Optional MIME type for the file (e.g., 'application/pdf').
    # @param config [Config::Extraction, Hash, nil] Extraction configuration controlling
    # @return [Result] Extraction result containing content, metadata, tables, and images
    # @raise [Errors::IOError] If the file cannot be read or access is denied
    # @raise [Errors::ParsingError] If document parsing fails
    # @raise [Errors::UnsupportedFormatError] If the file format is not supported
    # @raise [Errors::OCRError] If OCR is enabled and fails
    # @raise [Errors::MissingDependencyError] If a required dependency is missing
    # @example Extract a PDF file
    # @example Extract with explicit MIME type
    # @example Extract with OCR enabled
    def extract_file_sync(path, mime_type: nil, config: nil)
      opts = normalize_config(config)
      hash = if mime_type
               native_extract_file_sync(path.to_s, mime_type.to_s, **opts)
             else
               native_extract_file_sync(path.to_s, **opts)
             end
      result = Result.new(hash)
      record_cache_entry!(result, opts)
      result
    end

    # Synchronously extract content from byte data.
    #
    # Performs document extraction directly from binary data in memory. Useful for
    # extracting content from files already loaded into memory or from network streams.
    #
    # @param data [String] Binary document data (can contain any byte values)
    # @param mime_type [String] MIME type of the data (required, e.g., 'application/pdf').
    #   This parameter is mandatory to guide the extraction engine.
    # @param config [Config::Extraction, Hash, nil] Extraction configuration. Accepts
    #   either a {Config::Extraction} object or a configuration hash.
    #
    # @return [Result] Extraction result containing content, metadata, tables, and images
    #
    # @raise [Errors::ParsingError] If document parsing fails
    # @raise [Errors::UnsupportedFormatError] If the MIME type is not supported
    # @raise [Errors::OCRError] If OCR is enabled and fails
    # @raise [Errors::MissingDependencyError] If a required dependency is missing
    #
    # @example Extract PDF from memory
    #   pdf_data = File.read("document.pdf", binmode: true)
    #   result = Kreuzberg.extract_bytes_sync(pdf_data, "application/pdf")
    #   puts result.content
    #
    # @example Extract from a network stream
    #   response = HTTParty.get("https://example.com/document.docx")
    #   result = Kreuzberg.extract_bytes_sync(response.body, "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
    def extract_bytes_sync(data, mime_type, config: nil)
      opts = normalize_config(config)
      hash = native_extract_bytes_sync(data.to_s, mime_type.to_s, **opts)
      result = Result.new(hash)
      record_cache_entry!(result, opts)
      result
    end

    # Synchronously extract content from multiple files.
    #
    # Processes multiple files in a single batch operation. Files are extracted sequentially,
    # and results maintain the same order as the input paths. This is useful for bulk
    # processing multiple documents with consistent configuration.
    #
    # @param paths [Array<String, Pathname>] Array of file paths to extract. Each path
    #   is converted to a string and MIME type is auto-detected from extension.
    # @param config [Config::Extraction, Hash, nil] Extraction configuration applied to all files.
    #   Accepts either a {Config::Extraction} object or a configuration hash.
    #
    # @return [Array<Result>] Array of extraction results in the same order as input paths.
    #   Array length matches the input paths length.
    #
    # @raise [Errors::IOError] If any file cannot be read
    # @raise [Errors::ParsingError] If any document parsing fails
    # @raise [Errors::UnsupportedFormatError] If any file format is not supported
    # @raise [Errors::OCRError] If OCR is enabled and fails on any document
    # @raise [Errors::MissingDependencyError] If a required dependency is missing
    #
    # @example Batch extract multiple PDFs
    #   paths = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
    #   results = Kreuzberg.batch_extract_files_sync(paths)
    #   results.each_with_index do |result, idx|
    #     puts "File #{idx}: #{result.content.length} characters"
    #   end
    #
    # @example Batch extract with consistent configuration
    #   paths = Dir.glob("documents/*.pdf")
    #   config = Kreuzberg::Config::Extraction.new(force_ocr: true)
    #   results = Kreuzberg.batch_extract_files_sync(paths, config: config)
    def batch_extract_files_sync(paths, config: nil)
      opts = normalize_config(config)
      hashes = native_batch_extract_files_sync(paths.map(&:to_s), **opts)
      results = hashes.map { |hash| Result.new(hash) }
      record_cache_entry!(results, opts)
      results
    end

    # Asynchronously extract content from a file.
    #
    # Non-blocking extraction that returns a {Result} promise. Extraction is performed
    # in the background using native threads or the Tokio runtime. This method is
    # preferred for I/O-bound operations and integrating with async workflows.
    #
    # @param path [String, Pathname] Path to the document file to extract
    # @param mime_type [String, nil] Optional MIME type for the file (e.g., 'application/pdf').
    #   If omitted, type is detected from file extension.
    # @param config [Config::Extraction, Hash, nil] Extraction configuration. Accepts
    #   either a {Config::Extraction} object or a configuration hash.
    #
    # @return [Result] Extraction result containing content, metadata, tables, and images.
    #   In async contexts, this result is available upon method return.
    #
    # @raise [Errors::IOError] If the file cannot be read or access is denied
    # @raise [Errors::ParsingError] If document parsing fails
    # @raise [Errors::UnsupportedFormatError] If the file format is not supported
    # @raise [Errors::OCRError] If OCR is enabled and fails
    # @raise [Errors::MissingDependencyError] If a required dependency is missing
    #
    # @example Extract a PDF file asynchronously
    #   result = Kreuzberg.extract_file("large_document.pdf")
    #   puts result.content
    #
    # @example Extract with custom OCR configuration
    #   config = Kreuzberg::Config::Extraction.new(
    #     ocr: Kreuzberg::Config::OCR.new(language: "deu")
    #   )
    #   result = Kreuzberg.extract_file("document.pdf", config: config)
    def extract_file(path, mime_type: nil, config: nil)
      opts = normalize_config(config)
      hash = if mime_type
               native_extract_file(path.to_s, mime_type.to_s, **opts)
             else
               native_extract_file(path.to_s, **opts)
             end
      result = Result.new(hash)
      record_cache_entry!(result, opts)
      result
    end

    # Asynchronously extract content from byte data.
    #
    # Non-blocking extraction from in-memory binary data. Like {#extract_file},
    # this performs extraction in the background, making it suitable for handling
    # high-volume extraction workloads without blocking the main thread.
    #
    # @param data [String] Binary document data (can contain any byte values)
    # @param mime_type [String] MIME type of the data (required, e.g., 'application/pdf').
    #   This parameter is mandatory to guide the extraction engine.
    # @param config [Config::Extraction, Hash, nil] Extraction configuration. Accepts
    #   either a {Config::Extraction} object or a configuration hash.
    #
    # @return [Result] Extraction result containing content, metadata, tables, and images
    #
    # @raise [Errors::ParsingError] If document parsing fails
    # @raise [Errors::UnsupportedFormatError] If the MIME type is not supported
    # @raise [Errors::OCRError] If OCR is enabled and fails
    # @raise [Errors::MissingDependencyError] If a required dependency is missing
    #
    # @example Extract PDF from memory asynchronously
    #   pdf_data = File.read("document.pdf", binmode: true)
    #   result = Kreuzberg.extract_bytes(pdf_data, "application/pdf")
    #   puts result.content
    #
    # @example Extract with image extraction
    #   data = File.read("file.docx", binmode: true)
    #   config = Kreuzberg::Config::Extraction.new(
    #     image_extraction: Kreuzberg::Config::ImageExtraction.new(extract_images: true)
    #   )
    #   result = Kreuzberg.extract_bytes(data, "application/vnd.openxmlformats-officedocument.wordprocessingml.document", config: config)
    def extract_bytes(data, mime_type, config: nil)
      opts = normalize_config(config)
      hash = native_extract_bytes(data.to_s, mime_type.to_s, **opts)
      result = Result.new(hash)
      record_cache_entry!(result, opts)
      result
    end

    # Asynchronously extract content from multiple files.
    #
    # Non-blocking batch extraction from multiple files. Results maintain the same order
    # as input paths. This is the preferred method for bulk processing when non-blocking
    # I/O is required (e.g., in web servers or async applications).
    #
    # @param paths [Array<String, Pathname>] Array of file paths to extract. Each path
    #   is converted to a string and MIME type is auto-detected from extension.
    # @param config [Config::Extraction, Hash, nil] Extraction configuration applied to all files.
    #   Accepts either a {Config::Extraction} object or a configuration hash.
    #
    # @return [Array<Result>] Array of extraction results in the same order as input paths.
    #   Array length matches the input paths length.
    #
    # @raise [Errors::IOError] If any file cannot be read
    # @raise [Errors::ParsingError] If any document parsing fails
    # @raise [Errors::UnsupportedFormatError] If any file format is not supported
    # @raise [Errors::OCRError] If OCR is enabled and fails on any document
    # @raise [Errors::MissingDependencyError] If a required dependency is missing
    #
    # @example Batch extract multiple files asynchronously
    #   paths = ["invoice_1.pdf", "invoice_2.pdf", "invoice_3.pdf"]
    #   results = Kreuzberg.batch_extract_files(paths)
    #   results.each_with_index do |result, idx|
    #     puts "Invoice #{idx}: #{result.detected_languages}"
    #   end
    #
    # @example Batch extract with chunking
    #   paths = Dir.glob("reports/*.docx")
    #   config = Kreuzberg::Config::Extraction.new(
    #     chunking: Kreuzberg::Config::Chunking.new(max_chars: 1000, max_overlap: 200)
    #   )
    #   results = Kreuzberg.batch_extract_files(paths, config: config)
    def batch_extract_files(paths, config: nil)
      opts = normalize_config(config)
      hashes = native_batch_extract_files(paths.map(&:to_s), **opts)
      results = hashes.map { |hash| Result.new(hash) }
      record_cache_entry!(results, opts)
      results
    end

    # Synchronously extract content from multiple byte data sources.
    #
    # Processes multiple in-memory binary documents in a single batch operation. Results
    # maintain the same order as the input data array. The mime_types array must have
    # the same length as the data_array.
    #
    # @param data_array [Array<String>] Array of binary document data. Each element can
    #   contain any byte values (e.g., PDF binary data).
    # @param mime_types [Array<String>] Array of MIME types corresponding to each data item.
    #   Must be the same length as data_array (e.g., ["application/pdf", "application/msword"]).
    # @param config [Config::Extraction, Hash, nil] Extraction configuration applied to all items.
    #   Accepts either a {Config::Extraction} object or a configuration hash.
    #
    # @return [Array<Result>] Array of extraction results in the same order as input data.
    #   Array length matches the data_array length.
    #
    # @raise [ArgumentError] If data_array and mime_types have different lengths
    # @raise [Errors::ParsingError] If any document parsing fails
    # @raise [Errors::UnsupportedFormatError] If any MIME type is not supported
    # @raise [Errors::OCRError] If OCR is enabled and fails on any document
    # @raise [Errors::MissingDependencyError] If a required dependency is missing
    #
    # @example Batch extract binary documents
    #   pdf_data_1 = File.read("doc1.pdf", binmode: true)
    #   pdf_data_2 = File.read("doc2.pdf", binmode: true)
    #   docx_data = File.read("report.docx", binmode: true)
    #
    #   data = [pdf_data_1, pdf_data_2, docx_data]
    #   types = ["application/pdf", "application/pdf", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"]
    #   results = Kreuzberg.batch_extract_bytes_sync(data, types)
    #   results.each { |r| puts r.content }
    def batch_extract_bytes_sync(data_array, mime_types, config: nil)
      opts = normalize_config(config)
      hashes = native_batch_extract_bytes_sync(data_array.map(&:to_s), mime_types.map(&:to_s), **opts)
      results = hashes.map { |hash| Result.new(hash) }
      record_cache_entry!(results, opts)
      results
    end

    # Asynchronously extract content from multiple byte data sources.
    #
    # Non-blocking batch extraction from multiple in-memory binary documents. Results
    # maintain the same order as the input data array. This method is preferred when
    # processing multiple documents without blocking (e.g., handling multiple uploads).
    #
    # @param data_array [Array<String>] Array of binary document data. Each element can
    #   contain any byte values (e.g., PDF binary data).
    # @param mime_types [Array<String>] Array of MIME types corresponding to each data item.
    #   Must be the same length as data_array (e.g., ["application/pdf", "application/msword"]).
    # @param config [Config::Extraction, Hash, nil] Extraction configuration applied to all items.
    #   Accepts either a {Config::Extraction} object or a configuration hash.
    #
    # @return [Array<Result>] Array of extraction results in the same order as input data.
    #   Array length matches the data_array length.
    #
    # @raise [ArgumentError] If data_array and mime_types have different lengths
    # @raise [Errors::ParsingError] If any document parsing fails
    # @raise [Errors::UnsupportedFormatError] If any MIME type is not supported
    # @raise [Errors::OCRError] If OCR is enabled and fails on any document
    # @raise [Errors::MissingDependencyError] If a required dependency is missing
    #
    # @example Batch extract uploaded documents asynchronously
    #   # From a web request with multiple file uploads
    #   uploaded_files = params[:files]  # Array of uploaded file objects
    #   data = uploaded_files.map(&:read)
    #   types = uploaded_files.map(&:content_type)
    #
    #   results = Kreuzberg.batch_extract_bytes(data, types)
    #   results.each { |r| puts r.content }
    #
    # @example Batch extract with OCR
    #   data = [scan_1_bytes, scan_2_bytes, scan_3_bytes]
    #   types = ["image/png", "image/png", "image/png"]
    #   config = Kreuzberg::Config::Extraction.new(force_ocr: true)
    #   results = Kreuzberg.batch_extract_bytes(data, types, config: config)
    def batch_extract_bytes(data_array, mime_types, config: nil)
      opts = normalize_config(config)
      hashes = native_batch_extract_bytes(data_array.map(&:to_s), mime_types.map(&:to_s), **opts)
      results = hashes.map { |hash| Result.new(hash) }
      record_cache_entry!(results, opts)
      results
    end

    def normalize_config(config)
      return {} if config.nil?
      return config if config.is_a?(Hash)

      config.to_h
    end
  end
end
