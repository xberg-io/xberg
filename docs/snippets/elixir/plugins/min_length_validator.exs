```elixir title="Elixir"
defmodule MinLengthValidator do
  @behaviour Kreuzberg.Plugin.Validator

  @min_length 50

  def name, do: "min_length_validator"

  def validate(result) do
    if String.length(result.content) >= @min_length do
      :ok
    else
      {:error, "Content too short"}
    end
  end

  def should_validate?(_result), do: true
  def priority, do: 1
  def initialize, do: :ok
  def shutdown, do: :ok
  def version, do: "1.0.0"
end

# Register the validator
Kreuzberg.Plugin.register_validator(MinLengthValidator)

# Example usage with extraction
{:ok, result} = Kreuzberg.extract_file(
  "document.pdf",
  nil
)

case result do
  result ->
    IO.puts("Extraction successful")
    IO.puts("Content length: #{String.length(result.content)}")
end
```
