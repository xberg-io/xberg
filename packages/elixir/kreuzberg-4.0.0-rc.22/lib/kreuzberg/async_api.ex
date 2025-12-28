defmodule Kreuzberg.AsyncAPI do
  @moduledoc """
  Asynchronous extraction operations using Elixir Tasks.

  This module provides Task-based async wrappers for all extraction operations,
  allowing concurrent document processing. Each function returns a Task that can be
  awaited using `Task.await/2` to retrieve the extraction result.

  This is useful for:
  - Processing multiple documents concurrently
  - Avoiding blocking operations in web requests
  - Building pipelines of extraction operations
  - Handling long-running extractions in background workers

  ## Task-Based Approach

  Each async function wraps the corresponding synchronous operation in `Task.async/1`,
  which schedules the work on the default task supervisor. Users have full control
  over task lifecycle using standard Task functions like `Task.await/2` and
  `Task.await_many/2`.

  ## Examples

      # Extract a single document asynchronously
      task = Kreuzberg.AsyncAPI.extract_async(pdf_binary, "application/pdf")
      {:ok, result} = Task.await(task)

      # Extract multiple documents concurrently
      tasks = [
        Kreuzberg.AsyncAPI.extract_async(pdf1, "application/pdf"),
        Kreuzberg.AsyncAPI.extract_async(pdf2, "application/pdf"),
        Kreuzberg.AsyncAPI.extract_async(pdf3, "application/pdf")
      ]
      results = Task.await_many(tasks)

      # Extract files concurrently
      tasks = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
        |> Enum.map(&Kreuzberg.AsyncAPI.extract_file_async/1)
      {:ok, results} = Task.await_many(tasks)
               |> Enum.reduce({:ok, []}, fn
                 {:ok, result}, {:ok, acc} -> {:ok, [result | acc]}
                 {:error, reason}, _acc -> {:error, reason}
               end)

      # Batch extract with configuration
      files = ["file1.pdf", "file2.pdf"]
      config = %Kreuzberg.ExtractionConfig{extract_images: true}
      task = Kreuzberg.AsyncAPI.batch_extract_files_async(files, "application/pdf", config)
      {:ok, results} = Task.await(task)

  ## Return Values

  All functions return a Task that will contain:
  - `{:ok, result}` - Successful extraction
  - `{:error, reason}` - Extraction failure with error message

  Users can handle errors by pattern matching on the awaited result.

  ## Configuration

  All async functions accept the same configuration options as their synchronous
  counterparts:
  - `Kreuzberg.ExtractionConfig` struct
  - Plain map with string keys
  - Keyword list
  - `nil` (uses defaults)

  ## Comparison with Synchronous API

  | Operation | Sync | Async |
  |-----------|------|-------|
  | Extract binary | `Kreuzberg.extract/2-3` | `extract_async/2-3` |
  | Extract file | `Kreuzberg.extract_file/2-3` | `extract_file_async/2-3` |
  | Batch extract files | `Kreuzberg.batch_extract_files/2-3` | `batch_extract_files_async/2-3` |
  | Batch extract bytes | `Kreuzberg.batch_extract_bytes/3-4` | `batch_extract_bytes_async/3-4` |

  ## Implementation Notes

  - Tasks are scheduled on the default task supervisor
  - No custom error handling is performed; errors are propagated as-is
  - Configuration validation happens when the task is awaited
  - Each async function is thread-safe and can be called from any process
  """

  alias Kreuzberg.{ExtractionConfig, ExtractionResult}

  @doc """
  Extract content from binary data asynchronously.

  Returns a Task that will perform the extraction concurrently. The task can be
  awaited using `Task.await/2` to retrieve the result.

  ## Parameters

    * `input` - Binary data to extract from
    * `mime_type` - MIME type of the data (e.g., "application/pdf")
    * `config` - Optional ExtractionConfig struct, map, or keyword list (defaults to nil)

  ## Returns

    * A Task that will resolve to `{:ok, ExtractionResult.t()}` or `{:error, String.t()}`

  ## Examples

      # Extract a PDF asynchronously
      task = Kreuzberg.AsyncAPI.extract_async(pdf_binary, "application/pdf")
      {:ok, result} = Task.await(task)
      result.content

      # With configuration
      config = %Kreuzberg.ExtractionConfig{extract_images: true}
      task = Kreuzberg.AsyncAPI.extract_async(pdf_binary, "application/pdf", config)
      {:ok, result} = Task.await(task)

      # Using keyword list configuration
      task = Kreuzberg.AsyncAPI.extract_async(
        data,
        "application/pdf",
        ocr: %{"enabled" => true}
      )
      {:ok, result} = Task.await(task)
  """
  @spec extract_async(
          binary(),
          String.t(),
          ExtractionConfig.t() | map() | keyword() | nil
        ) :: Task.t({:ok, ExtractionResult.t()} | {:error, String.t()})
  def extract_async(input, mime_type, config \\ nil) do
    Task.async(fn ->
      Kreuzberg.extract(input, mime_type, config)
    end)
  end

  @doc """
  Extract content from a file asynchronously.

  Returns a Task that will perform the file extraction concurrently. The task can be
  awaited using `Task.await/2` to retrieve the result.

  The MIME type can be explicitly provided or automatically detected from the file
  extension if not specified.

  ## Parameters

    * `path` - File path as String or Path.t()
    * `mime_type` - Optional MIME type (defaults to nil for auto-detection)
    * `config` - Optional ExtractionConfig struct, map, or keyword list (defaults to nil)

  ## Returns

    * A Task that will resolve to `{:ok, ExtractionResult.t()}` or `{:error, String.t()}`

  ## Examples

      # Extract a file asynchronously
      task = Kreuzberg.AsyncAPI.extract_file_async("document.pdf", "application/pdf")
      {:ok, result} = Task.await(task)

      # Extract with auto-detection
      task = Kreuzberg.AsyncAPI.extract_file_async("document.pdf")
      {:ok, result} = Task.await(task)

      # With configuration
      config = %Kreuzberg.ExtractionConfig{force_ocr: true}
      task = Kreuzberg.AsyncAPI.extract_file_async(
        "document.pdf",
        "application/pdf",
        config
      )
      {:ok, result} = Task.await(task)

      # Extract multiple files concurrently
      tasks = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
        |> Enum.map(&Kreuzberg.AsyncAPI.extract_file_async/1)
      results = Task.await_many(tasks)
  """
  @spec extract_file_async(
          String.t() | Path.t(),
          String.t() | nil,
          ExtractionConfig.t() | map() | keyword() | nil
        ) :: Task.t({:ok, ExtractionResult.t()} | {:error, String.t()})
  def extract_file_async(path, mime_type \\ nil, config \\ nil) do
    Task.async(fn ->
      Kreuzberg.extract_file(path, mime_type, config)
    end)
  end

  @doc """
  Batch extract content from multiple files asynchronously.

  Returns a Task that will perform batch file extraction concurrently. The task can be
  awaited using `Task.await/2` to retrieve a list of extraction results.

  Batch operations can be more efficient than processing files individually when
  dealing with large numbers of documents.

  ## Parameters

    * `paths` - List of file paths (String or Path.t() values)
    * `mime_type` - Optional MIME type for all files (defaults to nil for auto-detection)
    * `config` - Optional ExtractionConfig struct, map, or keyword list (defaults to nil)

  ## Returns

    * A Task that will resolve to `{:ok, [ExtractionResult.t()]}` or `{:error, String.t()}`

  ## Examples

      # Batch extract multiple files asynchronously
      paths = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]
      task = Kreuzberg.AsyncAPI.batch_extract_files_async(paths, "application/pdf")
      {:ok, results} = Task.await(task)
      Enum.map(results, & &1.content)

      # With configuration
      config = %Kreuzberg.ExtractionConfig{extract_images: true}
      task = Kreuzberg.AsyncAPI.batch_extract_files_async(
        ["file1.pdf", "file2.pdf"],
        "application/pdf",
        config
      )
      {:ok, results} = Task.await(task)

      # Auto-detect MIME types
      task = Kreuzberg.AsyncAPI.batch_extract_files_async(
        ["file1.pdf", "file2.txt", "file3.docx"]
      )
      {:ok, results} = Task.await(task)
  """
  @spec batch_extract_files_async(
          [String.t() | Path.t()],
          String.t() | nil,
          ExtractionConfig.t() | map() | keyword() | nil
        ) :: Task.t({:ok, [ExtractionResult.t()]} | {:error, String.t()})
  def batch_extract_files_async(paths, mime_type \\ nil, config \\ nil) do
    Task.async(fn ->
      Kreuzberg.BatchAPI.batch_extract_files(paths, mime_type, config)
    end)
  end

  @doc """
  Batch extract content from multiple binary inputs asynchronously.

  Returns a Task that will perform batch binary extraction concurrently. The task can be
  awaited using `Task.await/2` to retrieve a list of extraction results.

  MIME types can be provided as a single type for all inputs or as a list with one
  MIME type per input.

  ## Parameters

    * `data_list` - List of binary data inputs
    * `mime_types` - Single MIME type string (applied to all inputs) or list of MIME types
    * `config` - Optional ExtractionConfig struct, map, or keyword list (defaults to nil)

  ## Returns

    * A Task that will resolve to `{:ok, [ExtractionResult.t()]}` or `{:error, String.t()}`

  ## Examples

      # Batch extract multiple PDFs from binary data
      data_list = [pdf1_binary, pdf2_binary, pdf3_binary]
      task = Kreuzberg.AsyncAPI.batch_extract_bytes_async(
        data_list,
        "application/pdf"
      )
      {:ok, results} = Task.await(task)

      # With different MIME types for each input
      data_list = [pdf_binary, docx_binary, txt_binary]
      mime_types = ["application/pdf", "application/vnd.openxmlformats-officedocument.wordprocessingml.document", "text/plain"]
      task = Kreuzberg.AsyncAPI.batch_extract_bytes_async(data_list, mime_types)
      {:ok, results} = Task.await(task)

      # With configuration
      config = %Kreuzberg.ExtractionConfig{ocr: %{"enabled" => true}}
      task = Kreuzberg.AsyncAPI.batch_extract_bytes_async(
        [pdf1, pdf2],
        "application/pdf",
        config
      )
      {:ok, results} = Task.await(task)
  """
  @spec batch_extract_bytes_async(
          [binary()],
          String.t() | [String.t()],
          ExtractionConfig.t() | map() | keyword() | nil
        ) :: Task.t({:ok, [ExtractionResult.t()]} | {:error, String.t()})
  def batch_extract_bytes_async(data_list, mime_types, config \\ nil) do
    Task.async(fn ->
      Kreuzberg.BatchAPI.batch_extract_bytes(data_list, mime_types, config)
    end)
  end
end
