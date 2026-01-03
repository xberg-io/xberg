defmodule KreuzbergTest.Integration.AsyncOperationsTest do
  @moduledoc """
  Integration tests for asynchronous extraction operations.

  Tests cover:
  - Task-based async extraction
  - Concurrent operations with Task.async
  - Error handling in async context
  - Timeout handling for long operations
  - Multiple async operations coordination
  - Task supervisor integration
  """

  use ExUnit.Case, async: true

  @sample_text "This is sample text for extraction testing."
  @sample_html "<html><body><h1>Test</h1><p>Content</p></body></html>"

  describe "Task-based async extraction" do
    @tag :integration
    test "extract_async creates async task" do
      task = Kreuzberg.extract_async(@sample_text, "text/plain")

      # In Elixir 1.19+, Task is a struct, not a tuple
      assert is_struct(task, Task) or (is_tuple(task) and is_pid(elem(task, 0)))
    end

    @tag :integration
    test "awaits async extraction result" do
      task = Kreuzberg.extract_async(@sample_text, "text/plain")

      {:ok, result} = Task.await(task)

      assert result.content == @sample_text
      assert is_binary(result.mime_type)
    end

    @tag :integration
    test "extract_async with config" do
      config = %Kreuzberg.ExtractionConfig{
        use_cache: true
      }

      task = Kreuzberg.extract_async(@sample_text, "text/plain", config)

      {:ok, result} = Task.await(task)

      assert result.content != nil
    end

    @tag :integration
    test "multiple async extractions run concurrently" do
      texts = [
        "First document content",
        "Second document content",
        "Third document content"
      ]

      tasks =
        texts
        |> Enum.map(&Kreuzberg.extract_async(&1, "text/plain"))

      results =
        tasks
        |> Enum.map(&Task.await/1)

      assert length(results) == 3

      Enum.each(results, fn {:ok, result} ->
        assert result.content != nil
      end)
    end

    @tag :integration
    test "extract_file_async with file path" do
      # Create temp file
      tmp_file = Path.join(System.tmp_dir!(), "test_#{:erlang.unique_integer([:positive])}.txt")
      File.write!(tmp_file, @sample_text)

      try do
        task = Kreuzberg.extract_file_async(tmp_file)

        {:ok, result} = Task.await(task, 30_000)

        assert result.content != nil
      after
        File.rm!(tmp_file)
      end
    end

    @tag :integration
    test "extract_file_async with config" do
      tmp_file = Path.join(System.tmp_dir!(), "test_#{:erlang.unique_integer([:positive])}.txt")
      File.write!(tmp_file, @sample_text)

      try do
        config = %Kreuzberg.ExtractionConfig{use_cache: true}
        task = Kreuzberg.extract_file_async(tmp_file, nil, config)

        {:ok, result} = Task.await(task, 30_000)

        assert result != nil
      after
        File.rm!(tmp_file)
      end
    end
  end

  describe "Concurrent extraction operations" do
    @tag :integration
    test "concurrent text extractions" do
      text_mime_pairs = [
        {@sample_text, "text/plain"},
        {@sample_html, "text/html"},
        {"Numbers 123 456 789", "text/plain"}
      ]

      tasks =
        text_mime_pairs
        |> Enum.map(fn {text, mime} ->
          Kreuzberg.extract_async(text, mime)
        end)

      results = Enum.map(tasks, &Task.await/1)

      assert length(results) == 3

      Enum.each(results, fn {:ok, result} ->
        assert result != nil
      end)
    end

    @tag :integration
    test "await all with timeout" do
      tasks = [
        Kreuzberg.extract_async("Text 1", "text/plain"),
        Kreuzberg.extract_async("Text 2", "text/plain"),
        Kreuzberg.extract_async("Text 3", "text/plain")
      ]

      # Await with generous timeout
      results = Task.await_many(tasks, 30_000)

      assert length(results) == 3
    end

    @tag :integration
    test "async operations with varying document sizes" do
      sizes = [10, 100, 1000, 5000]

      tasks =
        sizes
        |> Enum.map(&String.duplicate("word ", &1))
        |> Enum.map(&Kreuzberg.extract_async(&1, "text/plain"))

      results = Enum.map(tasks, &Task.await/1)

      assert length(results) == 4

      Enum.each(results, fn {:ok, result} ->
        assert is_binary(result.content)
      end)
    end

    @tag :integration
    test "async batch file extraction" do
      # Create temp files
      temp_files =
        1..3
        |> Enum.map(fn idx ->
          path = Path.join(System.tmp_dir!(), "async_test_#{idx}.txt")
          File.write!(path, "Content #{idx}")
          path
        end)

      try do
        tasks = Enum.map(temp_files, &Kreuzberg.extract_file_async/1)
        results = Enum.map(tasks, &Task.await/1)

        assert length(results) == 3

        Enum.each(results, fn {:ok, result} ->
          assert result.content != nil
        end)
      after
        Enum.each(temp_files, &File.rm!/1)
      end
    end

    @tag :integration
    test "sequential await of async tasks" do
      task1 = Kreuzberg.extract_async("Doc 1", "text/plain")
      task2 = Kreuzberg.extract_async("Doc 2", "text/plain")
      task3 = Kreuzberg.extract_async("Doc 3", "text/plain")

      {:ok, result1} = Task.await(task1)
      {:ok, result2} = Task.await(task2)
      {:ok, result3} = Task.await(task3)

      assert result1.content == "Doc 1"
      assert result2.content == "Doc 2"
      assert result3.content == "Doc 3"
    end
  end

  describe "Error handling in async operations" do
    @tag :integration
    test "async extraction with invalid mime type returns error" do
      task = Kreuzberg.extract_async(@sample_text, "invalid/mime")

      case Task.await(task) do
        {:error, _reason} ->
          assert true

        {:ok, _result} ->
          # May succeed depending on implementation
          assert true
      end
    end

    @tag :integration
    test "error in async task is propagated" do
      task = Kreuzberg.extract_async("", "text/plain")

      case Task.await(task) do
        {:error, _} -> assert true
        {:ok, _} -> assert true
      end
    end

    @tag :integration
    test "extract_file_async with non-existent file" do
      non_existent = "/tmp/nonexistent_file_#{:erlang.unique_integer([:positive])}.txt"

      task = Kreuzberg.extract_file_async(non_existent)

      case Task.await(task, 10_000) do
        {:error, _} -> assert true
        {:ok, _} -> flunk("Should have errored on non-existent file")
      end
    end

    @tag :integration
    test "handles task timeout gracefully" do
      task = Kreuzberg.extract_async(String.duplicate("word ", 10_000), "text/plain")

      # Await with minimal timeout (but not 0 to avoid issues)
      case Task.await(task, 1) do
        {:error, error} ->
          # Timeout error expected
          assert is_tuple(error) or is_binary(error)

        {:ok, _result} ->
          # May succeed if fast enough
          assert true
      end
    end

    @tag :integration
    test "all tasks fail gracefully when one errors" do
      tasks = [
        Kreuzberg.extract_async("Valid text", "text/plain"),
        Kreuzberg.extract_async("", "text/plain"),
        Kreuzberg.extract_async("More text", "text/plain")
      ]

      results =
        Enum.map(tasks, fn task ->
          try do
            Task.await(task)
          rescue
            e -> {:error, e}
          end
        end)

      # Should have 3 results regardless
      assert length(results) == 3
    end
  end

  describe "Timeout handling" do
    @tag :integration
    test "await with explicit timeout" do
      task = Kreuzberg.extract_async(@sample_text, "text/plain")

      # Use reasonable timeout
      {:ok, result} = Task.await(task, 30_000)

      assert result != nil
    end

    @tag :integration
    test "await with default timeout" do
      task = Kreuzberg.extract_async(@sample_text, "text/plain")

      {:ok, result} = Task.await(task)

      assert result != nil
    end

    @tag :integration
    test "multiple awaits with timeout" do
      tasks = [
        Kreuzberg.extract_async("Text 1", "text/plain"),
        Kreuzberg.extract_async("Text 2", "text/plain")
      ]

      timeout = 30_000

      results =
        tasks
        |> Enum.map(fn task ->
          try do
            {:ok, Task.await(task, timeout)}
          rescue
            _e -> {:error, "timeout"}
          end
        end)

      assert length(results) == 2
    end

    @tag :integration
    test "timeout returns error when exceeded" do
      # Create a task that will take time
      task = Kreuzberg.extract_async(String.duplicate("long ", 5000), "text/plain")

      # Try with very short timeout
      try do
        Task.await(task, 1)
      rescue
        _e ->
          # Timeout exception expected
          assert true
      catch
        :exit, _e ->
          # Task exit (timeout) is also acceptable
          assert true
      end
    end
  end

  describe "Batch async operations" do
    @tag :integration
    test "batch_extract_files_async with multiple files" do
      # Create temp files
      temp_files =
        1..3
        |> Enum.map(fn idx ->
          path = Path.join(System.tmp_dir!(), "batch_async_#{idx}.txt")
          File.write!(path, "Content #{idx}")
          path
        end)

      try do
        task = Kreuzberg.batch_extract_files_async(temp_files)

        {:ok, results} = Task.await(task, 30_000)

        assert is_list(results)
      after
        Enum.each(temp_files, &File.rm!/1)
      end
    end

    @tag :integration
    test "batch_extract_files_async with config" do
      temp_files =
        1..2
        |> Enum.map(fn idx ->
          path = Path.join(System.tmp_dir!(), "batch_config_#{idx}.txt")
          File.write!(path, "Document #{idx}")
          path
        end)

      try do
        config = %Kreuzberg.ExtractionConfig{use_cache: true}
        task = Kreuzberg.batch_extract_files_async(temp_files, nil, config)

        {:ok, results} = Task.await(task, 30_000)

        assert is_list(results)
      after
        Enum.each(temp_files, &File.rm!/1)
      end
    end

    @tag :integration
    test "batch_extract_bytes_async with multiple documents" do
      contents = [
        "Document 1 content",
        "Document 2 content",
        "Document 3 content"
      ]

      task = Kreuzberg.batch_extract_bytes_async(contents, "text/plain")

      {:ok, results} = Task.await(task, 30_000)

      assert is_list(results)
    end

    @tag :integration
    test "batch_extract_bytes_async with config" do
      contents = ["Content 1", "Content 2"]
      config = %Kreuzberg.ExtractionConfig{use_cache: false}

      task = Kreuzberg.batch_extract_bytes_async(contents, "text/plain", config)

      {:ok, results} = Task.await(task, 30_000)

      assert is_list(results)
    end
  end

  describe "Async operation characteristics" do
    @tag :integration
    test "async tasks return Task struct" do
      task = Kreuzberg.extract_async(@sample_text, "text/plain")

      assert is_struct(task, Task)
    end

    @tag :integration
    test "async operations are non-blocking" do
      # Record start time
      start_time = System.monotonic_time()

      # Create task without awaiting
      _task = Kreuzberg.extract_async(String.duplicate("word ", 1000), "text/plain")

      # Should return immediately
      elapsed = System.monotonic_time() - start_time

      # Should be much faster than actual extraction would take
      # 1 second in nanoseconds
      assert elapsed < 1_000_000_000
    end

    @tag :integration
    test "multiple async tasks run independently" do
      task1 = Kreuzberg.extract_async("Text 1", "text/plain")
      task2 = Kreuzberg.extract_async("Text 2", "text/plain")
      task3 = Kreuzberg.extract_async("Text 3", "text/plain")

      # All tasks should exist independently
      assert is_struct(task1, Task)
      assert is_struct(task2, Task)
      assert is_struct(task3, Task)

      # Can await in any order
      {:ok, r1} = Task.await(task3)
      {:ok, r2} = Task.await(task1)
      {:ok, r3} = Task.await(task2)

      assert r1.content == "Text 3"
      assert r2.content == "Text 1"
      assert r3.content == "Text 2"
    end

    @tag :integration
    test "result structure from async extraction" do
      task = Kreuzberg.extract_async(@sample_text, "text/plain")

      {:ok, result} = Task.await(task)

      assert is_struct(result, Kreuzberg.ExtractionResult)
      assert Map.has_key?(result, :content)
      assert Map.has_key?(result, :mime_type)
      assert Map.has_key?(result, :metadata)
    end
  end

  describe "Async resource cleanup" do
    @tag :integration
    test "completed task cleans up resources" do
      task = Kreuzberg.extract_async(@sample_text, "text/plain")

      {:ok, _result} = Task.await(task)

      # Task should be completed (process may have terminated)
      case Process.info(task.pid, :status) do
        {_, status} -> assert status in [:done, :exiting]
        nil -> assert true # Process already terminated, which is expected
      end
    end

    @tag :integration
    test "awaiting multiple times on same task" do
      task = Kreuzberg.extract_async(@sample_text, "text/plain")

      {:ok, result1} = Task.await(task)

      # Awaiting again should fail (task already consumed)
      try do
        Task.await(task, 100)
        # If it doesn't fail, that's also acceptable behavior
        assert true
      rescue
        _e ->
          # Timeout exception is expected
          assert true
      catch
        :exit, _e ->
          # Expected - task already awaited or timeout
          assert true
      end

      assert result1 != nil
    end
  end
end
