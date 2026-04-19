defmodule Kreuzberg.Page do
  @moduledoc """
  Structure representing a single page extracted from a multi-page document.

  Matches the Rust `PageContent` struct.

  ## Fields

    * `:page_number` - Page number (0-indexed in Rust)
    * `:content` - Text content extracted from this page
    * `:tables` - Tables found on this page
    * `:images` - Images found on this page
    * `:hierarchy` - Optional hierarchy information (heading levels and blocks)
    * `:is_blank` - Whether the page is blank (nil if unknown)
    * `:layout_regions` - Detected layout regions on this page (nil if layout detection was not enabled)

  ## Examples

      iex> page = %Kreuzberg.Page{
      ...>   page_number: 0,
      ...>   content: "Page 1 content here"
      ...> }
      iex> page.page_number
      0
  """

  @type t :: %__MODULE__{
          page_number: non_neg_integer(),
          content: String.t(),
          tables: list(Kreuzberg.Table.t()),
          images: list(Kreuzberg.Image.t()),
          hierarchy: Kreuzberg.PageHierarchy.t() | nil,
          is_blank: boolean() | nil,
          layout_regions: list(Kreuzberg.LayoutRegion.t()) | nil
        }

  defstruct [
    :hierarchy,
    :is_blank,
    :layout_regions,
    page_number: 0,
    content: "",
    tables: [],
    images: []
  ]

  @doc """
  Creates a Page struct from a map.

  Converts nested table and image maps to their proper struct types.

  ## Examples

      iex> Kreuzberg.Page.from_map(%{"page_number" => 0, "content" => "text"})
      %Kreuzberg.Page{page_number: 0, content: "text"}
  """
  @spec from_map(map()) :: t()
  def from_map(data) when is_map(data) do
    %__MODULE__{
      page_number: data["page_number"] || 0,
      content: data["content"] || "",
      tables: normalize_tables(data["tables"]),
      images: normalize_images(data["images"]),
      hierarchy: normalize_hierarchy(data["hierarchy"]),
      is_blank: data["is_blank"],
      layout_regions: normalize_layout_regions(data["layout_regions"])
    }
  end

  @doc """
  Converts a Page struct to a map.
  """
  @spec to_map(t()) :: map()
  def to_map(%__MODULE__{} = page) do
    %{
      "page_number" => page.page_number,
      "content" => page.content,
      "tables" => Enum.map(page.tables, &Kreuzberg.Table.to_map/1),
      "images" => Enum.map(page.images, &Kreuzberg.Image.to_map/1),
      "hierarchy" =>
        case page.hierarchy do
          nil -> nil
          h -> Kreuzberg.PageHierarchy.to_map(h)
        end,
      "is_blank" => page.is_blank,
      "layout_regions" =>
        case page.layout_regions do
          nil -> nil
          regions -> Enum.map(regions, &Kreuzberg.LayoutRegion.to_map/1)
        end
    }
  end

  defp normalize_tables(nil), do: []
  defp normalize_tables([]), do: []

  defp normalize_tables(tables) when is_list(tables) do
    Enum.map(tables, fn
      %Kreuzberg.Table{} = t -> t
      map when is_map(map) -> Kreuzberg.Table.from_map(map)
      other -> other
    end)
  end

  defp normalize_images(nil), do: []
  defp normalize_images([]), do: []

  defp normalize_images(images) when is_list(images) do
    Enum.map(images, fn
      %Kreuzberg.Image{} = i -> i
      map when is_map(map) -> Kreuzberg.Image.from_map(map)
      other -> other
    end)
  end

  defp normalize_hierarchy(nil), do: nil
  defp normalize_hierarchy(%Kreuzberg.PageHierarchy{} = h), do: h
  defp normalize_hierarchy(map) when is_map(map), do: Kreuzberg.PageHierarchy.from_map(map)

  defp normalize_layout_regions(nil), do: nil
  defp normalize_layout_regions([]), do: []

  defp normalize_layout_regions(regions) when is_list(regions) do
    Enum.map(regions, fn
      %Kreuzberg.LayoutRegion{} = r -> r
      map when is_map(map) -> Kreuzberg.LayoutRegion.from_map(map)
      other -> other
    end)
  end
end
