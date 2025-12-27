# frozen_string_literal: true

require 'json'

module Kreuzberg
  module ErrorContext
    class << self
      # @return [Integer] Error code constant (ERROR_CODE_* values), or 0 on success
      # @example Check last error
      def last_error_code
        Kreuzberg._last_error_code_native
      rescue StandardError
        0
      end

      # Get panic context information from the last error.
      #
      # Returns a {Errors::PanicContext} object containing detailed information about
      # the last panic that occurred in the Rust core. Includes file path, line number,
      # function name, error message, and timestamp.
      #
      # @return [Errors::PanicContext, nil] Panic context if a panic occurred, nil otherwise
      #
      # @example Get panic details
      #   panic = Kreuzberg::ErrorContext.last_panic_context
      #   if panic
      #     puts "Panic at #{panic.file}:#{panic.line} in #{panic.function}"
      #     puts "Message: #{panic.message}"
      #     puts "Time: #{panic.timestamp_secs}"
      #   end
      def last_panic_context
        json_str = Kreuzberg._last_panic_context_json_native
        return nil unless json_str

        Errors::PanicContext.from_json(json_str)
      rescue StandardError
        nil
      end

      # Get panic context as raw JSON string.
      #
      # Returns the panic context information as a JSON string for raw access or
      # custom parsing. Returns nil if no panic has occurred.
      #
      # @return [String, nil] JSON-serialized panic context, or nil if no panic
      #
      # @example Get raw JSON panic context
      #   json = Kreuzberg::ErrorContext.last_panic_context_json
      #   if json
      #     panic_data = JSON.parse(json)
      #     puts panic_data
      #   end
      def last_panic_context_json
        Kreuzberg._last_panic_context_json_native
      rescue StandardError
        nil
      end

      # Get detailed error information from the last operation.
      #
      # Returns comprehensive error details including message, code, type, source location,
      # and panic information.
      #
      # @return [Hash] Hash with keys: :message, :error_code, :error_type, :source_file,
      #   :source_function, :source_line, :context_info, :is_panic
      #
      # @example Get error details
      #   details = Kreuzberg::ErrorContext.error_details
      #   puts "Error: #{details[:message]}"
      #   puts "Code: #{details[:error_code]}"
      #   puts "Type: #{details[:error_type]}"
      def error_details
        Kreuzberg._get_error_details_native
      rescue StandardError
        {}
      end

      # Classify an error message into a Kreuzberg error code.
      #
      # Analyzes an error message and returns the most likely error code (0-7).
      # Useful for converting third-party error messages into Kreuzberg categories.
      #
      # @param message [String] The error message to classify
      # @return [Integer] Error code (0-7)
      #
      # Error code mapping:
      # - 0: Validation
      # - 1: Parsing
      # - 2: OCR
      # - 3: MissingDependency
      # - 4: IO
      # - 5: Plugin
      # - 6: UnsupportedFormat
      # - 7: Internal
      #
      # @example Classify an error
      #   code = Kreuzberg::ErrorContext.classify_error("File not found")
      #   if code == 4
      #     puts "This is an I/O error"
      #   end
      def classify_error(message)
        Kreuzberg._classify_error_native(message)
      rescue StandardError
        7
      end

      # Get the human-readable name of an error code.
      #
      # @param code [Integer] Numeric error code (0-7)
      # @return [String] Human-readable error code name (e.g., "validation", "io")
      #
      # @example Get error code name
      #   name = Kreuzberg::ErrorContext.error_code_name(0)
      #   puts name  # => "validation"
      def error_code_name(code)
        Kreuzberg._error_code_name_native(code)
      rescue StandardError
        'unknown'
      end

      # Get the description of an error code.
      #
      # @param code [Integer] Numeric error code (0-7)
      # @return [String] Description of the error code
      #
      # @example Get error code description
      #   desc = Kreuzberg::ErrorContext.error_code_description(0)
      #   puts desc  # => "Input validation error"
      def error_code_description(code)
        Kreuzberg._error_code_description_native(code)
      rescue StandardError
        'Unknown error code'
      end
    end
  end
end
