defmodule Kreuzberg.ElementType do
  @moduledoc """
  Enumeration of semantic element types in a document.

  Matches the Rust `ElementType` enum.

  ## Values

    * `:title` - Document title
    * `:narrative_text` - Main narrative text body
    * `:heading` - Section heading
    * `:list_item` - List item
    * `:table` - Table element
    * `:image` - Image element
    * `:page_break` - Page break marker
    * `:code_block` - Code block
    * `:block_quote` - Block quote
    * `:footer` - Footer text
    * `:header` - Header text
  """

  @type t ::
          :title
          | :narrative_text
          | :heading
          | :list_item
          | :table
          | :image
          | :page_break
          | :code_block
          | :block_quote
          | :footer
          | :header

  @doc """
  Returns all valid ElementType values.

  ## Examples

      iex> Kreuzberg.ElementType.values()
      [:title, :narrative_text, :heading, :list_item, :table, :image, :page_break, :code_block, :block_quote, :footer, :header]
  """
  @spec values() :: list(t())
  def values,
    do: [
      :title,
      :narrative_text,
      :heading,
      :list_item,
      :table,
      :image,
      :page_break,
      :code_block,
      :block_quote,
      :footer,
      :header
    ]
end
