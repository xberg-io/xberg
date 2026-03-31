defmodule Kreuzberg.ResultFormat do
  @moduledoc """
  Enumeration of result structure formats.

  Matches the Rust `ResultFormat` enum.

  ## Values

    * `:unified` - All content in a unified content field
    * `:element_based` - Content split into semantic elements
  """

  @type t :: :unified | :element_based

  @doc """
  Returns all valid ResultFormat values.

  ## Examples

      iex> Kreuzberg.ResultFormat.values()
      [:unified, :element_based]
  """
  @spec values() :: list(t())
  def values, do: [:unified, :element_based]
end
