defmodule Kreuzberg do
  @moduledoc """
  High-performance document intelligence library for Elixir.

  Kreuzberg provides advanced document extraction capabilities with support for:
  - PDF, DOCX, HTML, and more
  - OCR for scanned documents
  - Custom post-processors and validators
  - Image extraction

  ## Examples

      # Basic extraction
      {:ok, result} = Kreuzberg.extract(pdf_binary, :pdf)

      # Extract with options (to be implemented)
      # options = %Kreuzberg.Options{extract_images: true}
      # {:ok, result} = Kreuzberg.extract(pdf_binary, :pdf, options)
  """

  alias Kreuzberg.Native

  @type input_type :: :pdf | :docx | :html | :markdown | :text
  @type extraction_result :: {:ok, map()} | {:error, term()}

  @doc """
  Extract content from a document.

  ## Parameters
    - `input`: Binary data of the document
    - `input_type`: Type of the document (`:pdf`, `:docx`, `:html`, etc.)

  ## Returns
    - `{:ok, result}` on success
    - `{:error, reason}` on failure
  """
  @spec extract(binary(), input_type()) :: extraction_result()
  def extract(input, input_type) when is_binary(input) and is_atom(input_type) do
    input_type_string = Atom.to_string(input_type)
    Native.extract(input, input_type_string)
  end

  @doc """
  Extract content from a document and raise on failure.
  """
  @spec extract!(binary(), input_type()) :: map()
  def extract!(input, input_type) do
    case extract(input, input_type) do
      {:ok, result} -> result
      {:error, reason} -> raise "Extraction failed: #{inspect(reason)}"
    end
  end
end
