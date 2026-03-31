defmodule Kreuzberg.ArchiveEntry do
  @moduledoc """
  Structure representing an entry extracted from an archive file.

  Matches the Rust `ArchiveEntry` struct.

  ## Fields

    * `:path` - The path of the entry within the archive
    * `:mime_type` - The MIME type of the entry content
    * `:result` - The extraction result for this entry
  """

  @type t :: %__MODULE__{
          path: String.t(),
          mime_type: String.t(),
          result: map() | nil
        }

  defstruct path: "", mime_type: "", result: nil

  @doc """
  Creates an ArchiveEntry struct from a map.

  ## Examples

      iex> Kreuzberg.ArchiveEntry.from_map(%{
      ...>   "path" => "document.pdf",
      ...>   "mime_type" => "application/pdf",
      ...>   "result" => %{"content" => "text"}
      ...> })
      %Kreuzberg.ArchiveEntry{
        path: "document.pdf",
        mime_type: "application/pdf",
        result: %{"content" => "text"}
      }
  """
  @spec from_map(map()) :: t()
  def from_map(data) when is_map(data) do
    %__MODULE__{
      path: data["path"] || "",
      mime_type: data["mime_type"] || "",
      result: data["result"]
    }
  end

  @doc """
  Converts an ArchiveEntry struct to a map.

  ## Examples

      iex> entry = %Kreuzberg.ArchiveEntry{path: "doc.pdf", mime_type: "application/pdf"}
      iex> Kreuzberg.ArchiveEntry.to_map(entry)
      %{
        "path" => "doc.pdf",
        "mime_type" => "application/pdf",
        "result" => nil
      }
  """
  @spec to_map(t()) :: map()
  def to_map(%__MODULE__{} = entry) do
    %{
      "path" => entry.path,
      "mime_type" => entry.mime_type,
      "result" => entry.result
    }
  end
end
