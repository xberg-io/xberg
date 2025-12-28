defmodule Kreuzberg.Validators do
  @moduledoc """
  Configuration validators for Kreuzberg extraction options.

  This module provides validation functions for various configuration parameters
  used in document extraction. Each validator returns either `:ok` for valid input
  or `{:error, reason}` for invalid input.

  All validators delegate to corresponding Rust NIF implementations for consistent
  validation logic across language bindings.

  ## Validator Functions

  - `validate_chunking_params/1` - Validate chunking configuration parameters
  - `validate_language_code/1` - Validate ISO 639 language codes
  - `validate_dpi/1` - Validate DPI (dots per inch) values
  - `validate_confidence/1` - Validate confidence threshold values (0.0-1.0)
  - `validate_ocr_backend/1` - Validate OCR backend names
  - `validate_binarization_method/1` - Validate image binarization methods
  - `validate_tesseract_psm/1` - Validate Tesseract Page Segmentation Mode values
  - `validate_tesseract_oem/1` - Validate Tesseract OCR Engine Mode values

  ## Examples

      iex> Kreuzberg.Validators.validate_language_code("en")
      :ok

      iex> Kreuzberg.Validators.validate_language_code("invalid")
      {:error, "Invalid language code 'invalid'. Use ISO 639-1 (2-letter, e.g., 'en', 'de') or ISO 639-3 (3-letter, e.g., 'eng', 'deu') codes. Common codes: en, de, fr, es, it, pt, nl, pl, ru, zh, ja, ko, ar, hi, th."}

      iex> Kreuzberg.Validators.validate_dpi(300)
      :ok

      iex> Kreuzberg.Validators.validate_dpi(0)
      {:error, "Invalid DPI value '0'. Must be a positive integer, typically 72-600."}

      iex> Kreuzberg.Validators.validate_confidence(0.5)
      :ok

      iex> Kreuzberg.Validators.validate_confidence(1.5)
      {:error, "Invalid confidence threshold '1.5'. Must be between 0.0 and 1.0."}

      iex> Kreuzberg.Validators.validate_ocr_backend("tesseract")
      :ok

      iex> Kreuzberg.Validators.validate_ocr_backend("invalid_backend")
      {:error, "Invalid OCR backend 'invalid_backend'. Valid options are: tesseract, easyocr, paddleocr"}

      iex> Kreuzberg.Validators.validate_binarization_method("otsu")
      :ok

      iex> Kreuzberg.Validators.validate_binarization_method("invalid")
      {:error, "Invalid binarization method 'invalid'. Valid options are: otsu, adaptive, sauvola"}

      iex> Kreuzberg.Validators.validate_tesseract_psm(6)
      :ok

      iex> Kreuzberg.Validators.validate_tesseract_psm(14)
      {:error, "Invalid tesseract PSM value '14'. Valid range is 0-13. Common values: 3 (auto), 6 (single block), 11 (sparse text)."}

      iex> Kreuzberg.Validators.validate_tesseract_oem(1)
      :ok

      iex> Kreuzberg.Validators.validate_tesseract_oem(4)
      {:error, "Invalid tesseract OEM value '4'. Valid range is 0-3. 0=Legacy, 1=LSTM, 2=Legacy+LSTM, 3=Default"}

      iex> Kreuzberg.Validators.validate_chunking_params(%{"max_chars" => 1000, "max_overlap" => 200})
      :ok

      iex> Kreuzberg.Validators.validate_chunking_params(%{"max_chars" => 100, "max_overlap" => 150})
      {:error, "max_overlap (150) must be less than max_chars (100)"}
  """

  alias Kreuzberg.{Native, Helpers}

  @doc """
  Validate chunking configuration parameters.

  Validates that chunking parameters are valid:
  - `max_chars` must be greater than 0
  - `max_overlap` must be less than `max_chars`

  ## Parameters

    * `params` - A map with keys:
      - `"max_chars"` or `:max_chars` - Maximum characters per chunk (required)
      - `"max_overlap"` or `:max_overlap` - Overlap between chunks (required)

  ## Returns

    * `:ok` - If parameters are valid
    * `{:error, reason}` - If parameters are invalid

  ## Examples

      iex> Kreuzberg.Validators.validate_chunking_params(%{"max_chars" => 1000, "max_overlap" => 200})
      :ok

      iex> Kreuzberg.Validators.validate_chunking_params(%{max_chars: 1000, max_overlap: 200})
      :ok

      iex> Kreuzberg.Validators.validate_chunking_params(%{"max_chars" => 0, "max_overlap" => 100})
      {:error, "max_chars must be greater than 0"}

      iex> Kreuzberg.Validators.validate_chunking_params(%{"max_chars" => 100, "max_overlap" => 150})
      {:error, "max_overlap (150) must be less than max_chars (100)"}
  """
  @spec validate_chunking_params(map()) :: :ok | {:error, String.t()}
  def validate_chunking_params(params) when is_map(params) do
    # Normalize keys to strings for consistent handling
    normalized = Helpers.normalize_map_keys(params)

    with {:ok, max_chars} <- fetch_positive_integer(normalized, "max_chars"),
         {:ok, max_overlap} <- fetch_non_negative_integer(normalized, "max_overlap") do
      Native.validate_chunking_params(max_chars, max_overlap)
    end
  end

  @doc """
  Validate an ISO 639 language code.

  Accepts both 2-letter ISO 639-1 codes (e.g., "en", "de") and
  3-letter ISO 639-3 codes (e.g., "eng", "deu").

  ## Parameters

    * `code` - A language code string (e.g., "en", "eng", "de", "deu")

  ## Returns

    * `:ok` - If the language code is valid
    * `{:error, reason}` - If the language code is invalid

  ## Valid Language Codes

  Supports major languages including:
  - ISO 639-1 (2-letter): en, de, fr, es, it, pt, nl, pl, ru, zh, ja, ko, ar, hi, th, and more
  - ISO 639-3 (3-letter): eng, deu, fra, spa, ita, por, nld, pol, rus, zho, jpn, kor, and more

  ## Examples

      iex> Kreuzberg.Validators.validate_language_code("en")
      :ok

      iex> Kreuzberg.Validators.validate_language_code("eng")
      :ok

      iex> Kreuzberg.Validators.validate_language_code("de")
      :ok

      iex> Kreuzberg.Validators.validate_language_code("invalid")
      {:error, _}
  """
  @spec validate_language_code(String.t()) :: :ok | {:error, String.t()}
  def validate_language_code(code) when is_binary(code) do
    Native.validate_language_code(code)
  end

  @doc """
  Validate a DPI (dots per inch) value.

  DPI should be a positive integer, typically in the range 72-600.
  The maximum allowed DPI is 2400.

  ## Parameters

    * `dpi` - A positive integer representing DPI

  ## Returns

    * `:ok` - If the DPI value is valid
    * `{:error, reason}` - If the DPI value is invalid

  ## Valid Range

    * Minimum: 1
    * Maximum: 2400
    * Typical values: 72, 96, 150, 300, 600

  ## Examples

      iex> Kreuzberg.Validators.validate_dpi(96)
      :ok

      iex> Kreuzberg.Validators.validate_dpi(300)
      :ok

      iex> Kreuzberg.Validators.validate_dpi(0)
      {:error, _}

      iex> Kreuzberg.Validators.validate_dpi(-1)
      {:error, _}
  """
  @spec validate_dpi(integer()) :: :ok | {:error, String.t()}
  def validate_dpi(dpi) when is_integer(dpi) do
    Native.validate_dpi(dpi)
  end

  @doc """
  Validate a confidence threshold value.

  Confidence thresholds must be between 0.0 and 1.0 inclusive.

  ## Parameters

    * `confidence` - A float representing a confidence threshold

  ## Returns

    * `:ok` - If the confidence value is valid
    * `{:error, reason}` - If the confidence value is invalid

  ## Valid Range

    * Minimum: 0.0
    * Maximum: 1.0

  ## Examples

      iex> Kreuzberg.Validators.validate_confidence(0.5)
      :ok

      iex> Kreuzberg.Validators.validate_confidence(0.0)
      :ok

      iex> Kreuzberg.Validators.validate_confidence(1.0)
      :ok

      iex> Kreuzberg.Validators.validate_confidence(-0.1)
      {:error, _}

      iex> Kreuzberg.Validators.validate_confidence(1.5)
      {:error, _}
  """
  @spec validate_confidence(float()) :: :ok | {:error, String.t()}
  def validate_confidence(confidence) when is_float(confidence) do
    Native.validate_confidence(confidence)
  end

  # Allow integers to be coerced to floats for convenience
  def validate_confidence(confidence) when is_integer(confidence) do
    Native.validate_confidence(confidence / 1.0)
  end

  @doc """
  Validate an OCR backend name.

  OCR backend must be one of the supported backends: tesseract, easyocr, or paddleocr.

  ## Parameters

    * `backend` - A string representing the OCR backend name

  ## Returns

    * `:ok` - If the backend name is valid
    * `{:error, reason}` - If the backend name is invalid

  ## Valid Backends

    * "tesseract" - Tesseract OCR engine
    * "easyocr" - EasyOCR engine
    * "paddleocr" - PaddleOCR engine

  ## Examples

      iex> Kreuzberg.Validators.validate_ocr_backend("tesseract")
      :ok

      iex> Kreuzberg.Validators.validate_ocr_backend("easyocr")
      :ok

      iex> Kreuzberg.Validators.validate_ocr_backend("paddleocr")
      :ok

      iex> Kreuzberg.Validators.validate_ocr_backend("invalid_backend")
      {:error, _}
  """
  @spec validate_ocr_backend(String.t()) :: :ok | {:error, String.t()}
  def validate_ocr_backend(backend) when is_binary(backend) do
    Native.validate_ocr_backend(backend)
  end

  @doc """
  Validate an image binarization method.

  Binarization method must be one of the supported methods: otsu, adaptive, or sauvola.

  ## Parameters

    * `method` - A string representing the binarization method

  ## Returns

    * `:ok` - If the binarization method is valid
    * `{:error, reason}` - If the binarization method is invalid

  ## Valid Methods

    * "otsu" - Otsu's method for automatic threshold selection
    * "adaptive" - Adaptive binarization based on local statistics
    * "sauvola" - Sauvola's method for document image binarization

  ## Examples

      iex> Kreuzberg.Validators.validate_binarization_method("otsu")
      :ok

      iex> Kreuzberg.Validators.validate_binarization_method("adaptive")
      :ok

      iex> Kreuzberg.Validators.validate_binarization_method("sauvola")
      :ok

      iex> Kreuzberg.Validators.validate_binarization_method("invalid")
      {:error, _}
  """
  @spec validate_binarization_method(String.t()) :: :ok | {:error, String.t()}
  def validate_binarization_method(method) when is_binary(method) do
    Native.validate_binarization_method(method)
  end

  @doc """
  Validate a Tesseract Page Segmentation Mode (PSM) value.

  PSM values range from 0 to 13 and control how Tesseract segments the page.

  ## Parameters

    * `psm` - An integer representing the PSM mode (0-13)

  ## Returns

    * `:ok` - If the PSM value is valid
    * `{:error, reason}` - If the PSM value is invalid

  ## Valid PSM Values

    * 0 - Orientation and script detection only
    * 1 - Automatic page segmentation with OSD
    * 2 - Automatic page segmentation, but no OSD, or OCR
    * 3 - Fully automatic page segmentation, but no OSD (default)
    * 4 - Assume a single column of text of variable sizes
    * 5 - Assume a single uniform block of vertically aligned text
    * 6 - Assume a single uniform block of text (most common)
    * 7 - Treat the image as a single text line
    * 8 - Treat the image as a single word
    * 9 - Treat the image as a single word in a circle
    * 10 - Treat the image as a single character
    * 11 - Sparse text; find as much text as possible in no particular order
    * 12 - Sparse text with OSD
    * 13 - Raw line: treat the image as a single text line, bypassing hacks that are Tesseract-specific

  ## Examples

      iex> Kreuzberg.Validators.validate_tesseract_psm(3)
      :ok

      iex> Kreuzberg.Validators.validate_tesseract_psm(6)
      :ok

      iex> Kreuzberg.Validators.validate_tesseract_psm(14)
      {:error, _}

      iex> Kreuzberg.Validators.validate_tesseract_psm(-1)
      {:error, _}
  """
  @spec validate_tesseract_psm(integer()) :: :ok | {:error, String.t()}
  def validate_tesseract_psm(psm) when is_integer(psm) do
    Native.validate_tesseract_psm(psm)
  end

  @doc """
  Validate a Tesseract OCR Engine Mode (OEM) value.

  OEM values range from 0 to 3 and control which OCR engine Tesseract uses.

  ## Parameters

    * `oem` - An integer representing the OEM mode (0-3)

  ## Returns

    * `:ok` - If the OEM value is valid
    * `{:error, reason}` - If the OEM value is invalid

  ## Valid OEM Values

    * 0 - Legacy engine only
    * 1 - Neural nets LSTM engine only
    * 2 - Legacy + LSTM engines (best accuracy)
    * 3 - Default (use whatever is available)

  ## Examples

      iex> Kreuzberg.Validators.validate_tesseract_oem(0)
      :ok

      iex> Kreuzberg.Validators.validate_tesseract_oem(1)
      :ok

      iex> Kreuzberg.Validators.validate_tesseract_oem(4)
      {:error, _}

      iex> Kreuzberg.Validators.validate_tesseract_oem(-1)
      {:error, _}
  """
  @spec validate_tesseract_oem(integer()) :: :ok | {:error, String.t()}
  def validate_tesseract_oem(oem) when is_integer(oem) do
    Native.validate_tesseract_oem(oem)
  end

  # Private helper functions

  @doc false
  defp fetch_positive_integer(map, key) do
    case Map.fetch(map, key) do
      {:ok, value} when is_integer(value) and value > 0 ->
        {:ok, value}

      {:ok, value} when is_integer(value) ->
        {:error, "#{key} must be greater than 0"}

      {:ok, _value} ->
        {:error, "#{key} must be an integer"}

      :error ->
        {:error, "Missing required parameter: #{key}"}
    end
  end

  @doc false
  defp fetch_non_negative_integer(map, key) do
    case Map.fetch(map, key) do
      {:ok, value} when is_integer(value) and value >= 0 ->
        {:ok, value}

      {:ok, value} when is_integer(value) ->
        {:error, "#{key} must be non-negative"}

      {:ok, _value} ->
        {:error, "#{key} must be an integer"}

      :error ->
        {:error, "Missing required parameter: #{key}"}
    end
  end
end
