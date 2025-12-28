```elixir title="Elixir"
alias Kreuzberg.Plugin

# Quality Score Validator Plugin
# This validator ensures extracted content meets a minimum quality threshold.
# It checks the quality_score metadata field and rejects low-quality extractions.

defmodule MyApp.Plugins.QualityScoreValidator do
  @behaviour Kreuzberg.Plugin.Validator
  require Logger

  @impl true
  def name do
    "QualityScoreValidator"
  end

  @impl true
  def initialize do
    :ok
  end

  @impl true
  def shutdown do
    :ok
  end

  @impl true
  def priority do
    100
  end

  @impl true
  def should_validate?(result) do
    true
  end

  @impl true
  def validate(result) do
    # Extract quality score from ExtractionResult struct's metadata map
    quality_score = result.metadata["quality_score"] || 0.0

    if is_number(quality_score) and quality_score >= 0.5 do
      :ok
    else
      {:error, "Quality score too low: #{Float.round(quality_score, 2)}. Minimum required: 0.5"}
    end
  end
end

# Register the quality validator plugin
Plugin.register_validator(MyApp.Plugins.QualityScoreValidator)

# Example usage with extraction
# Note: In real usage, result will be an ExtractionResult struct, not a map.
# This example shows the data structure for illustration purposes.
result = %{
  "content" => "Extracted document content",
  "metadata" => %{
    "quality_score" => 0.85,
    "pages" => 1
  }
}

case MyApp.Plugins.QualityScoreValidator.validate(result) do
  :ok ->
    IO.puts("Quality validation passed: #{result["metadata"]["quality_score"]}")

  {:error, reason} ->
    IO.puts("Quality validation failed: #{reason}")
end

# List all registered validators
{:ok, validators} = Plugin.list_validators()
IO.inspect(validators, label: "Registered Validators")
```
