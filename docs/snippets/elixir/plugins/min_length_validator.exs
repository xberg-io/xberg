```elixir title="Elixir"
defmodule MinLengthValidator do
  @behaviour Xberg.Plugin.Validator

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
Xberg.Plugin.register_validator(MinLengthValidator)

# Example usage with extraction
input = %Xberg.ExtractInput{kind: :uri, uri: "document.pdf"}
{:ok, output} = Xberg.extract(input, nil)
result = List.first(output.results)

case result do
  result ->
    IO.puts("Extraction successful")
    IO.puts("Content length: #{String.length(result.content)}")
end
```
