```elixir title="Elixir"
defmodule MyApp.MinLengthValidator do
  @behaviour Kreuzberg.Plugin.Validator

  def name, do: "min_length_validator"

  def validate(result) do
    if String.length(result.content) >= 50 do
      :ok
    else
      {:error, "Content too short: #{String.length(result.content)} chars"}
    end
  end

  def should_validate?(_result), do: true
  def priority, do: 10
  def initialize, do: :ok
  def shutdown, do: :ok
  def version, do: "1.0.0"
end

# Register validator
Kreuzberg.Plugin.register_validator(MyApp.MinLengthValidator)

# Use with extraction
{:ok, result} = Kreuzberg.extract_file(
  "document.pdf",
  nil
)

IO.puts("Content length: #{String.length(result.content)}")
```
