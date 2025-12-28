```elixir title="Elixir"
# Preprocess images before OCR for improved text extraction
# Image preprocessing can enhance OCR accuracy on poor quality scans

alias Kreuzberg.ExtractionConfig

defmodule ImagePreprocessor do
  @moduledoc """
  Preprocess images for optimal OCR results.
  Provides utilities for image enhancement before text extraction.
  """

  @doc """
  Extract with image preprocessing enabled.

  Preprocessing improves OCR accuracy on documents with:
  - Low contrast text
  - Skewed pages
  - Poor image quality
  - Faded or faint text
  """
  def extract_with_preprocessing(file_path, opts \\ []) do
    # Get preprocessing options
    deskew = Keyword.get(opts, :deskew, true)
    denoise = Keyword.get(opts, :denoise, true)
    binarize = Keyword.get(opts, :binarize, false)
    brightness_threshold = Keyword.get(opts, :brightness_threshold, 50)

    config = %ExtractionConfig{
      ocr: %{
        "enabled" => true,
        "backend" => "tesseract",
        "language" => "eng",
        # Enable preprocessing for better OCR results
        "preprocessing" => %{
          "enabled" => true,
          "deskew" => deskew,
          "denoise" => denoise,
          "binarize" => binarize,
          "brightness_threshold" => brightness_threshold
        }
      },
      chunking: %{
        "enabled" => true,
        "max_chars" => 1500,
        "max_overlap" => 150
      },
      language_detection: %{
        "enabled" => true
      },
      use_cache: false  # Disable caching for preprocessing runs
    }

    Kreuzberg.extract_file(file_path, nil, config)
  end

  @doc """
  Extract with aggressive preprocessing for poor quality documents.

  Use this for heavily degraded or difficult-to-read documents.
  """
  def extract_with_aggressive_preprocessing(file_path) do
    extract_with_preprocessing(file_path,
      deskew: true,
      denoise: true,
      binarize: true,
      brightness_threshold: 75
    )
  end

  @doc """
  Compare extraction quality with and without preprocessing.

  Useful for determining optimal preprocessing settings.
  """
  def compare_preprocessing_quality(file_path) do
    IO.puts("Comparing preprocessing options...\n")

    # Extract without preprocessing
    IO.puts("Extracting without preprocessing...")
    config_standard = %ExtractionConfig{
      ocr: %{
        "enabled" => true,
        "backend" => "tesseract",
        "language" => "eng",
        "preprocessing" => %{"enabled" => false}
      },
      use_cache: false
    }

    {:ok, result_standard} = Kreuzberg.extract_file(file_path, nil, config_standard)

    # Extract with preprocessing
    IO.puts("Extracting with preprocessing...")
    {:ok, result_preprocessed} = extract_with_aggressive_preprocessing(file_path)

    # Compare results
    standard_size = byte_size(result_standard.content)
    preprocessed_size = byte_size(result_preprocessed.content)

    IO.puts("\n=== Preprocessing Comparison ===")
    IO.puts("Standard extraction: #{standard_size} bytes")
    IO.puts("Preprocessed extraction: #{preprocessed_size} bytes")
    IO.puts("Size difference: #{abs(preprocessed_size - standard_size)} bytes")

    # Compare chunk quality
    standard_chunks = result_standard.chunks || []
    preprocessed_chunks = result_preprocessed.chunks || []
    IO.puts("\nStandard chunks: #{length(standard_chunks)}")
    IO.puts("Preprocessed chunks: #{length(preprocessed_chunks)}")

    # Show content comparison
    IO.puts("\n=== Content Comparison ===")
    IO.puts("Standard preview:")
    IO.puts(String.slice(result_standard.content, 0..199))
    IO.puts("\n...")

    IO.puts("\nPreprocessed preview:")
    IO.puts(String.slice(result_preprocessed.content, 0..199))
    IO.puts("\n...")

    # Return comparison data
    %{
      standard_content: result_standard.content,
      preprocessed_content: result_preprocessed.content,
      standard_size: standard_size,
      preprocessed_size: preprocessed_size,
      improvement: if standard_size > 0 do
        Float.round((preprocessed_size - standard_size) / standard_size * 100, 2)
      else
        0
      end
    }
  end
end

# Usage examples

# Example 1: Standard preprocessing
file_path = "scanned_document.pdf"

IO.puts("Example 1: Standard Extraction with Preprocessing\n")
case ImagePreprocessor.extract_with_preprocessing(file_path) do
  {:ok, result} ->
    IO.puts("Extraction successful!")
    IO.puts("Content length: #{byte_size(result.content)} bytes")
    IO.puts("Chunks created: #{length(result.chunks || [])}")
    IO.puts("Preview: #{String.slice(result.content, 0..100)}...\n")

  {:error, reason} ->
    IO.puts("Error: #{reason}\n")
end

# Example 2: Aggressive preprocessing for difficult documents
IO.puts("Example 2: Aggressive Preprocessing for Poor Quality\n")
case ImagePreprocessor.extract_with_aggressive_preprocessing(file_path) do
  {:ok, result} ->
    IO.puts("Aggressive preprocessing extraction successful!")
    IO.puts("Content length: #{byte_size(result.content)} bytes")

  {:error, reason} ->
    IO.puts("Error: #{reason}\n")
end

# Example 3: Compare preprocessing options
IO.puts("Example 3: Compare Preprocessing Quality\n")
try do
  comparison = ImagePreprocessor.compare_preprocessing_quality(file_path)
  IO.puts("\nImprovement with preprocessing: #{comparison.improvement}%")
rescue
  error ->
    IO.puts("Comparison completed with notice: #{inspect(error)}")
end
```
