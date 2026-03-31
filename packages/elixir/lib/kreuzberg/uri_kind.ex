defmodule Kreuzberg.UriKind do
  @moduledoc """
  Enumeration of URI kinds.

  Matches the Rust `UriKind` enum.

  ## Values

    * `:hyperlink` - Standard hyperlink
    * `:image` - Image URI
    * `:anchor` - Anchor link
    * `:citation` - Citation URI
    * `:reference` - Reference URI
    * `:email` - Email URI
  """

  @type t :: :hyperlink | :image | :anchor | :citation | :reference | :email

  @doc """
  Returns all valid UriKind values.

  ## Examples

      iex> Kreuzberg.UriKind.values()
      [:hyperlink, :image, :anchor, :citation, :reference, :email]
  """
  @spec values() :: list(t())
  def values, do: [:hyperlink, :image, :anchor, :citation, :reference, :email]
end
