defmodule Kreuzberg.Test.ExamplePostProcessor do
  @moduledoc """
  Example post-processor plugin for text normalization.

  This post-processor demonstrates how to implement a custom post-processor
  that normalizes extracted text content by converting to lowercase and trimming
  whitespace.

  ## Processing Stage

  Runs in the `:middle` stage, after initial extraction but before final validation.

  ## Behavior

  - Converts all content to lowercase
  - Trims leading/trailing whitespace
  - Removes extra whitespace between words

  ## Example

      config = %{"normalize_whitespace" => true}
      result = %{content: "  Hello  WORLD  ", metadata: %{}}
      processed = Kreuzberg.Test.ExamplePostProcessor.process(result, config)
      # Result: %{content: "hello world", metadata: %{}}
  """

  @behaviour Kreuzberg.Plugin.PostProcessor

  @impl true
  def name do
    "text_normalizer"
  end

  @impl true
  def version do
    "1.0.0"
  end

  @impl true
  def processing_stage do
    :middle
  end

  @impl true
  def initialize do
    # No special initialization needed for this processor
    :ok
  end

  @impl true
  def shutdown do
    # No cleanup needed for this processor
    :ok
  end

  @impl true
  def process(result, config) do
    config = config || %{}

    result
    |> normalize_content(config)
    |> add_processor_metadata(config)
  end

  # Private helpers

  defp normalize_content(result, _config) do
    content = result.content
    # Convert to lowercase
    |> String.downcase()
    # Trim leading/trailing whitespace
    |> String.trim()
    # Replace multiple spaces with single space
    |> normalize_whitespace()

    %{result | content: content}
  end

  defp normalize_whitespace(text) do
    Regex.replace(~r/\s+/, text, " ")
  end

  defp add_processor_metadata(result, config) do
    if Map.get(config, "add_processor_info", false) do
      metadata =
        (result.metadata || %{})
        |> Map.put("processed_by", "text_normalizer")
        |> Map.put("processing_stage", "middle")

      %{result | metadata: metadata}
    else
      result
    end
  end
end
