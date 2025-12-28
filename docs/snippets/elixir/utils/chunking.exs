# Utility functions for working with chunks
defmodule ChunkingUtils do
  @doc """
  Calculate statistics for a list of chunks.
  Returns min, max, and average chunk sizes.
  """
  def chunk_size_stats(chunks) when is_list(chunks) do
    case chunks do
      [] ->
        %{min: 0, max: 0, avg: 0}

      chunks ->
        sizes = Enum.map(chunks, &chunk_text_length/1)

        %{
          min: Enum.min(sizes),
          max: Enum.max(sizes),
          avg: div(Enum.sum(sizes), length(sizes)),
          count: length(chunks)
        }
    end
  end

  @doc """
  Filter chunks by a minimum size threshold.
  """
  def filter_by_min_size(chunks, min_size) do
    Enum.filter(chunks, &(chunk_text_length(&1) >= min_size))
  end

  @doc """
  Merge adjacent chunks if they are below a size threshold.
  """
  def merge_small_chunks(chunks, threshold) do
    chunks
    |> Enum.reduce([], fn chunk, acc ->
      case acc do
        [] ->
          [chunk]

        [last | rest] ->
          last_size = chunk_text_length(last)

          if last_size < threshold do
            merged_text = "#{last["text"]} #{chunk["text"]}"
            merged_chunk = Map.put(chunk, "text", merged_text)
            [merged_chunk | rest]
          else
            [chunk, last | rest]
          end
      end
    end)
    |> Enum.reverse()
  end

  @doc """
  Group chunks by document section (if metadata contains section info).
  """
  def group_by_section(chunks) do
    Enum.group_by(chunks, fn chunk ->
      Map.get(chunk, "metadata", %{})
      |> Map.get("section", "general")
    end)
  end

  # Private helper
  defp chunk_text_length(chunk) do
    chunk
    |> Map.get("text", "")
    |> String.length()
  end
end

# Example usage
config = %Kreuzberg.ExtractionConfig{
  chunking: %{"enabled" => true, "max_chars" => 1000}
}

{:ok, result} = Kreuzberg.extract_file("doc.pdf", nil, config)
chunks = result.chunks || []

IO.puts("=== Chunk Statistics ===")
IO.inspect(ChunkingUtils.chunk_size_stats(chunks))

IO.puts("\n=== Chunks by Section ===")
IO.inspect(ChunkingUtils.group_by_section(chunks))

IO.puts("\n=== Filtering chunks >= 500 chars ===")
IO.inspect(ChunkingUtils.filter_by_min_size(chunks, 500))

IO.puts("\n=== Merging small chunks < 200 chars ===")
IO.inspect(ChunkingUtils.merge_small_chunks(chunks, 200))
