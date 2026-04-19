defmodule Kreuzberg.LayoutRegion do
  @moduledoc """
  A detected layout region on a page.

  When layout detection is enabled, each page may have layout regions
  identifying different content types (text, pictures, tables, etc.)
  with confidence scores and spatial positions.

  ## Fields

    * `:class` - Layout class name (e.g. "picture", "table", "text", "section_header")
    * `:confidence` - Detection confidence score (0.0 to 1.0)
    * `:bounding_box` - Map with x0, y0, x1, y1 coordinates, or nil
    * `:area_fraction` - Fraction of page area covered (0.0 to 1.0)

  ## Examples

      iex> region = %Kreuzberg.LayoutRegion{
      ...>   class: "picture",
      ...>   confidence: 0.95,
      ...>   area_fraction: 0.3
      ...> }
      iex> region.class
      "picture"
  """

  @type t :: %__MODULE__{
          class: String.t(),
          confidence: float(),
          bounding_box: map() | nil,
          area_fraction: float()
        }

  defstruct [
    :bounding_box,
    class: "",
    confidence: 0.0,
    area_fraction: 0.0
  ]

  @doc """
  Creates a LayoutRegion struct from a map.

  ## Parameters

    * `data` - A map containing layout region fields

  ## Returns

  A `LayoutRegion` struct with properly typed fields.

  ## Examples

      iex> Kreuzberg.LayoutRegion.from_map(%{"class" => "table", "confidence" => 0.9, "area_fraction" => 0.2})
      %Kreuzberg.LayoutRegion{class: "table", confidence: 0.9, area_fraction: 0.2}
  """
  @spec from_map(map()) :: t()
  def from_map(data) when is_map(data) do
    %__MODULE__{
      class: data["class"] || "",
      confidence: to_float(data["confidence"] || 0.0),
      bounding_box: data["bounding_box"],
      area_fraction: to_float(data["area_fraction"] || 0.0)
    }
  end

  @doc """
  Converts a LayoutRegion struct to a map.

  ## Parameters

    * `region` - A `LayoutRegion` struct

  ## Returns

  A map with string keys representing all fields.

  ## Examples

      iex> region = %Kreuzberg.LayoutRegion{class: "picture", confidence: 0.95, area_fraction: 0.3}
      iex> Kreuzberg.LayoutRegion.to_map(region)
      %{"class" => "picture", "confidence" => 0.95, "bounding_box" => nil, "area_fraction" => 0.3}
  """
  @spec to_map(t()) :: map()
  def to_map(%__MODULE__{} = region) do
    %{
      "class" => region.class,
      "confidence" => region.confidence,
      "bounding_box" => region.bounding_box,
      "area_fraction" => region.area_fraction
    }
  end

  defp to_float(value) when is_float(value), do: value
  defp to_float(value) when is_integer(value), do: value * 1.0

  defp to_float(value) when is_binary(value) do
    case Float.parse(value) do
      {f, _} -> f
      :error -> 0.0
    end
  end

  defp to_float(_), do: 0.0
end
