```elixir title="Elixir"
# Basic CLI Tool - Simple command-line interface for Kreuzberg
# Demonstrates how to build a CLI for document extraction

defmodule KreuzbergCLI do
  @moduledoc """
  Command-line interface for Kreuzberg document extraction.

  Provides a simple, user-friendly CLI for extracting documents
  from the command line with progress feedback.
  """

  require Logger

  @doc """
  Main entry point for CLI application.

  Parses command-line arguments and executes the appropriate command.
  """
  def main(args) do
    args
    |> parse_args()
    |> execute()
  end

  defp parse_args(args) do
    case args do
      [] ->
        {:error, :no_command}

      ["extract", file | rest] ->
        opts = parse_options(rest, %{})
        {:extract, file, opts}

      ["batch", dir | rest] ->
        opts = parse_options(rest, %{})
        {:batch, dir, opts}

      ["help"] ->
        {:help}

      ["-h"] ->
        {:help}

      ["--help"] ->
        {:help}

      [cmd] ->
        {:error, "Unknown command: #{cmd}"}

      _ ->
        {:error, :invalid_args}
    end
  end

  defp parse_options([], acc), do: acc

  defp parse_options(["-v" | rest], acc) do
    parse_options(rest, Map.put(acc, :verbose, true))
  end

  defp parse_options(["--mime-type", mime | rest], acc) do
    parse_options(rest, Map.put(acc, :mime_type, mime))
  end

  defp parse_options(["--output", path | rest], acc) do
    parse_options(rest, Map.put(acc, :output, path))
  end

  defp parse_options(["--ocr" | rest], acc) do
    parse_options(rest, Map.put(acc, :enable_ocr, true))
  end

  defp parse_options(["--chunks" | rest], acc) do
    parse_options(rest, Map.put(acc, :enable_chunks, true))
  end

  defp parse_options([_ | rest], acc) do
    parse_options(rest, acc)
  end

  defp execute({:help}) do
    print_help()
    :ok
  end

  defp execute({:extract, file, opts}) do
    unless File.exists?(file) do
      IO.puts(:stderr, "Error: File not found: #{file}")
      :error
    else
      extract_file(file, opts)
    end
  end

  defp execute({:batch, dir, opts}) do
    unless File.dir?(dir) do
      IO.puts(:stderr, "Error: Directory not found: #{dir}")
      :error
    else
      batch_extract(dir, opts)
    end
  end

  defp execute({:error, reason}) do
    IO.puts(:stderr, "Error: #{inspect(reason)}")
    print_help()
    :error
  end

  defp extract_file(file_path, opts) do
    verbose = Map.get(opts, :verbose, false)
    mime_type = Map.get(opts, :mime_type, nil)
    output_path = Map.get(opts, :output, nil)

    config = build_config(opts)

    IO.puts("Extracting: #{file_path}")
    start_time = System.monotonic_time(:millisecond)

    case Kreuzberg.extract_file(file_path, mime_type, config) do
      {:ok, result} ->
        elapsed = System.monotonic_time(:millisecond) - start_time

        print_extraction_result(result, elapsed, verbose)

        if output_path do
          save_result(result, output_path)
        end

        :ok

      {:error, reason} ->
        IO.puts(:stderr, "Extraction failed: #{inspect(reason)}")
        :error
    end
  end

  defp batch_extract(dir, opts) do
    verbose = Map.get(opts, :verbose, false)
    config = build_config(opts)

    pattern = Path.join(dir, "**/*.{pdf,docx,xlsx,txt,html,md}")
    files = Path.wildcard(pattern)

    case files do
      [] ->
        IO.puts("No documents found in #{dir}")
        :ok

      _ ->
        IO.puts("Found #{length(files)} documents\n")

        results =
          files
          |> Enum.with_index(1)
          |> Enum.map(fn {file, idx} ->
            IO.write("  [#{idx}/#{length(files)}] ")
            start_time = System.monotonic_time(:millisecond)

            case Kreuzberg.extract_file(file, nil, config) do
              {:ok, result} ->
                elapsed = System.monotonic_time(:millisecond) - start_time
                IO.puts("#{Path.basename(file)} (#{elapsed}ms)")
                {:ok, file, result, elapsed}

              {:error, reason} ->
                IO.puts("#{Path.basename(file)} - ERROR")
                if verbose, do: IO.puts("  Error: #{inspect(reason)}")
                {:error, file, reason}
            end
          end)

        print_batch_summary(results)
        :ok
    end
  end

  defp build_config(opts) do
    %Kreuzberg.ExtractionConfig{
      ocr:
        if(Map.get(opts, :enable_ocr),
          do: %{"enabled" => true, "backend" => "tesseract"},
          else: nil
        ),
      chunking:
        if(Map.get(opts, :enable_chunks),
          do: %{"enabled" => true, "max_chars" => 1000, "max_overlap" => 100},
          else: nil
        ),
      use_cache: true
    }
  end

  defp print_extraction_result(result, elapsed_ms, verbose) do
    IO.puts("\nExtraction Results:")
    IO.puts("  Content size: #{byte_size(result.content)} bytes")
    IO.puts("  MIME type: #{result.mime_type}")
    IO.puts("  Processing time: #{elapsed_ms}ms")

    if result.metadata do
      IO.puts("  Metadata keys: #{Enum.count(result.metadata)}")
    end

    if result.tables && !Enum.empty?(result.tables) do
      IO.puts("  Tables found: #{length(result.tables)}")
    end

    if result.images && !Enum.empty?(result.images) do
      IO.puts("  Images found: #{length(result.images)}")
    end

    if result.chunks && !Enum.empty?(result.chunks) do
      IO.puts("  Chunks created: #{length(result.chunks)}")
    end

    if result.detected_languages && !Enum.empty?(result.detected_languages) do
      IO.puts("  Languages: #{Enum.join(result.detected_languages, ", ")}")
    end

    if verbose do
      IO.puts("\n  Full metadata:")
      IO.inspect(result.metadata, pretty: true)
    end

    IO.puts("")
  end

  defp print_batch_summary(results) do
    total = length(results)
    successful = Enum.count(results, &match?({:ok, _, _, _}, &1))
    failed = Enum.count(results, &match?({:error, _, _}, &1))

    total_time =
      results
      |> Enum.filter(&match?({:ok, _, _, _}, &1))
      |> Enum.map(fn {:ok, _, _, time} -> time end)
      |> Enum.sum()

    IO.puts("\nBatch Summary:")
    IO.puts("  Total: #{total}")
    IO.puts("  Successful: #{successful}")
    IO.puts("  Failed: #{failed}")
    IO.puts("  Total time: #{total_time}ms")
    IO.puts("  Average time: #{div(total_time, max(successful, 1))}ms/document")
  end

  defp save_result(result, output_path) do
    output_data = %{
      content: result.content,
      mime_type: result.mime_type,
      metadata: result.metadata,
      tables: result.tables || [],
      images: result.images || [],
      chunks: result.chunks || [],
      detected_languages: result.detected_languages || [],
      extracted_at: DateTime.utc_now()
    }

    case File.write(output_path, Jason.encode!(output_data, pretty: true)) do
      :ok ->
        IO.puts("Results saved to: #{output_path}")

      {:error, reason} ->
        IO.puts(:stderr, "Failed to save results: #{inspect(reason)}")
    end
  end

  defp print_help do
    IO.puts("""
    Kreuzberg CLI - Document Extraction Tool

    USAGE:
      kreuzberg extract <file> [OPTIONS]
      kreuzberg batch <directory> [OPTIONS]
      kreuzberg help

    COMMANDS:
      extract <file>        Extract content from a single document
      batch <directory>     Extract all documents in a directory
      help                  Show this help message

    OPTIONS:
      -v, --verbose         Show detailed output
      --mime-type <type>    Specify MIME type (e.g., application/pdf)
      --output <path>       Save results to JSON file
      --ocr                 Enable OCR for scanned documents
      --chunks              Enable document chunking for RAG

    EXAMPLES:
      kreuzberg extract document.pdf
      kreuzberg extract document.pdf --output results.json
      kreuzberg batch ./documents --ocr --chunks
      kreuzberg batch ./documents -v --output summary.json
    """)
  end
end

# Entry point for escript
def main(args) do
  case KreuzbergCLI.main(args) do
    :ok -> 0
    :error -> 1
  end
end
```
