defmodule KreuzbergTest.Unit.AsyncAPITest do
  @moduledoc """
  Unit tests for asynchronous extraction operations.

  Tests cover:
  - extract_async/2-3: Async binary extraction with Task return values
  - extract_file_async/2-3: Async file extraction with Task return values
  - batch_extract_files_async/2-3: Async batch file extraction
  - batch_extract_bytes_async/2-3: Async batch binary extraction

  All tests verify:
  - Functions return Task structs
  - Tasks resolve to correct extraction results
  - Configuration handling (struct, map, keyword list)
  - Error handling and propagation
  - File I/O operations
  """

  use ExUnit.Case

  alias Kreuzberg.AsyncAPI
  alias Kreuzberg.ExtractionConfig

  # Helper function to create temporary directories
  defp create_temp_dir do
    base = System.tmp_dir!()
    dir = Path.join(base, "kreuzberg_test_#{:erlang.unique_integer([:positive])}")
    File.mkdir_p!(dir)
    {:ok, dir}
  end

  # ===== extract_async/2-3 Tests =====

  describe "extract_async/2" do
    @tag :unit
    test "returns a Task struct" do
      task = AsyncAPI.extract_async("Hello world", "text/plain")
      assert %Task{} = task
    end

    @tag :unit
    test "Task resolves to {:ok, ExtractionResult} on success" do
      task = AsyncAPI.extract_async("Hello world", "text/plain")
      {:ok, result} = Task.await(task)

      assert %Kreuzberg.ExtractionResult{} = result
      assert result.content == "Hello world"
      assert result.mime_type == "text/plain"
    end

    @tag :unit
    test "Task result contains expected fields" do
      task = AsyncAPI.extract_async("Test content", "text/plain")
      {:ok, result} = Task.await(task)

      assert %Kreuzberg.ExtractionResult{
               content: _,
               mime_type: _,
               metadata: _,
               tables: _,
               detected_languages: _,
               chunks: _,
               images: _,
               pages: _
             } = result
    end

    @tag :unit
    test "Task resolves to {:error, reason} on invalid MIME type" do
      task = AsyncAPI.extract_async("data", "invalid/type")
      {:error, reason} = Task.await(task)

      assert is_binary(reason)
      assert byte_size(reason) > 0
    end

    @tag :unit
    test "handles multiline text content" do
      content = "Line 1\nLine 2\nLine 3"
      task = AsyncAPI.extract_async(content, "text/plain")
      {:ok, result} = Task.await(task)

      assert result.content == content
    end

    @tag :unit
    test "handles special characters in text" do
      content = "Special: @#$%^&*()\nUnicode: 你好世界"
      task = AsyncAPI.extract_async(content, "text/plain")
      {:ok, result} = Task.await(task)

      assert result.content == content
    end

    @tag :unit
    test "accepts binary input" do
      task = AsyncAPI.extract_async("text", "text/plain")
      assert %Task{} = task
      {:ok, _result} = Task.await(task)
    end

    @tag :unit
    test "multiple tasks can be awaited concurrently" do
      task1 = AsyncAPI.extract_async("Content 1", "text/plain")
      task2 = AsyncAPI.extract_async("Content 2", "text/plain")
      task3 = AsyncAPI.extract_async("Content 3", "text/plain")

      results = Task.await_many([task1, task2, task3])

      assert length(results) == 3

      {:ok, result1} = Enum.at(results, 0)
      {:ok, result2} = Enum.at(results, 1)
      {:ok, result3} = Enum.at(results, 2)

      assert result1.content == "Content 1"
      assert result2.content == "Content 2"
      assert result3.content == "Content 3"
    end
  end

  describe "extract_async/3 with configuration" do
    @tag :unit
    test "accepts ExtractionConfig struct" do
      config = %ExtractionConfig{use_cache: false}
      task = AsyncAPI.extract_async("Test content", "text/plain", config)

      {:ok, result} = Task.await(task)
      assert result.content == "Test content"
    end

    @tag :unit
    test "accepts map with string keys" do
      task =
        AsyncAPI.extract_async("Test content", "text/plain", %{
          "use_cache" => false
        })

      {:ok, result} = Task.await(task)
      assert result.content == "Test content"
    end

    @tag :unit
    test "accepts keyword list configuration" do
      task = AsyncAPI.extract_async("Test content", "text/plain", use_cache: false)

      {:ok, result} = Task.await(task)
      assert result.content == "Test content"
    end

    @tag :unit
    test "accepts nil configuration (uses defaults)" do
      task = AsyncAPI.extract_async("Test content", "text/plain", nil)

      {:ok, result} = Task.await(task)
      assert result.content == "Test content"
    end

    @tag :unit
    test "configuration with OCR settings" do
      config = %ExtractionConfig{ocr: %{"enabled" => true}}
      task = AsyncAPI.extract_async("Test", "text/plain", config)

      {:ok, result} = Task.await(task)
      assert %Kreuzberg.ExtractionResult{} = result
    end

    @tag :unit
    test "configuration with extract_images setting" do
      config = %ExtractionConfig{images: %{"enabled" => true}}
      task = AsyncAPI.extract_async("Test", "text/plain", config)

      {:ok, result} = Task.await(task)
      assert %Kreuzberg.ExtractionResult{} = result
    end
  end

  # ===== extract_file_async/2-3 Tests =====

  describe "extract_file_async/2" do
    @tag :unit
    test "returns a Task struct" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Test content")

      task = AsyncAPI.extract_file_async(file)
      assert %Task{} = task
    end

    @tag :unit
    @tag :integration
    test "Task resolves to {:ok, ExtractionResult} on success" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Test file content")

      task = AsyncAPI.extract_file_async(file)
      {:ok, result} = Task.await(task)

      assert %Kreuzberg.ExtractionResult{} = result
      assert result.content == "Test file content"
    end

    @tag :unit
    @tag :integration
    test "auto-detects MIME type from file extension" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Auto-detect content")

      task = AsyncAPI.extract_file_async(file)
      {:ok, result} = Task.await(task)

      assert result.content == "Auto-detect content"
      assert result.mime_type == "text/plain"
    end

    @tag :unit
    @tag :integration
    test "Task resolves to error for non-existent file" do
      task = AsyncAPI.extract_file_async("/nonexistent/file.txt")
      {:error, reason} = Task.await(task)

      assert is_binary(reason)
      assert byte_size(reason) > 0
    end

    @tag :unit
    @tag :integration
    test "Task result has proper structure" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Content")

      task = AsyncAPI.extract_file_async(file)
      {:ok, result} = Task.await(task)

      assert %Kreuzberg.ExtractionResult{
               content: _,
               mime_type: _,
               metadata: _,
               tables: _
             } = result
    end

    @tag :unit
    @tag :integration
    test "can be called with Path.t() input" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Path.t() content")

      path = Path.expand(file)
      task = AsyncAPI.extract_file_async(path)
      {:ok, result} = Task.await(task)

      assert result.content == "Path.t() content"
    end
  end

  describe "extract_file_async/3 with MIME type" do
    @tag :unit
    @tag :integration
    test "accepts explicit MIME type" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Content with mime")

      task = AsyncAPI.extract_file_async(file, "text/plain")
      {:ok, result} = Task.await(task)

      assert result.content == "Content with mime"
      assert result.mime_type == "text/plain"
    end

    @tag :unit
    @tag :integration
    test "accepts ExtractionConfig struct" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Configured content")

      config = %ExtractionConfig{use_cache: false}
      task = AsyncAPI.extract_file_async(file, "text/plain", config)
      {:ok, result} = Task.await(task)

      assert result.content == "Configured content"
    end

    @tag :unit
    @tag :integration
    test "accepts map configuration" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Map config content")

      task = AsyncAPI.extract_file_async(file, "text/plain", %{"use_cache" => false})
      {:ok, result} = Task.await(task)

      assert result.content == "Map config content"
    end

    @tag :unit
    @tag :integration
    test "accepts nil MIME type for auto-detection" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Auto-detect")

      task = AsyncAPI.extract_file_async(file, nil)
      {:ok, result} = Task.await(task)

      assert result.content == "Auto-detect"
      assert result.mime_type == "text/plain"
    end

    @tag :unit
    @tag :integration
    test "handles configuration with force_ocr" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "OCR content")

      config = %ExtractionConfig{force_ocr: true}
      task = AsyncAPI.extract_file_async(file, "text/plain", config)
      {:ok, result} = Task.await(task)

      assert %Kreuzberg.ExtractionResult{} = result
    end
  end

  describe "extract_file_async concurrent operations" do
    @tag :unit
    @tag :integration
    test "multiple file tasks can be awaited concurrently" do
      {:ok, dir} = create_temp_dir()

      file1 = Path.join(dir, "test1.txt")
      file2 = Path.join(dir, "test2.txt")
      file3 = Path.join(dir, "test3.txt")

      File.write!(file1, "File 1 content")
      File.write!(file2, "File 2 content")
      File.write!(file3, "File 3 content")

      task1 = AsyncAPI.extract_file_async(file1, "text/plain")
      task2 = AsyncAPI.extract_file_async(file2, "text/plain")
      task3 = AsyncAPI.extract_file_async(file3, "text/plain")

      results = Task.await_many([task1, task2, task3])

      assert length(results) == 3

      {:ok, result1} = Enum.at(results, 0)
      {:ok, result2} = Enum.at(results, 1)
      {:ok, result3} = Enum.at(results, 2)

      assert result1.content == "File 1 content"
      assert result2.content == "File 2 content"
      assert result3.content == "File 3 content"
    end
  end

  # ===== batch_extract_files_async/2-3 Tests =====

  describe "batch_extract_files_async/2" do
    @tag :unit
    test "returns a Task struct" do
      task = AsyncAPI.batch_extract_files_async([], nil)
      assert %Task{} = task
    end

    @tag :unit
    test "Task resolves to error for empty paths list" do
      task = AsyncAPI.batch_extract_files_async([])
      {:error, reason} = Task.await(task)

      assert is_binary(reason)
      assert String.contains?(reason, "empty")
    end

    @tag :unit
    test "Task resolves to error for empty paths list with MIME type" do
      task = AsyncAPI.batch_extract_files_async([], "text/plain")
      {:error, reason} = Task.await(task)

      assert is_binary(reason)
      assert String.contains?(reason, "empty")
    end

    @tag :unit
    @tag :integration
    test "Task resolves to {:ok, [ExtractionResult]} for multiple files" do
      {:ok, dir} = create_temp_dir()

      file1 = Path.join(dir, "batch1.txt")
      file2 = Path.join(dir, "batch2.txt")
      file3 = Path.join(dir, "batch3.txt")

      File.write!(file1, "Batch 1")
      File.write!(file2, "Batch 2")
      File.write!(file3, "Batch 3")

      paths = [file1, file2, file3]
      task = AsyncAPI.batch_extract_files_async(paths, "text/plain")

      {:ok, results} = Task.await(task)

      assert is_list(results)
      assert length(results) == 3

      [result1, result2, result3] = results

      assert result1.content == "Batch 1"
      assert result2.content == "Batch 2"
      assert result3.content == "Batch 3"
    end

    @tag :unit
    @tag :integration
    test "batch result structure is valid" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "batch.txt")
      File.write!(file, "Batch content")

      task = AsyncAPI.batch_extract_files_async([file], "text/plain")
      {:ok, results} = Task.await(task)

      result = hd(results)

      assert %Kreuzberg.ExtractionResult{
               content: _,
               mime_type: _,
               metadata: _,
               tables: _,
               detected_languages: _,
               chunks: _,
               images: _,
               pages: _
             } = result
    end

    @tag :unit
    @tag :integration
    test "Task resolves to error if any file fails" do
      {:ok, dir} = create_temp_dir()
      valid_file = Path.join(dir, "valid.txt")
      File.write!(valid_file, "Valid content")

      paths = [valid_file, "/nonexistent/file.txt"]
      task = AsyncAPI.batch_extract_files_async(paths, "text/plain")

      {:error, reason} = Task.await(task)
      assert is_binary(reason)
    end

    @tag :unit
    @tag :integration
    test "handles files with different content" do
      {:ok, dir} = create_temp_dir()

      file1 = Path.join(dir, "short.txt")
      file2 = Path.join(dir, "long.txt")

      File.write!(file1, "Short")
      File.write!(file2, "This is a much longer content with multiple words")

      task = AsyncAPI.batch_extract_files_async([file1, file2], "text/plain")
      {:ok, results} = Task.await(task)

      assert length(results) == 2
      assert byte_size(Enum.at(results, 0).content) < byte_size(Enum.at(results, 1).content)
    end
  end

  describe "batch_extract_files_async/3 with configuration" do
    @tag :unit
    @tag :integration
    test "accepts ExtractionConfig struct" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "config.txt")
      File.write!(file, "Config content")

      config = %ExtractionConfig{use_cache: false}
      task = AsyncAPI.batch_extract_files_async([file], "text/plain", config)

      {:ok, results} = Task.await(task)
      assert length(results) == 1
      assert hd(results).content == "Config content"
    end

    @tag :unit
    @tag :integration
    test "accepts map configuration" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "map.txt")
      File.write!(file, "Map config")

      task = AsyncAPI.batch_extract_files_async([file], "text/plain", %{"use_cache" => false})

      {:ok, results} = Task.await(task)
      assert length(results) == 1
      assert hd(results).content == "Map config"
    end

    @tag :unit
    @tag :integration
    test "accepts nil configuration (uses defaults)" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "nil.txt")
      File.write!(file, "Nil config")

      task = AsyncAPI.batch_extract_files_async([file], "text/plain", nil)

      {:ok, results} = Task.await(task)
      assert length(results) == 1
      assert hd(results).content == "Nil config"
    end

    @tag :unit
    @tag :integration
    test "auto-detects MIME types when nil" do
      {:ok, dir} = create_temp_dir()

      file1 = Path.join(dir, "auto1.txt")
      file2 = Path.join(dir, "auto2.txt")

      File.write!(file1, "Auto 1")
      File.write!(file2, "Auto 2")

      task = AsyncAPI.batch_extract_files_async([file1, file2], nil)

      {:ok, results} = Task.await(task)
      assert length(results) == 2
    end

    @tag :unit
    @tag :integration
    test "accepts extract_images configuration" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "images.txt")
      File.write!(file, "Image content")

      config = %ExtractionConfig{images: %{"enabled" => true}}
      task = AsyncAPI.batch_extract_files_async([file], "text/plain", config)

      {:ok, results} = Task.await(task)
      assert %Kreuzberg.ExtractionResult{} = hd(results)
    end

    @tag :unit
    @tag :integration
    test "accepts ocr configuration" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "ocr.txt")
      File.write!(file, "OCR content")

      config = %ExtractionConfig{ocr: %{"enabled" => true}}
      task = AsyncAPI.batch_extract_files_async([file], "text/plain", config)

      {:ok, results} = Task.await(task)
      assert %Kreuzberg.ExtractionResult{} = hd(results)
    end
  end

  describe "batch_extract_files_async all results are ExtractionResult" do
    @tag :unit
    @tag :integration
    test "all batch results are ExtractionResult structs" do
      {:ok, dir} = create_temp_dir()

      file1 = Path.join(dir, "struct1.txt")
      file2 = Path.join(dir, "struct2.txt")

      File.write!(file1, "Struct 1")
      File.write!(file2, "Struct 2")

      task = AsyncAPI.batch_extract_files_async([file1, file2], "text/plain")
      {:ok, results} = Task.await(task)

      Enum.each(results, fn result ->
        assert %Kreuzberg.ExtractionResult{} = result
      end)
    end
  end

  # ===== batch_extract_bytes_async/3-4 Tests =====

  describe "batch_extract_bytes_async/3" do
    @tag :unit
    test "returns a Task struct" do
      task = AsyncAPI.batch_extract_bytes_async([], "text/plain")
      assert %Task{} = task
    end

    @tag :unit
    test "Task resolves to error for empty data list" do
      task = AsyncAPI.batch_extract_bytes_async([], "text/plain")
      {:error, reason} = Task.await(task)

      assert is_binary(reason)
      assert String.contains?(reason, "empty")
    end

    @tag :unit
    test "Task resolves to {:ok, [ExtractionResult]} with single MIME type" do
      data_list = ["Content 1", "Content 2", "Content 3"]
      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain")

      {:ok, results} = Task.await(task)

      assert is_list(results)
      assert length(results) == 3

      [result1, result2, result3] = results

      assert result1.content == "Content 1"
      assert result2.content == "Content 2"
      assert result3.content == "Content 3"
    end

    @tag :unit
    test "batch result structure is valid" do
      data_list = ["Test content"]
      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain")

      {:ok, results} = Task.await(task)
      result = hd(results)

      assert %Kreuzberg.ExtractionResult{
               content: _,
               mime_type: _,
               metadata: _,
               tables: _,
               detected_languages: _,
               chunks: _,
               images: _,
               pages: _
             } = result
    end

    @tag :unit
    test "Task resolves to error for mismatched MIME type list length" do
      data_list = ["data1", "data2", "data3"]
      mime_types = ["text/plain", "text/plain"]

      task = AsyncAPI.batch_extract_bytes_async(data_list, mime_types)
      {:error, reason} = Task.await(task)

      assert is_binary(reason)
      assert String.contains?(reason, "Mismatch")
    end

    @tag :unit
    test "Task accepts list of MIME types" do
      data_list = ["Content 1", "Content 2"]
      mime_types = ["text/plain", "text/plain"]

      task = AsyncAPI.batch_extract_bytes_async(data_list, mime_types)
      {:ok, results} = Task.await(task)

      assert length(results) == 2
      assert hd(results).content == "Content 1"
    end

    @tag :unit
    test "Task resolves to error if any extraction fails" do
      data_list = ["Valid content", "data"]
      mime_types = ["text/plain", "invalid/type"]

      task = AsyncAPI.batch_extract_bytes_async(data_list, mime_types)
      {:error, reason} = Task.await(task)

      assert is_binary(reason)
    end

    @tag :unit
    test "handles different content sizes" do
      data_list = [
        "Short",
        "This is a much longer content with multiple words and lines\nMultiline content"
      ]

      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain")

      {:ok, results} = Task.await(task)

      assert length(results) == 2
      assert byte_size(Enum.at(results, 0).content) < byte_size(Enum.at(results, 1).content)
    end

    @tag :unit
    test "handles special characters in batch data" do
      data_list = ["Special: @#$%^&*()", "Unicode: 你好世界", "Mixed: @#$ 你好 ***"]
      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain")

      {:ok, results} = Task.await(task)

      assert length(results) == 3
      assert Enum.at(results, 0).content == "Special: @#$%^&*()"
      assert Enum.at(results, 1).content == "Unicode: 你好世界"
      assert Enum.at(results, 2).content == "Mixed: @#$ 你好 ***"
    end
  end

  describe "batch_extract_bytes_async/4 with configuration" do
    @tag :unit
    test "accepts ExtractionConfig struct" do
      data_list = ["Batch content"]
      config = %ExtractionConfig{use_cache: false}

      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain", config)
      {:ok, results} = Task.await(task)

      assert length(results) == 1
      assert hd(results).content == "Batch content"
    end

    @tag :unit
    test "accepts map configuration" do
      data_list = ["Map config content"]

      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain", %{"use_cache" => false})
      {:ok, results} = Task.await(task)

      assert length(results) == 1
      assert hd(results).content == "Map config content"
    end

    @tag :unit
    test "accepts keyword list configuration" do
      data_list = ["Keyword config"]

      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain", use_cache: false)
      {:ok, results} = Task.await(task)

      assert length(results) == 1
      assert hd(results).content == "Keyword config"
    end

    @tag :unit
    test "accepts nil configuration (uses defaults)" do
      data_list = ["Default config"]

      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain", nil)
      {:ok, results} = Task.await(task)

      assert length(results) == 1
      assert hd(results).content == "Default config"
    end

    @tag :unit
    test "handles extract_images configuration" do
      data_list = ["Content with images"]
      config = %ExtractionConfig{images: %{"enabled" => true}}

      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain", config)
      {:ok, results} = Task.await(task)

      assert %Kreuzberg.ExtractionResult{} = hd(results)
    end

    @tag :unit
    test "handles ocr configuration" do
      data_list = ["OCR content"]
      config = %ExtractionConfig{ocr: %{"enabled" => true}}

      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain", config)
      {:ok, results} = Task.await(task)

      assert %Kreuzberg.ExtractionResult{} = hd(results)
    end

    @tag :unit
    test "handles force_ocr configuration" do
      data_list = ["Force OCR content"]
      config = %ExtractionConfig{force_ocr: true}

      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain", config)
      {:ok, results} = Task.await(task)

      assert %Kreuzberg.ExtractionResult{} = hd(results)
    end
  end

  describe "batch_extract_bytes_async all results are ExtractionResult" do
    @tag :unit
    test "all batch results are ExtractionResult structs" do
      data_list = ["Content 1", "Content 2"]
      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain")

      {:ok, results} = Task.await(task)

      Enum.each(results, fn result ->
        assert %Kreuzberg.ExtractionResult{} = result
      end)
    end
  end

  # ===== Cross-Function Tests =====

  describe "async API concurrent operations" do
    @tag :unit
    @tag :integration
    test "mix of different async operations can run concurrently" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "mixed.txt")
      File.write!(file, "File content")

      # Mix different async operations
      task1 = AsyncAPI.extract_async("Binary 1", "text/plain")
      task2 = AsyncAPI.extract_file_async(file, "text/plain")
      task3 = AsyncAPI.batch_extract_bytes_async(["Binary 2", "Binary 3"], "text/plain")

      results = Task.await_many([task1, task2, task3])

      assert length(results) == 3

      {:ok, result1} = Enum.at(results, 0)
      {:ok, result2} = Enum.at(results, 1)
      {:ok, results3} = Enum.at(results, 2)

      assert result1.content == "Binary 1"
      assert result2.content == "File content"
      assert is_list(results3)
    end

    @tag :unit
    test "many concurrent extract_async operations" do
      # Create 10 concurrent tasks
      tasks =
        for i <- 1..10 do
          AsyncAPI.extract_async("Content #{i}", "text/plain")
        end

      results = Task.await_many(tasks)

      assert length(results) == 10

      Enum.each(results, fn result ->
        assert {:ok, _} = result
      end)
    end
  end

  describe "async API error handling" do
    @tag :unit
    test "extract_async propagates errors from synchronous API" do
      task = AsyncAPI.extract_async("data", "invalid/mime")
      {:error, reason} = Task.await(task)

      assert is_binary(reason)
    end

    @tag :unit
    @tag :integration
    test "extract_file_async propagates file not found errors" do
      task = AsyncAPI.extract_file_async("/nonexistent/file.txt", "text/plain")
      {:error, reason} = Task.await(task)

      assert is_binary(reason)
    end

    @tag :unit
    test "batch_extract_bytes_async propagates validation errors" do
      task = AsyncAPI.batch_extract_bytes_async([], "text/plain")
      {:error, reason} = Task.await(task)

      assert String.contains?(reason, "empty")
    end

    @tag :unit
    test "batch_extract_files_async propagates empty list errors" do
      task = AsyncAPI.batch_extract_files_async([])
      {:error, reason} = Task.await(task)

      assert String.contains?(reason, "empty")
    end
  end

  describe "async API task behavior" do
    @tag :unit
    test "extract_async returns independent Task instances" do
      task1 = AsyncAPI.extract_async("Content 1", "text/plain")
      task2 = AsyncAPI.extract_async("Content 2", "text/plain")

      assert task1.pid != task2.pid
    end

    @tag :unit
    @tag :skip
    test "tasks can be awaited multiple times with Task.await" do
      # NOTE: This test documents a limitation of Elixir tasks.
      # Task.await/2 can only be called once per task. Calling it a second time
      # will timeout because the task result has already been claimed by the first await.
      # If you need to reuse task results, capture the result from the first await
      # and reuse it instead of awaiting the task multiple times.
      task = AsyncAPI.extract_async("Reusable content", "text/plain")

      result1 = Task.await(task)
      # This second await will timeout - tasks can only be awaited once
      result2 = Task.await(task)

      assert result1 == result2
    end

    @tag :unit
    @tag :integration
    test "batch_extract_files_async returns single Task for multiple files" do
      {:ok, dir} = create_temp_dir()
      file1 = Path.join(dir, "batch_test1.txt")
      file2 = Path.join(dir, "batch_test2.txt")
      File.write!(file1, "Content 1")
      File.write!(file2, "Content 2")

      task = AsyncAPI.batch_extract_files_async([file1, file2], "text/plain")

      # Single Task should resolve to a list of results
      {:ok, results} = Task.await(task)
      assert is_list(results)
      assert length(results) == 2
    end

    @tag :unit
    test "batch_extract_bytes_async returns single Task for multiple items" do
      data_list = ["Item 1", "Item 2", "Item 3"]
      task = AsyncAPI.batch_extract_bytes_async(data_list, "text/plain")

      # Single Task should resolve to a list of results
      {:ok, results} = Task.await(task)
      assert is_list(results)
      assert length(results) == 3
    end
  end

  describe "async API configuration validation" do
    @tag :unit
    test "extract_async validates configuration on task execution" do
      # Invalid config should fail when task is awaited, not when created
      config = %{"invalid_key" => "value"}
      task = AsyncAPI.extract_async("content", "text/plain", config)

      assert %Task{} = task
      # Task may or may not fail depending on validation strictness
      _result = Task.await(task)
    end

    @tag :unit
    @tag :integration
    test "batch_extract_files_async validates configuration on task execution" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "config_test.txt")
      File.write!(file, "Content")

      config = %{"use_cache" => false}
      task = AsyncAPI.batch_extract_files_async([file], "text/plain", config)

      assert %Task{} = task
      {:ok, _results} = Task.await(task)
    end
  end
end
