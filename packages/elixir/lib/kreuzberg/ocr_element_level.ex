defmodule Kreuzberg.OcrElementLevel do
  @moduledoc """
  Enumeration of OCR element hierarchical levels.

  Matches the Rust `OcrElementLevel` enum.

  ## Values

    * `:word` - Word-level element
    * `:line` - Line-level element
    * `:block` - Block-level element
    * `:page` - Page-level element
  """

  @type t :: :word | :line | :block | :page

  @doc """
  Returns all valid OcrElementLevel values.

  ## Examples

      iex> Kreuzberg.OcrElementLevel.values()
      [:word, :line, :block, :page]
  """
  @spec values() :: list(t())
  def values, do: [:word, :line, :block, :page]
end
