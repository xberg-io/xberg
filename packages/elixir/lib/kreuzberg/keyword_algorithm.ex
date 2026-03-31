defmodule Kreuzberg.KeywordAlgorithm do
  @moduledoc """
  Enumeration of keyword extraction algorithms.

  Matches the Rust `KeywordAlgorithm` enum.

  ## Values

    * `:yake` - YAKE keyword extraction algorithm
    * `:rake` - RAKE keyword extraction algorithm
  """

  @type t :: :yake | :rake

  @doc """
  Returns all valid KeywordAlgorithm values.

  ## Examples

      iex> Kreuzberg.KeywordAlgorithm.values()
      [:yake, :rake]
  """
  @spec values() :: list(t())
  def values, do: [:yake, :rake]
end
