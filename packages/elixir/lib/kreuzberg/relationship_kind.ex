defmodule Kreuzberg.RelationshipKind do
  @moduledoc """
  Enumeration of relationship kinds between document elements.

  Matches the Rust `RelationshipKind` enum.

  ## Values

    * `:footnote_reference` - Reference to a footnote
    * `:citation_reference` - Reference to a citation
    * `:internal_link` - Internal document link
    * `:caption` - Caption relationship
    * `:label` - Label relationship
    * `:toc_entry` - Table of contents entry
    * `:cross_reference` - Cross-reference between elements
  """

  @type t ::
          :footnote_reference
          | :citation_reference
          | :internal_link
          | :caption
          | :label
          | :toc_entry
          | :cross_reference

  @doc """
  Returns all valid RelationshipKind values.

  ## Examples

      iex> Kreuzberg.RelationshipKind.values()
      [:footnote_reference, :citation_reference, :internal_link, :caption, :label, :toc_entry, :cross_reference]
  """
  @spec values() :: list(t())
  def values,
    do: [
      :footnote_reference,
      :citation_reference,
      :internal_link,
      :caption,
      :label,
      :toc_entry,
      :cross_reference
    ]
end
