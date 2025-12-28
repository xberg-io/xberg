defmodule Kreuzberg.Error do
  @moduledoc """
  Exception module for Kreuzberg extraction errors.

  Defines error types and provides standardized error handling for all
  extraction and configuration-related failures in the Kreuzberg library.

  ## Error Types

  All errors inherit from this exception and include:

    * `:message` - Human-readable error description
    * `:reason` - Categorized error reason (atom)
    * `:context` - Optional additional context about the error

  ## Exceptions

    * `Kreuzberg.Error` - Base exception for all Kreuzberg errors

  ## Examples

      iex> raise Kreuzberg.Error, message: "Invalid PDF", reason: :invalid_format
      ** (Kreuzberg.Error) Invalid PDF

      iex> try do
      ...>   raise Kreuzberg.Error, message: "OCR failed", reason: :ocr_error
      ...> rescue
      ...>   e in Kreuzberg.Error ->
      ...>     {e.message, e.reason}
      ...> end
      {"OCR failed", :ocr_error}
  """

  defexception [:message, :reason, :context]

  @type t :: %__MODULE__{
          message: String.t() | nil,
          reason: atom() | nil,
          context: map() | nil
        }

  @type reason ::
          :invalid_format
          | :invalid_config
          | :ocr_error
          | :extraction_error
          | :io_error
          | :nif_error
          | :unknown_error

  @doc """
  Creates a new Kreuzberg error.

  ## Parameters

    * `message` - The error message (defaults to reason atom string)
    * `reason` - The error reason (atom categorizing the error type)
    * `context` - Optional map with additional error context

  ## Returns

  An exception struct that can be raised.

  ## Examples

      iex> error = Kreuzberg.Error.new("File not found", :io_error)
      iex> error.message
      "File not found"
      iex> error.reason
      :io_error

      iex> error = Kreuzberg.Error.new(
      ...>   "Unsupported format",
      ...>   :invalid_format,
      ...>   %{"format" => "xyz", "supported" => ["pdf", "docx"]}
      ...> )
      iex> error.context
      %{"format" => "xyz", "supported" => ["pdf", "docx"]}
  """
  @spec new(String.t(), reason(), map() | nil) :: t()
  def new(message, reason, context \\ nil) do
    %__MODULE__{
      message: message,
      reason: reason,
      context: context
    }
  end

  @doc """
  Converts an error to a descriptive string representation.

  Includes the message and reason, with context details if available.

  ## Parameters

    * `error` - A Kreuzberg.Error struct

  ## Returns

  A formatted error string.

  ## Examples

      iex> error = Kreuzberg.Error.new("Failed to extract", :extraction_error)
      iex> Kreuzberg.Error.to_string(error)
      "Failed to extract (extraction_error)"

      iex> error = Kreuzberg.Error.new(
      ...>   "Invalid format",
      ...>   :invalid_format,
      ...>   %{"details" => "unsupported"}
      ...> )
      iex> Kreuzberg.Error.to_string(error)
      "Invalid format (invalid_format) - context: %{\\"details\\" => \\"unsupported\\"}"
  """
  @spec to_string(t()) :: String.t()
  def to_string(%__MODULE__{message: message, reason: reason, context: context}) do
    base = "#{message} (#{reason})"

    if context do
      "#{base} - context: #{inspect(context)}"
    else
      base
    end
  end

  @impl true
  def message(%__MODULE__{message: message, reason: reason, context: context}) do
    cond do
      message && reason && context ->
        "#{message} (#{reason}) - context: #{inspect(context)}"

      message && reason ->
        "#{message} (#{reason})"

      message ->
        message

      reason ->
        Atom.to_string(reason)

      true ->
        "Kreuzberg error"
    end
  end
end
