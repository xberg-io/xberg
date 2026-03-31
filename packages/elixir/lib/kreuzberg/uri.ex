defmodule Kreuzberg.Uri do
  @moduledoc """
  Structure representing a URI extracted from a document.

  Matches the Rust `Uri` struct.

  ## Fields

    * `:url` - The URL string
    * `:label` - Optional display label for the URI
    * `:page` - Optional page number where the URI appears
    * `:kind` - The kind of URI (e.g., "hyperlink", "image", "anchor")
  """

  @type t :: %__MODULE__{
          url: String.t(),
          label: String.t() | nil,
          page: integer() | nil,
          kind: String.t()
        }

  defstruct [
    :label,
    :page,
    url: "",
    kind: "hyperlink"
  ]

  @doc """
  Creates a Uri struct from a map.

  ## Examples

      iex> Kreuzberg.Uri.from_map(%{
      ...>   "url" => "https://example.com",
      ...>   "label" => "Example",
      ...>   "page" => 1,
      ...>   "kind" => "hyperlink"
      ...> })
      %Kreuzberg.Uri{
        url: "https://example.com",
        label: "Example",
        page: 1,
        kind: "hyperlink"
      }
  """
  @spec from_map(map()) :: t()
  def from_map(data) when is_map(data) do
    %__MODULE__{
      url: data["url"] || "",
      label: data["label"],
      page: data["page"],
      kind: data["kind"] || "hyperlink"
    }
  end

  @doc """
  Converts a Uri struct to a map.

  ## Examples

      iex> uri = %Kreuzberg.Uri{url: "https://example.com", kind: "hyperlink"}
      iex> Kreuzberg.Uri.to_map(uri)
      %{
        "url" => "https://example.com",
        "label" => nil,
        "page" => nil,
        "kind" => "hyperlink"
      }
  """
  @spec to_map(t()) :: map()
  def to_map(%__MODULE__{} = uri) do
    %{
      "url" => uri.url,
      "label" => uri.label,
      "page" => uri.page,
      "kind" => uri.kind
    }
  end
end
