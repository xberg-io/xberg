```elixir title="Elixir"
# MCP Server Integration - Start a Kreuzberg MCP server
# Demonstrates how to set up and manage an MCP server for remote document extraction

defmodule KreuzbergMCPServer do
  @moduledoc """
  MCP (Model Context Protocol) server for Kreuzberg document extraction.

  Provides a standardized interface for remote clients to extract documents
  using the Kreuzberg library via the Model Context Protocol.
  """

  require Logger
  alias Kreuzberg.ExtractionConfig

  @doc """
  Start the MCP server on the specified host and port.

  The server accepts extraction requests from MCP clients and returns
  structured document data including content, metadata, and extracted elements.

  ## Options

    * `:host` - Server host (default: "127.0.0.1")
    * `:port` - Server port (default: 8080)
    * `:max_connections` - Maximum concurrent connections (default: 10)
  """
  def start_server(opts \\ []) do
    host = Keyword.get(opts, :host, "127.0.0.1")
    port = Keyword.get(opts, :port, 8080)
    max_connections = Keyword.get(opts, :max_connections, 10)

    Logger.info("Starting Kreuzberg MCP server on #{host}:#{port}")

    {:ok, _pid} =
      :cowboy.start_clear(
        :kreuzberg_http,
        [{:port, port}],
        %{
          env: [
            {:dispatch,
             [
               {:_,
                [
                  {"/extract", KreuzbergMCPServer.Handler, []},
                  {"/extract/file", KreuzbergMCPServer.FileHandler, []},
                  {"/health", KreuzbergMCPServer.HealthHandler, []}
                ]}
             ]}
          ]
        }
      )

    Logger.info("MCP server started successfully")
    {:ok, "Server running on #{host}:#{port}"}
  end

  @doc """
  Stop the MCP server gracefully.
  """
  def stop_server do
    Logger.info("Stopping Kreuzberg MCP server")
    :cowboy.stop_listener(:kreuzberg_http)
    Logger.info("MCP server stopped")
    :ok
  end
end

# Handler for extraction requests
defmodule KreuzbergMCPServer.Handler do
  @moduledoc """
  HTTP handler for MCP extraction requests.
  Processes incoming extraction requests with optional configuration.
  """

  require Logger

  def init(req, state) do
    req
    |> handle_request()
    |> reply()
    |> wrap_response(state)
  end

  defp handle_request(req) do
    case req.method do
      "POST" -> handle_extraction(req)
      _ -> error_response(405, "Method not allowed")
    end
  end

  defp handle_extraction(req) do
    case :cowboy_req.read_body(req) do
      {:ok, body, req} ->
        case Jason.decode(body) do
          {:ok, params} ->
            extract_from_params(params, req)

          {:error, reason} ->
            error_response(400, "Invalid JSON: #{inspect(reason)}")
        end

      {:error, reason} ->
        error_response(400, "Failed to read body: #{inspect(reason)}")
    end
  end

  defp extract_from_params(params, req) do
    file_path = Map.get(params, "file_path")
    mime_type = Map.get(params, "mime_type")
    config_opts = Map.get(params, "config", %{})

    unless file_path do
      error_response(400, "Missing required parameter: file_path")
    else
      config = build_config(config_opts)

      case Kreuzberg.extract_file(file_path, mime_type, config) do
        {:ok, result} ->
          response_data = %{
            success: true,
            content: result.content,
            mime_type: result.mime_type,
            metadata: result.metadata || %{},
            tables: result.tables || [],
            chunks: result.chunks || [],
            images: result.images || [],
            detected_languages: result.detected_languages || []
          }

          success_response(200, response_data, req)

        {:error, reason} ->
          error_response(400, "Extraction failed: #{inspect(reason)}")
      end
    end
  end

  defp build_config(opts) when is_map(opts) do
    %Kreuzberg.ExtractionConfig{
      ocr: opts["ocr"],
      chunking: opts["chunking"],
      quality_processing: opts["quality_processing"],
      language_detection: opts["language_detection"],
      images: opts["images"],
      use_cache: Map.get(opts, "use_cache", true)
    }
  end

  defp build_config(_), do: nil

  defp success_response(status, data, req) do
    {:ok,
     :cowboy_req.reply(
       status,
       %{"content-type" => "application/json"},
       Jason.encode!(data),
       req
     )}
  end

  defp error_response(status, message) do
    {:error,
     status,
     Jason.encode!(%{
       success: false,
       error: message
     })}
  end

  defp reply({:ok, req}), do: {req, :ok}
  defp reply({:error, status, body}), do: {status, body}

  defp wrap_response({req, :ok}, state), do: {:ok, req, state}
  defp wrap_response({status, body}, state) do
    # Note: In actual implementation, req needs to be passed through the pipeline
    # For now, create a minimal request object for error responses
    req = :cowboy_req.new()
    {:cowboy_req.reply(status, %{}, body, req), state}
  end
end

# Health check handler
defmodule KreuzbergMCPServer.HealthHandler do
  @moduledoc """
  Health check endpoint for the MCP server.
  """

  def init(req, state) do
    response = Jason.encode!(%{
      status: "healthy",
      service: "kreuzberg-mcp",
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
    })

    req =
      :cowboy_req.reply(
        200,
        %{"content-type" => "application/json"},
        response,
        req
      )

    {:ok, req, state}
  end
end

# File upload handler
defmodule KreuzbergMCPServer.FileHandler do
  @moduledoc """
  Handler for multipart file uploads for extraction.
  """

  require Logger

  def init(req, state) do
    case req.method do
      "POST" -> handle_file_upload(req, state)
      _ -> {:cowboy_req.reply(405, %{}, "Method not allowed", req), state}
    end
  end

  defp handle_file_upload(req, state) do
    # Store uploaded file temporarily
    temp_path = "/tmp/kreuzberg_#{System.unique_integer([:positive])}"

    case :cowboy_req.read_body(req) do
      {:ok, body, req} ->
        File.write!(temp_path, body)

        case Kreuzberg.extract_file(temp_path) do
          {:ok, result} ->
            response = Jason.encode!(%{
              success: true,
              content_size: byte_size(result.content),
              mime_type: result.mime_type,
              metadata: result.metadata
            })

            req =
              :cowboy_req.reply(
                200,
                %{"content-type" => "application/json"},
                response,
                req
              )

            File.rm(temp_path)
            {:ok, req, state}

          {:error, reason} ->
            response = Jason.encode!(%{success: false, error: inspect(reason)})

            req =
              :cowboy_req.reply(
                400,
                %{"content-type" => "application/json"},
                response,
                req
              )

            File.rm(temp_path)
            {:ok, req, state}
        end

      {:error, reason} ->
        response = Jason.encode!(%{success: false, error: inspect(reason)})

        req =
          :cowboy_req.reply(
            400,
            %{"content-type" => "application/json"},
            response,
            req
          )

        {:ok, req, state}
    end
  end
end

# Usage example - start the server
IO.puts("=== Kreuzberg MCP Server ===\n")

case KreuzbergMCPServer.start_server(port: 8080) do
  {:ok, message} ->
    IO.puts(message)
    IO.puts("\nServer is running and ready to accept requests:")
    IO.puts("  - POST /extract - Extract from file path")
    IO.puts("  - POST /extract/file - Upload and extract")
    IO.puts("  - GET /health - Health check")

    # Keep the server running
    IO.puts("\nServer started. Press Ctrl+C to stop.")
    Process.sleep(:infinity)

  {:error, reason} ->
    IO.puts("Failed to start server: #{inspect(reason)}")
end
```
