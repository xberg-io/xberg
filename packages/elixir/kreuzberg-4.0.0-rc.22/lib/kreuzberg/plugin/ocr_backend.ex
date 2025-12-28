defmodule Kreuzberg.Plugin.OcrBackend do
  @moduledoc """
  Behaviour module for OCR backends in the Kreuzberg plugin system.

  This module defines the interface that all OCR backend implementations must follow
  to integrate with the Kreuzberg document extraction library. OCR backends are responsible
  for extracting text from images using optical character recognition.

  ## Overview

  An OCR backend is a module that implements this behaviour and provides the ability to:

    * Initialize and shutdown the OCR engine
    * Detect supported languages
    * Process image binary data and extract text
    * Process image files and extract text
    * Report version information

  ## Implementing an OCR Backend

  To create a custom OCR backend, create a module that implements this behaviour:

      defmodule MyApp.CustomOcrBackend do
        @behaviour Kreuzberg.Plugin.OcrBackend

        @impl true
        def name(), do: "custom_ocr"

        @impl true
        def supported_languages(), do: ["eng", "deu", "fra"]

        @impl true
        def initialize() do
          # Initialize your OCR engine
          :ok
        end

        @impl true
        def shutdown() do
          # Clean up resources
          :ok
        end

        @impl true
        def process_image(image_data, language) when is_binary(image_data) do
          # Extract text from image binary data
          {:ok, extracted_text}
        end

        @impl true
        def process_file(path, language) when is_binary(path) do
          # Extract text from image file
          {:ok, extracted_text}
        end

        @impl true
        def version(), do: "1.0.0"
      end

  ## Language Codes

  Language codes should follow the ISO 639-3 standard for consistency:

    * `"eng"` - English
    * `"deu"` - German
    * `"fra"` - French
    * `"spa"` - Spanish
    * `"ita"` - Italian
    * `"jpn"` - Japanese
    * `"rus"` - Russian
    * `"chi"` - Chinese (Simplified)
    * `"chi_tra"` - Chinese (Traditional)

  ## Error Handling

  All functions that can fail should return `{:error, reason}` tuples where `reason` is
  a descriptive string. Use clear, actionable error messages to help users debug issues.

  Common error scenarios:

    * Invalid image format: `{:error, "Unsupported image format: png"}`
    * Unsupported language: `{:error, "Language not supported: xyz"}`
    * Engine initialization failure: `{:error, "Failed to initialize OCR engine"}`
    * File read errors: `{:error, "Cannot read file: /path/to/file"}`
    * OCR processing errors: `{:error, "OCR processing failed: timeout"}`

  ## Example: Tesseract-based Backend

  Here's a complete example of implementing a Tesseract-based OCR backend:

  - `name()` - Returns "tesseract"
  - `supported_languages()` - Returns list of supported language codes
  - `initialize()` - Verifies tesseract is installed via `System.cmd("tesseract", ["--version"])`
  - `shutdown()` - Performs cleanup (Tesseract requires no explicit shutdown)
  - `process_image(image_data, language)` - Writes image to temp file, runs tesseract with language param,
    reads output text file, cleans up temp files
  - `process_file(path, language)` - Reads image file and delegates to `process_image/2`
  - `version()` - Runs `tesseract --version` and extracts the version string

  Error handling covers:
  - Missing tesseract installation
  - File I/O errors during temporary file operations
  - OCR processing failures

  ## Integration with Kreuzberg

  Once implemented, OCR backends can be integrated into the Kreuzberg extraction pipeline
  through the configuration system:

      config = %Kreuzberg.ExtractionConfig{
        ocr: %{
          "enabled" => true,
          "backend" => "tesseract",
          "languages" => ["eng", "deu"]
        }
      }

      {:ok, result} = Kreuzberg.extract(pdf_binary, "application/pdf", config)

  ## Callbacks

  """

  @doc """
  Returns the human-readable name of the OCR backend.

  Used for identification and logging purposes. Should be a lowercase atom-compatible string.

  ## Returns

  A string identifier for the backend (e.g., "tesseract", "paddleocr", "custom_ocr").

  ## Examples

      iex> Kreuzberg.Plugin.ExampleBackend.name()
      "example_ocr"
  """
  @callback name() :: String.t()

  @doc """
  Returns a list of supported language codes.

  Languages should be reported using ISO 639-3 codes (3-letter codes) where possible.
  If specific variants are supported (e.g., Traditional vs Simplified Chinese),
  use a suffix with underscore (e.g., "chi_tra").

  ## Returns

  A list of language codes the backend can process.

  ## Examples

      iex> Kreuzberg.Plugin.ExampleBackend.supported_languages()
      ["eng", "deu", "fra", "spa"]
  """
  @callback supported_languages() :: [String.t()]

  @doc """
  Processes image binary data and extracts text.

  Takes raw image data (typically PNG, JPG, TIFF, etc.) and a language code,
  and returns the extracted text content or an error.

  ## Parameters

    * `image_data` - Binary image data (raw bytes)
    * `language` - ISO 639-3 language code (e.g., "eng", "deu")

  ## Returns

    * `{:ok, text}` - Extracted text as a string
    * `{:error, reason}` - Processing failed with error reason

  ## Examples

      iex> image_binary = File.read!("image.png")
      iex> Kreuzberg.Plugin.ExampleBackend.process_image(image_binary, "eng")
      {:ok, "Extracted text from the image"}

      iex> Kreuzberg.Plugin.ExampleBackend.process_image(<<>>, "unknown_lang")
      {:error, "Invalid image data"}
  """
  @callback process_image(image_data :: binary(), language :: String.t()) ::
              {:ok, String.t()} | {:error, String.t()}

  @doc """
  Processes an image file and extracts text.

  Reads an image from the specified file path and extracts text content.
  This is a convenience callback that can be implemented by reading the file
  and delegating to `process_image/2`.

  ## Parameters

    * `path` - File system path to the image
    * `language` - ISO 639-3 language code (e.g., "eng", "deu")

  ## Returns

    * `{:ok, text}` - Extracted text as a string
    * `{:error, reason}` - Processing failed with error reason

  ## Examples

      iex> Kreuzberg.Plugin.ExampleBackend.process_file("/tmp/document.png", "eng")
      {:ok, "Extracted text from the file"}

      iex> Kreuzberg.Plugin.ExampleBackend.process_file("/nonexistent/file.png", "eng")
      {:error, "Cannot read file: ..."}
  """
  @callback process_file(path :: String.t(), language :: String.t()) ::
              {:ok, String.t()} | {:error, String.t()}

  @doc """
  Initializes the OCR backend.

  Called once when the backend is first loaded. Should prepare any resources,
  verify dependencies, and perform any necessary setup. If initialization fails,
  the backend should not be used.

  ## Returns

    * `:ok` - Initialization successful
    * `{:error, reason}` - Initialization failed

  ## Examples

      iex> Kreuzberg.Plugin.ExampleBackend.initialize()
      :ok

      iex> # If dependencies are missing
      {:error, "Tesseract not found"}
  """
  @callback initialize() :: :ok | {:error, String.t()}

  @doc """
  Shuts down the OCR backend.

  Called when the backend is being unloaded. Should clean up resources,
  close connections, and perform any necessary teardown. Should not raise exceptions.

  This is a best-effort cleanup function. Errors in shutdown should be logged
  but should not prevent the shutdown process.

  ## Returns

    * `:ok` - Shutdown completed (errors may have been logged internally)

  ## Examples

      iex> Kreuzberg.Plugin.ExampleBackend.shutdown()
      :ok
  """
  @callback shutdown() :: :ok

  @doc """
  Returns the version of the OCR backend.

  Should return a semantic version string (e.g., "1.0.0") or a version
  string from the underlying OCR engine if available.

  ## Returns

  A version string for the backend.

  ## Examples

      iex> Kreuzberg.Plugin.ExampleBackend.version()
      "1.2.3"

      iex> Kreuzberg.Plugin.ExampleBackend.version()
      "tesseract 4.1.1"
  """
  @callback version() :: String.t()
end
