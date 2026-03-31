defmodule Kreuzberg.PdfAnnotationType do
  @moduledoc """
  Enumeration of PDF annotation types.

  Matches the Rust `PdfAnnotationType` enum.

  ## Values

    * `:text` - Text annotation
    * `:highlight` - Highlight annotation
    * `:link` - Link annotation
    * `:stamp` - Stamp annotation
    * `:underline` - Underline annotation
    * `:strike_out` - Strike-out annotation
    * `:other` - Other annotation type
  """

  @type t :: :text | :highlight | :link | :stamp | :underline | :strike_out | :other

  @doc """
  Returns all valid PdfAnnotationType values.

  ## Examples

      iex> Kreuzberg.PdfAnnotationType.values()
      [:text, :highlight, :link, :stamp, :underline, :strike_out, :other]
  """
  @spec values() :: list(t())
  def values, do: [:text, :highlight, :link, :stamp, :underline, :strike_out, :other]
end
