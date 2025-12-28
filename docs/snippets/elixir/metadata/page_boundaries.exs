# Page boundary tracking and analysis utilities
defmodule PageBoundaryUtils do
  @doc """
  Track page boundaries throughout the document extraction.
  Assumes chunks contain page_number metadata.
  """
  def track_page_boundaries(chunks) when is_list(chunks) do
    chunks
    |> Enum.with_index()
    |> Enum.reduce(%{}, fn {chunk, index}, acc ->
      page = extract_page_number(chunk)

      if page do
        acc
        |> Map.update(page, [index], &(&1 ++ [index]))
      else
        acc
      end
    end)
  end

  @doc """
  Get all chunks that belong to a specific page.
  """
  def get_chunks_by_page(chunks, page_number) when is_list(chunks) do
    chunks
    |> Enum.filter(fn chunk ->
      extract_page_number(chunk) == page_number
    end)
  end

  @doc """
  Identify page boundaries and create page-level segments.
  """
  def identify_page_segments(chunks) when is_list(chunks) do
    chunks
    |> Enum.reduce([], fn chunk, segments ->
      page = extract_page_number(chunk)
      text = Map.get(chunk, "text", "")

      case segments do
        [] ->
          [%{page: page, content: text, chunks: [chunk]}]

        [current | rest] ->
          if current.page == page do
            # Same page, append to current segment
            updated = %{
              current
              | content: current.content <> " " <> text,
                chunks: current.chunks ++ [chunk]
            }

            [updated | rest]
          else
            # New page, create new segment
            [%{page: page, content: text, chunks: [chunk]}, current | rest]
          end
      end
    end)
    |> Enum.reverse()
  end

  @doc """
  Analyze transitions between pages (e.g., page breaks).
  """
  def analyze_page_transitions(chunks) do
    chunks
    |> Enum.map(&extract_page_number/1)
    |> Enum.uniq()
    |> Enum.sort()
    |> then(fn pages ->
      %{
        total_pages: length(pages),
        page_numbers: pages,
        gaps: find_page_gaps(pages)
      }
    end)
  end

  @doc """
  Get statistics for each page (chunk count, text length, etc).
  """
  def page_statistics(chunks) when is_list(chunks) do
    chunks
    |> Enum.group_by(&extract_page_number/1)
    |> Enum.map(fn {page, page_chunks} ->
      total_length =
        page_chunks
        |> Enum.map(&String.length(Map.get(&1, "text", "")))
        |> Enum.sum()

      %{
        page_number: page,
        chunk_count: length(page_chunks),
        total_text_length: total_length,
        avg_chunk_size: if(length(page_chunks) > 0, do: div(total_length, length(page_chunks)), else: 0)
      }
    end)
    |> Enum.sort_by(&Map.get(&1, :page_number))
  end

  @doc """
  Extract content from a page range.
  """
  def extract_page_range(chunks, start_page, end_page) when is_list(chunks) do
    chunks
    |> Enum.filter(fn chunk ->
      page = extract_page_number(chunk)
      page && page >= start_page && page <= end_page
    end)
  end

  @doc """
  Add page boundary markers to chunks for processing.
  """
  def add_page_markers(chunks) when is_list(chunks) do
    page_segments = identify_page_segments(chunks)

    segments_map =
      page_segments
      |> Enum.reduce(%{}, fn segment, acc ->
        Map.put(acc, segment.page, segment)
      end)

    chunks
    |> Enum.map(fn chunk ->
      page = extract_page_number(chunk)
      metadata = Map.get(chunk, "metadata", %{})

      updated_metadata =
        case segments_map[page] do
          nil ->
            metadata

          segment ->
            metadata
            |> Map.put("page_number", page)
            |> Map.put("is_first_on_page", hd(segment.chunks) == chunk)
            |> Map.put("is_last_on_page", List.last(segment.chunks) == chunk)
            |> Map.put("position_on_page", find_chunk_position(segment.chunks, chunk))
        end

      Map.put(chunk, "metadata", updated_metadata)
    end)
  end

  @doc """
  Generate a page index for quick access.
  """
  def generate_page_index(chunks) when is_list(chunks) do
    chunks
    |> Enum.with_index()
    |> Enum.reduce(%{}, fn {chunk, index}, acc ->
      page = extract_page_number(chunk)

      if page do
        Map.update(acc, page, [index], &(&1 ++ [index]))
      else
        acc
      end
    end)
  end

  @doc """
  Create a table of contents based on page boundaries and content structure.
  """
  def create_page_toc(chunks) do
    chunks
    |> add_page_markers()
    |> Enum.filter(fn chunk ->
      metadata = Map.get(chunk, "metadata", %{})
      Map.get(metadata, "is_first_on_page", false)
    end)
    |> Enum.map(fn chunk ->
      text = Map.get(chunk, "text", "")
      metadata = Map.get(chunk, "metadata", %{})
      page = Map.get(metadata, "page_number", "unknown")

      # Extract first line or heading as TOC entry
      first_line =
        text
        |> String.split("\n")
        |> hd()
        |> String.trim()
        |> then(fn line ->
          if String.length(line) > 100 do
            String.slice(line, 0, 100) <> "..."
          else
            line
          end
        end)

      %{
        page: page,
        content: first_line,
        content_type: detect_content_type(text)
      }
    end)
  end

  # Private helpers
  defp extract_page_number(chunk) do
    chunk
    |> Map.get("metadata", %{})
    |> Map.get("page_number")
  end

  defp find_page_gaps(pages) do
    pages
    |> Enum.chunk_every(2, 1, :discard)
    |> Enum.filter(fn [a, b] -> b - a > 1 end)
    |> Enum.map(fn [a, b] -> {a, b} end)
  end

  defp find_chunk_position(chunks, target_chunk) do
    chunks
    |> Enum.find_index(&(&1 == target_chunk))
    |> then(fn
      nil -> nil
      index -> index + 1
    end)
  end

  defp detect_content_type(text) do
    cond do
      String.contains?(text, ~w[# ## ===]) -> "heading"
      String.contains?(text, ~w[- * •]) -> "list"
      String.length(text) < 100 -> "snippet"
      true -> "body"
    end
  end
end

# Example usage
config = %Kreuzberg.ExtractionConfig{
  chunking: %{"enabled" => true, "max_chars" => 1000}
}

{:ok, result} = Kreuzberg.extract_file("multipage_doc.pdf", nil, config)
chunks = result.chunks || []

IO.puts("=== Page Boundaries ===")
IO.inspect(PageBoundaryUtils.track_page_boundaries(chunks))

IO.puts("\n=== Page Statistics ===")
IO.inspect(PageBoundaryUtils.page_statistics(chunks))

IO.puts("\n=== Page Transitions Analysis ===")
IO.inspect(PageBoundaryUtils.analyze_page_transitions(chunks))

IO.puts("\n=== Chunks from Page 2-4 ===")
IO.inspect(PageBoundaryUtils.extract_page_range(chunks, 2, 4) |> length())

IO.puts("\n=== Page Markers Added ===")

marked_chunks = PageBoundaryUtils.add_page_markers(chunks)
IO.inspect(hd(marked_chunks))

IO.puts("\n=== Page Index ===")
IO.inspect(PageBoundaryUtils.generate_page_index(chunks))

IO.puts("\n=== Table of Contents ===")
IO.inspect(PageBoundaryUtils.create_page_toc(chunks))
