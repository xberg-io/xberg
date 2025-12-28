```elixir title="Elixir"
# MCP Custom Client - Connect to Kreuzberg MCP servers
# Demonstrates creating a reusable MCP client for document extraction

defmodule KreuzbergMCPClient do
  @moduledoc """
  MCP client for communicating with Kreuzberg extraction servers.

  Provides methods for extracting documents from remote MCP servers
  with support for caching, retry logic, and error handling.
  """

  require Logger

  defmodule Config do
    @moduledoc """
    Configuration for MCP client connections.
    """

    defstruct [
      :host,
      :port,
      :timeout_ms,
      :max_retries,
      :retry_delay_ms,
      :cache_dir
    ]

    def new(opts \\ []) do
      %Config{
        host: Keyword.get(opts, :host, "localhost"),
        port: Keyword.get(opts, :port, 8080),
        timeout_ms: Keyword.get(opts, :timeout_ms, 30000),
        max_retries: Keyword.get(opts, :max_retries, 3),
        retry_delay_ms: Keyword.get(opts, :retry_delay_ms, 1000),
        cache_dir: Keyword.get(opts, :cache_dir, nil)
      }
    end
  end

  @doc """
  Extract document from file via MCP server.

  Sends extraction request to remote Kreuzberg MCP server and returns
  structured extraction result with optional caching.

  ## Options

    * `:mime_type` - MIME type of document
    * `:config` - Extraction configuration map
    * `:use_cache` - Enable result caching (default: false)
  """
  @spec extract_file(Config.t(), String.t(), keyword()) ::
          {:ok, map()} | {:error, String.t()}
  def extract_file(config, file_path, opts \\ []) do
    mime_type = Keyword.get(opts, :mime_type)
    extraction_config = Keyword.get(opts, :config)
    use_cache = Keyword.get(opts, :use_cache, false)

    # Check cache first
    if use_cache and config.cache_dir do
      cache_key = compute_cache_key(file_path, mime_type, extraction_config)

      case get_from_cache(config.cache_dir, cache_key) do
        {:ok, cached_result} ->
          Logger.debug("Cache hit for #{file_path}")
          {:ok, cached_result}

        :miss ->
          # Cache miss, fetch from server
          case fetch_from_server(config, file_path, mime_type, extraction_config) do
            {:ok, result} ->
              if use_cache, do: store_in_cache(config.cache_dir, cache_key, result)
              {:ok, result}

            error ->
              error
          end
      end
    else
      fetch_from_server(config, file_path, mime_type, extraction_config)
    end
  end

  @doc """
  Upload and extract document via MCP server.

  Reads file from disk, uploads it to the server, and returns extraction result.
  Useful for server-side processing of large files.
  """
  @spec upload_and_extract(Config.t(), String.t(), keyword()) ::
          {:ok, map()} | {:error, String.t()}
  def upload_and_extract(config, file_path, opts \\ []) do
    unless File.exists?(file_path) do
      {:error, "File not found: #{file_path}"}
    else
      case File.read(file_path) do
        {:ok, body} ->
          url = "http://#{config.host}:#{config.port}/extract/file"

          headers = [
            {"Content-Type", "application/octet-stream"},
            {"X-File-Name", Path.basename(file_path)}
          ]

          case HTTPoison.post(url, body, headers, timeout: config.timeout_ms) do
            {:ok, response} ->
              handle_response(response)

            {:error, reason} ->
              Logger.error("Upload failed: #{inspect(reason)}")
              {:error, "Upload failed: #{inspect(reason)}"}
          end

        {:error, reason} ->
          {:error, "Failed to read file: #{inspect(reason)}"}
      end
    end
  end

  @doc """
  Check health status of MCP server.
  """
  @spec health_check(Config.t()) :: {:ok, map()} | {:error, String.t()}
  def health_check(config) do
    url = "http://#{config.host}:#{config.port}/health"

    case HTTPoison.get(url, [], timeout: config.timeout_ms) do
      {:ok, response} ->
        case handle_response(response) do
          {:ok, data} -> {:ok, data}
          error -> error
        end

      {:error, reason} ->
        {:error, "Health check failed: #{inspect(reason)}"}
    end
  end

  @doc """
  Batch extract multiple documents with parallel requests.

  Sends concurrent extraction requests for better throughput with large
  document collections.
  """
  @spec batch_extract(Config.t(), [String.t()], keyword()) ::
          {:ok, [map()]} | {:error, String.t()}
  def batch_extract(config, file_paths, opts \\ []) do
    Logger.info("Batch extracting #{length(file_paths)} documents")

    results =
      file_paths
      |> Task.async_stream(fn path ->
        extract_file(config, path, opts)
      end)
      |> Stream.map(fn {:ok, result} -> result end)
      |> Enum.to_list()

    success_count = Enum.count(results, &match?({:ok, _}, &1))
    Logger.info("Batch extraction complete: #{success_count}/#{length(file_paths)} succeeded")

    {:ok, results}
  end

  # Private helpers

  defp fetch_from_server(config, file_path, mime_type, extraction_config) do
    url = "http://#{config.host}:#{config.port}/extract"

    body =
      Jason.encode!(%{
        file_path: file_path,
        mime_type: mime_type,
        config: extraction_config
      })

    headers = [{"Content-Type", "application/json"}]

    retry_request(config, fn ->
      HTTPoison.post(url, body, headers, timeout: config.timeout_ms)
    end)
    |> case do
      {:ok, response} -> handle_response(response)
      error -> error
    end
  end

  defp retry_request(config, request_fn) do
    retry_request(config, request_fn, 0)
  end

  defp retry_request(config, request_fn, attempt) when attempt < config.max_retries do
    case request_fn.() do
      {:ok, response} ->
        {:ok, response}

      {:error, reason} ->
        Logger.warn("Request failed (attempt #{attempt + 1}): #{inspect(reason)}")
        Process.sleep(config.retry_delay_ms)
        retry_request(config, request_fn, attempt + 1)
    end
  end

  defp retry_request(_config, _request_fn, _attempt) do
    {:error, "Max retries exceeded"}
  end

  defp handle_response(%HTTPoison.Response{status_code: 200, body: body}) do
    case Jason.decode(body) do
      {:ok, data} ->
        if Map.get(data, "success") do
          {:ok, data}
        else
          {:error, Map.get(data, "error", "Unknown error")}
        end

      {:error, reason} ->
        {:error, "Failed to decode response: #{inspect(reason)}"}
    end
  end

  defp handle_response(%HTTPoison.Response{status_code: status, body: body}) do
    {:error, "Server error (#{status}): #{body}"}
  end

  defp compute_cache_key(file_path, mime_type, config) do
    content = "#{file_path}|#{mime_type}|#{inspect(config)}"
    :crypto.hash(:sha256, content) |> Base.encode16(case: :lower)
  end

  defp get_from_cache(cache_dir, cache_key) do
    cache_file = Path.join(cache_dir, "#{cache_key}.json")

    if File.exists?(cache_file) do
      case File.read(cache_file) do
        {:ok, content} ->
          {:ok, Jason.decode!(content)}

        :error ->
          :miss
      end
    else
      :miss
    end
  end

  defp store_in_cache(cache_dir, cache_key, result) do
    File.mkdir_p!(cache_dir)
    cache_file = Path.join(cache_dir, "#{cache_key}.json")
    File.write!(cache_file, Jason.encode!(result))
  end
end

# Usage examples
IO.puts("=== Kreuzberg MCP Client ===\n")

# Create client configuration
config = KreuzbergMCPClient.Config.new(
  host: "localhost",
  port: 8080,
  timeout_ms: 30000,
  max_retries: 3,
  cache_dir: "/tmp/kreuzberg_cache"
)

# Check server health
IO.puts("Checking server health...")

case KreuzbergMCPClient.health_check(config) do
  {:ok, health} ->
    IO.puts("Server status: #{health["status"]}")
    IO.puts("Service: #{health["service"]}\n")

  {:error, reason} ->
    IO.puts("Health check failed: #{reason}\n")
end

# Extract single document
IO.puts("Extracting document...")

case KreuzbergMCPClient.extract_file(config, "document.pdf", use_cache: true) do
  {:ok, result} ->
    IO.puts("Success!")
    IO.puts("Content size: #{byte_size(result["content"])} bytes")
    IO.puts("MIME type: #{result["mime_type"]}")
    IO.puts("Tables found: #{length(result["tables"])}")

  {:error, reason} ->
    IO.puts("Extraction failed: #{reason}")
end

IO.puts("")

# Batch extract multiple documents
IO.puts("Batch extracting multiple documents...")

documents = [
  "doc1.pdf",
  "doc2.pdf",
  "doc3.pdf"
]

case KreuzbergMCPClient.batch_extract(config, documents) do
  {:ok, results} ->
    IO.puts("Batch extraction complete!")
    successful = Enum.count(results, &match?({:ok, _}, &1))
    IO.puts("Successful: #{successful}/#{length(results)}")

  {:error, reason} ->
    IO.puts("Batch extraction failed: #{reason}")
end
```
