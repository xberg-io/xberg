defmodule Kreuzberg.Test.ExampleOcrBackend do
  @moduledoc """
  Example OCR backend plugin for testing.

  This OCR backend demonstrates how to implement a custom OCR backend that
  returns mock OCR text for testing purposes. It supports three languages
  and provides realistic mock responses based on language.

  ## Supported Languages

  - "eng" - English
  - "deu" - German (Deutsch)
  - "fra" - French

  ## Behavior

  - Returns mock OCR text when processing images
  - Supports both binary image data and file paths
  - Provides language-specific mock text
  - Validates language support before processing

  ## Example

      # Process image binary
      {:ok, text} = Kreuzberg.Test.ExampleOcrBackend.process_image(<<>>, "eng")
      # Returns: "Sample OCR extracted text in English"

      # Process image file
      {:ok, text} = Kreuzberg.Test.ExampleOcrBackend.process_file("/tmp/test.png", "deu")
      # Returns: "Beispieltext von der OCR-Extraktion auf Deutsch"

      # Unsupported language
      {:error, reason} = Kreuzberg.Test.ExampleOcrBackend.process_image(<<>>, "unk")
      # Returns: {:error, "Language not supported: unk"}
  """

  @behaviour Kreuzberg.Plugin.OcrBackend

  @impl true
  def name do
    "test_ocr"
  end

  @impl true
  def version do
    "1.0.0"
  end

  @impl true
  def supported_languages do
    ["eng", "deu", "fra"]
  end

  @impl true
  def initialize do
    # No special initialization needed for mock backend
    :ok
  end

  @impl true
  def shutdown do
    # No cleanup needed for mock backend
    :ok
  end

  @impl true
  def process_image(image_data, language) when is_binary(image_data) do
    # Validate image data is not empty
    if byte_size(image_data) == 0 do
      {:error, "Invalid image data: empty binary"}
    else
      # Validate language is supported
      process_with_language(language)
    end
  end

  @impl true
  def process_file(path, language) when is_binary(path) do
    # Validate path is a string
    case File.read(path) do
      {:ok, image_data} ->
        # File read successfully, process as image
        process_image(image_data, language)

      {:error, _reason} ->
        {:error, "Cannot read file: #{path}"}
    end
  end

  # Private helpers

  defp process_with_language(language) do
    case language do
      "eng" ->
        {:ok, mock_text_english()}

      "deu" ->
        {:ok, mock_text_german()}

      "fra" ->
        {:ok, mock_text_french()}

      lang ->
        {:error, "Language not supported: #{lang}"}
    end
  end

  defp mock_text_english do
    """
    Sample OCR extracted text in English.

    This is a mock OCR result demonstrating the test_ocr backend.
    It can extract text from images in multiple languages.

    The quick brown fox jumps over the lazy dog.
    """
    |> String.trim()
  end

  defp mock_text_german do
    """
    Beispieltext von der OCR-Extraktion auf Deutsch.

    Dies ist ein Mock-OCR-Ergebnis, das das test_ocr Backend demonstriert.
    Es kann Text aus Bildern in mehreren Sprachen extrahieren.

    Der schnelle braune Fuchs springt über den faulen Hund.
    """
    |> String.trim()
  end

  defp mock_text_french do
    """
    Exemple de texte extrait par OCR en français.

    Ceci est un résultat OCR simulé démontrant le backend test_ocr.
    Il peut extraire du texte d'images dans plusieurs langues.

    Le rapide renard brun saute par-dessus le chien paresseux.
    """
    |> String.trim()
  end
end
