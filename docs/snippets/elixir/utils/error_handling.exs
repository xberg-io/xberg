# Error handling patterns for Kreuzberg extraction
defmodule ErrorHandlingUtils do
  @doc """
  Safely extract a file with retry logic and error recovery.
  """
  def extract_with_retry(file_path, config, max_retries \\ 3) do
    extract_with_retry(file_path, config, max_retries, 1, nil)
  end

  defp extract_with_retry(_file_path, _config, max_retries, attempt, _error)
       when attempt > max_retries do
    {:error, "Max retries (#{max_retries}) exceeded"}
  end

  defp extract_with_retry(file_path, config, max_retries, attempt, _prev_error) do
    case Kreuzberg.extract_file(file_path, nil, config) do
      {:ok, result} ->
        {:ok, result}

      {:error, reason} ->
        IO.puts("Attempt #{attempt} failed: #{inspect(reason)}")
        Process.sleep(100 * attempt)  # Exponential backoff
        extract_with_retry(file_path, config, max_retries, attempt + 1, reason)
    end
  end

  @doc """
  Extract multiple files and collect results and errors separately.
  """
  def extract_multiple(files, config) do
    files
    |> Enum.map(fn file ->
      {file, Kreuzberg.extract_file(file, nil, config)}
    end)
    |> Enum.reduce(%{successes: [], failures: []}, fn {file, result}, acc ->
      case result do
        {:ok, data} ->
          Map.update!(acc, :successes, &[{file, data} | &1])

        {:error, reason} ->
          Map.update!(acc, :failures, &[{file, reason} | &1])
      end
    end)
    |> then(fn acc ->
      %{
        acc
        | successes: Enum.reverse(acc.successes),
          failures: Enum.reverse(acc.failures)
      }
    end)
  end

  @doc """
  Validate extraction result and return detailed error information.
  """
  def validate_result(result, required_fields \\ ["text", "metadata"]) do
    case result do
      {:ok, data} ->
        missing = Enum.filter(required_fields, &(!Map.has_key?(data, &1)))

        if Enum.empty?(missing) do
          {:ok, data}
        else
          {:error, "Missing required fields: #{inspect(missing)}"}
        end

      {:error, reason} ->
        {:error, format_error(reason)}
    end
  end

  @doc """
  Format errors into human-readable messages.
  """
  def format_error(reason) when is_binary(reason), do: reason

  def format_error(reason) when is_atom(reason) do
    case reason do
      :file_not_found -> "The specified file could not be found"
      :invalid_format -> "The file format is not supported"
      :extraction_failed -> "Failed to extract content from the file"
      :timeout -> "Extraction operation timed out"
      :permission_denied -> "Permission denied when accessing the file"
      other -> "Unknown error: #{inspect(other)}"
    end
  end

  def format_error(reason), do: inspect(reason)

  @doc """
  Log extraction metrics for debugging and monitoring.
  """
  def log_metrics(file_path, result, duration_ms) do
    status =
      case result do
        {:ok, _} -> "success"
        {:error, _} -> "failure"
      end

    IO.puts("""
    [#{DateTime.utc_now()}] Extraction Metrics
    - File: #{file_path}
    - Status: #{status}
    - Duration: #{duration_ms}ms
    """)

    case result do
      {:ok, data} ->
        IO.puts("- Chunks: #{length(data.chunks || [])}")
        IO.puts("- Text length: #{String.length(data.content || "")}")

      {:error, reason} ->
        IO.puts("- Error: #{format_error(reason)}")
    end
  end
end

# Example usage with error handling
config = %Kreuzberg.ExtractionConfig{
  chunking: %{"enabled" => true, "max_chars" => 1000}
}

IO.puts("=== Extract with Retry ===")

case ErrorHandlingUtils.extract_with_retry("document.pdf", config, 3) do
  {:ok, result} ->
    IO.puts("Extraction succeeded")
    IO.inspect(result)

  {:error, reason} ->
    IO.puts("Extraction failed: #{reason}")
end

IO.puts("\n=== Extract Multiple Files ===")

files = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]

case ErrorHandlingUtils.extract_multiple(files, config) do
  results ->
    IO.puts("Successes: #{length(results.successes)}")
    IO.puts("Failures: #{length(results.failures)}")
    IO.inspect(results)
end

IO.puts("\n=== Validate Result ===")

{:ok, result} = Kreuzberg.extract_file("test.pdf", nil, config)

case ErrorHandlingUtils.validate_result(result, ["text", "chunks"]) do
  {:ok, data} ->
    IO.puts("Validation passed")

  {:error, reason} ->
    IO.puts("Validation failed: #{reason}")
end
