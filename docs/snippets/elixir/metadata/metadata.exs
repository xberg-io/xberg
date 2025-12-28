# General metadata extraction and manipulation utilities
defmodule MetadataUtils do
  @doc """
  Extract standard metadata fields from extraction result.
  """
  def extract_standard_metadata(result) do
    metadata = Map.get(result, "metadata", %{})
    %{
      title: Map.get(metadata, "title", "Unknown"),
      author: Map.get(metadata, "author", "Unknown"),
      creation_date: Map.get(metadata, "creation_date"),
      modification_date: Map.get(metadata, "modification_date"),
      language: Map.get(metadata, "language"),
      page_count: Map.get(metadata, "page_count", 0),
      format: Map.get(metadata, "format", "unknown")
    }
    |> Enum.reject(fn {_k, v} -> is_nil(v) end)
    |> Enum.into(%{})
  end

  @doc """
  Merge metadata from multiple extraction results.
  """
  def merge_metadata(results) when is_list(results) do
    results
    |> Enum.reduce(%{}, fn result, acc ->
      metadata = Map.get(result, "metadata", %{})

      Enum.reduce(metadata, acc, fn {key, value}, meta_acc ->
        existing = Map.get(meta_acc, key, [])

        updated_value =
          case existing do
            [] -> [value]
            list when is_list(list) -> list ++ [value]
            single -> [single, value]
          end

        Map.put(meta_acc, key, updated_value)
      end)
    end)
  end

  @doc """
  Enrich chunks with additional metadata.
  """
  def enrich_chunks_metadata(chunks, source_metadata) do
    Enum.map(chunks, fn chunk ->
      chunk_metadata = Map.get(chunk, "metadata", %{})

      enriched =
        source_metadata
        |> Enum.reject(fn {_k, v} -> is_nil(v) end)
        |> Enum.into(chunk_metadata)

      Map.put(chunk, "metadata", enriched)
    end)
  end

  @doc """
  Extract metadata specific to a chunk's context (position, size, etc).
  """
  def extract_chunk_context(chunks) do
    total_chunks = length(chunks)

    chunks
    |> Enum.with_index()
    |> Enum.map(fn {chunk, index} ->
      metadata = Map.get(chunk, "metadata", %{})
      text = Map.get(chunk, "text", "")

      context = %{
        "chunk_index" => index,
        "chunk_number" => index + 1,
        "total_chunks" => total_chunks,
        "position_percentage" => Float.round((index + 1) / total_chunks * 100, 2),
        "text_length" => String.length(text),
        "word_count" => String.split(text) |> length(),
        "has_headings" => String.contains?(text, ~w[## # ===]),
        "has_lists" => String.contains?(text, ["- ", "* "])
      }

      enriched_metadata = Map.merge(metadata, context)
      Map.put(chunk, "metadata", enriched_metadata)
    end)
  end

  @doc """
  Filter metadata by a set of allowed keys.
  """
  def filter_metadata_keys(data, allowed_keys) when is_list(allowed_keys) do
    metadata = Map.get(data, "metadata", %{})

    filtered =
      metadata
      |> Enum.filter(fn {key, _value} -> Enum.member?(allowed_keys, key) end)
      |> Enum.into(%{})

    Map.put(data, "metadata", filtered)
  end

  @doc """
  Create a metadata summary across all chunks.
  """
  def metadata_summary(chunks) do
    %{
      total_chunks: length(chunks),
      total_text_length: Enum.reduce(chunks, 0, fn chunk, acc ->
        String.length(Map.get(chunk, "text", "")) + acc
      end),
      avg_chunk_size: calculate_avg_size(chunks),
      metadata_fields: extract_all_metadata_fields(chunks),
      enrichment_level: assess_enrichment(chunks)
    }
  end

  @doc """
  Generate a human-readable metadata report.
  """
  def generate_report(result) do
    standard = extract_standard_metadata(result)
    chunks = Map.get(result, "chunks", [])

    """
    === Document Metadata Report ===

    Standard Fields:
    #{format_dict(standard)}

    Chunk Statistics:
    - Total Chunks: #{length(chunks)}
    - Avg Chunk Size: #{calculate_avg_size(chunks)} characters

    Metadata Summary:
    #{format_dict(metadata_summary(chunks))}
    """
  end

  # Private helpers
  defp calculate_avg_size(chunks) do
    case chunks do
      [] ->
        0

      chunks ->
        total = Enum.reduce(chunks, 0, fn chunk, acc ->
          String.length(Map.get(chunk, "text", "")) + acc
        end)

        div(total, length(chunks))
    end
  end

  defp extract_all_metadata_fields(chunks) do
    chunks
    |> Enum.flat_map(fn chunk ->
      chunk
      |> Map.get("metadata", %{})
      |> Map.keys()
    end)
    |> Enum.uniq()
  end

  defp assess_enrichment(chunks) do
    avg_fields =
      chunks
      |> Enum.map(fn chunk ->
        chunk
        |> Map.get("metadata", %{})
        |> map_size()
      end)
      |> then(fn sizes ->
        if Enum.empty?(sizes) do
          0
        else
          div(Enum.sum(sizes), length(sizes))
        end
      end)

    case avg_fields do
      count when count >= 5 -> "high"
      count when count >= 2 -> "medium"
      _ -> "low"
    end
  end

  defp format_dict(dict) when is_map(dict) do
    dict
    |> Enum.map(fn {key, value} ->
      "  #{key}: #{inspect(value)}"
    end)
    |> Enum.join("\n")
  end
end

# Example usage
config = %Kreuzberg.ExtractionConfig{
  chunking: %{"enabled" => true, "max_chars" => 1000}
}

{:ok, result} = Kreuzberg.extract_file("document.pdf", nil, config)

IO.puts("=== Standard Metadata ===")
IO.inspect(MetadataUtils.extract_standard_metadata(result))

IO.puts("\n=== Chunk Context ===")

enhanced_chunks = MetadataUtils.extract_chunk_context(result.chunks || [])
IO.inspect(hd(enhanced_chunks))

IO.puts("\n=== Metadata Summary ===")
IO.inspect(MetadataUtils.metadata_summary(result.chunks || []))

IO.puts("\n=== Report ===")
IO.puts(MetadataUtils.generate_report(result))
