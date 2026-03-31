defmodule Kreuzberg.ContentLayer do
  @moduledoc """
  Enumeration of content layers within a document.

  Matches the Rust `ContentLayer` enum.

  ## Values

    * `:body` - Main body content
    * `:header` - Header content
    * `:footer` - Footer content
    * `:footnote` - Footnote content
  """

  @type t :: :body | :header | :footer | :footnote

  @doc """
  Returns all valid ContentLayer values.

  ## Examples

      iex> Kreuzberg.ContentLayer.values()
      [:body, :header, :footer, :footnote]
  """
  @spec values() :: list(t())
  def values, do: [:body, :header, :footer, :footnote]
end
