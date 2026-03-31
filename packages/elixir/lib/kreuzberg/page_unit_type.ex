defmodule Kreuzberg.PageUnitType do
  @moduledoc """
  Enumeration of page unit types in documents.

  Matches the Rust `PageUnitType` enum.

  ## Values

    * `:page` - Standard document page
    * `:slide` - Presentation slide
    * `:sheet` - Spreadsheet sheet
  """

  @type t :: :page | :slide | :sheet

  @doc """
  Returns all valid PageUnitType values.

  ## Examples

      iex> Kreuzberg.PageUnitType.values()
      [:page, :slide, :sheet]
  """
  @spec values() :: list(t())
  def values, do: [:page, :slide, :sheet]
end
