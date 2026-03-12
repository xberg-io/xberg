defmodule Kreuzberg.ChunkMetadata do
  @moduledoc """
  Metadata for a text chunk, tracking byte positions, indices, and page range.

  ## Fields

    * `:byte_start` - Start byte offset in the original content
    * `:byte_end` - End byte offset in the original content
    * `:token_count` - Optional number of tokens in the chunk
    * `:chunk_index` - Zero-indexed position of this chunk
    * `:total_chunks` - Total number of chunks
    * `:first_page` - Optional first page number covered by this chunk
    * `:last_page` - Optional last page number covered by this chunk
    * `:heading_context` - Optional heading hierarchy for this chunk's section
  """

  @type heading_level :: %{level: non_neg_integer(), text: String.t()}

  @type heading_context :: %{headings: [heading_level()]}

  @type t :: %__MODULE__{
          byte_start: non_neg_integer(),
          byte_end: non_neg_integer(),
          token_count: non_neg_integer() | nil,
          chunk_index: non_neg_integer(),
          total_chunks: non_neg_integer(),
          first_page: non_neg_integer() | nil,
          last_page: non_neg_integer() | nil,
          heading_context: heading_context() | nil
        }

  defstruct [
    :token_count,
    :first_page,
    :last_page,
    :heading_context,
    byte_start: 0,
    byte_end: 0,
    chunk_index: 0,
    total_chunks: 0
  ]

  @doc """
  Creates a ChunkMetadata struct from a map.

  ## Examples

      iex> Kreuzberg.ChunkMetadata.from_map(%{"byte_start" => 0, "byte_end" => 100, "chunk_index" => 0, "total_chunks" => 5})
      %Kreuzberg.ChunkMetadata{byte_start: 0, byte_end: 100, chunk_index: 0, total_chunks: 5}
  """
  @spec from_map(map()) :: t()
  def from_map(data) when is_map(data) do
    heading_context =
      case data["heading_context"] do
        %{"headings" => headings} when is_list(headings) ->
          %{headings: Enum.map(headings, fn h -> %{level: h["level"] || 0, text: h["text"] || ""} end)}
        _ ->
          nil
      end

    %__MODULE__{
      byte_start: data["byte_start"] || 0,
      byte_end: data["byte_end"] || 0,
      token_count: data["token_count"],
      chunk_index: data["chunk_index"] || 0,
      total_chunks: data["total_chunks"] || 0,
      first_page: data["first_page"],
      last_page: data["last_page"],
      heading_context: heading_context
    }
  end

  @doc """
  Converts a ChunkMetadata struct to a map.
  """
  @spec to_map(t()) :: map()
  def to_map(map) when is_map(map) and not is_struct(map), do: map

  def to_map(%__MODULE__{} = meta) do
    %{
      "byte_start" => meta.byte_start,
      "byte_end" => meta.byte_end,
      "token_count" => meta.token_count,
      "chunk_index" => meta.chunk_index,
      "total_chunks" => meta.total_chunks,
      "first_page" => meta.first_page,
      "last_page" => meta.last_page,
      "heading_context" => meta.heading_context
    }
  end
end
