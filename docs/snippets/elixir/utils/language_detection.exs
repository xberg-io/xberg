# Language detection and processing utilities
defmodule LanguageDetectionUtils do
  @default_confidence 0.8

  @language_indicators %{
    "en" => %{
      patterns: ~w[the and to of a in is that],
      common_words: ~w[english language detection],
      min_match: 2
    },
    "es" => %{
      patterns: ~w[de la el y en que],
      common_words: ~w[español detección idioma],
      min_match: 2
    },
    "fr" => %{
      patterns: ~w[le la de et une est],
      common_words: ~w[français détection langue],
      min_match: 2
    },
    "de" => %{
      patterns: ~w[der die und das in ein],
      common_words: ~w[deutsch erkennung sprache],
      min_match: 2
    }
  }

  @doc """
  Detect the language of extracted text using pattern matching.
  Returns {language_code, confidence} tuple.
  """
  def detect_language(text) when is_binary(text) and byte_size(text) > 0 do
    normalized = String.downcase(text)
    words = String.split(normalized, ~r/\W+/)

    scores =
      @language_indicators
      |> Enum.map(fn {lang, indicators} ->
        matched = count_pattern_matches(words, indicators.patterns)
        confidence = min(matched / indicators.min_match, 1.0)
        {lang, confidence}
      end)
      |> Enum.sort_by(&elem(&1, 1), :desc)

    case scores do
      [{lang, confidence} | _] when confidence > 0.3 ->
        {lang, min(confidence * 100, 100)}

      _ ->
        {"unknown", 0.0}
    end
  end

  def detect_language(_), do: {"unknown", 0.0}

  @doc """
  Detect language for each chunk and add it to metadata.
  """
  def detect_chunk_languages(chunks) when is_list(chunks) do
    Enum.map(chunks, fn chunk ->
      text = Map.get(chunk, "text", "")
      {lang, confidence} = detect_language(text)

      metadata = Map.get(chunk, "metadata", %{})

      updated_metadata =
        metadata
        |> Map.put("detected_language", lang)
        |> Map.put("language_confidence", confidence)

      Map.put(chunk, "metadata", updated_metadata)
    end)
  end

  @doc """
  Group chunks by detected language.
  """
  def group_chunks_by_language(chunks) do
    chunks
    |> detect_chunk_languages()
    |> Enum.group_by(fn chunk ->
      chunk
      |> Map.get("metadata", %{})
      |> Map.get("detected_language", "unknown")
    end)
  end

  @doc """
  Filter chunks by language with optional confidence threshold.
  """
  def filter_by_language(chunks, target_language, min_confidence \\ 0.7) do
    chunks
    |> detect_chunk_languages()
    |> Enum.filter(fn chunk ->
      metadata = Map.get(chunk, "metadata", %{})
      detected = Map.get(metadata, "detected_language", "unknown")
      confidence = Map.get(metadata, "language_confidence", 0.0) / 100

      detected == target_language and confidence >= min_confidence
    end)
  end

  @doc """
  Summarize language distribution across chunks.
  """
  def language_summary(chunks) do
    chunks
    |> group_chunks_by_language()
    |> Enum.map(fn {lang, group} ->
      avg_confidence =
        group
        |> Enum.map(fn chunk ->
          chunk
          |> Map.get("metadata", %{})
          |> Map.get("language_confidence", 0.0)
        end)
        |> then(fn confidences ->
          if Enum.empty?(confidences) do
            0.0
          else
            Enum.sum(confidences) / length(confidences)
          end
        end)

      %{
        language: lang,
        chunk_count: length(group),
        avg_confidence: Float.round(avg_confidence, 2)
      }
    end)
    |> Enum.sort_by(&Map.get(&1, :chunk_count), :desc)
  end

  @doc """
  Determine if text is mostly in a single language.
  """
  def is_single_language?(chunks, threshold \\ 0.8) do
    case language_summary(chunks) do
      [top | rest] ->
        dominant_ratio = top.chunk_count / Enum.reduce(rest, top.chunk_count, fn x, acc ->
          acc + x.chunk_count
        end)

        dominant_ratio >= threshold

      _ ->
        false
    end
  end

  # Private helpers
  defp count_pattern_matches(words, patterns) do
    Enum.count(words, &Enum.member?(patterns, &1))
  end
end

# Example usage
config = %Kreuzberg.ExtractionConfig{
  chunking: %{"enabled" => true, "max_chars" => 1000}
}

{:ok, result} = Kreuzberg.extract_file("multilingual_doc.pdf", nil, config)
chunks = result.chunks || []

IO.puts("=== Language Detection ===")

case LanguageDetectionUtils.detect_language(result.content || "") do
  {lang, confidence} ->
    IO.puts("Detected Language: #{lang}")
    IO.puts("Confidence: #{Float.round(confidence, 2)}%")
end

IO.puts("\n=== Language Summary ===")
IO.inspect(LanguageDetectionUtils.language_summary(chunks))

IO.puts("\n=== Group by Language ===")

LanguageDetectionUtils.group_chunks_by_language(chunks)
|> Enum.each(fn {lang, group} ->
  IO.puts("Language: #{lang} - Chunks: #{length(group)}")
end)

IO.puts("\n=== Filter English Chunks (min 80% confidence) ===")
IO.inspect(LanguageDetectionUtils.filter_by_language(chunks, "en", 0.8))

IO.puts("\n=== Check if Single Language ===")
IO.puts(LanguageDetectionUtils.is_single_language?(chunks, 0.8))
