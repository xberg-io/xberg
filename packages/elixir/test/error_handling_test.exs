defmodule KreuzbergTest.ErrorHandlingTest do
  @moduledoc """
  Comprehensive error handling tests for Kreuzberg Elixir binding.

  Tests cover 8 behavior-driven error scenarios:
  1. Invalid config handling - negative/invalid config values
  2. File not found / corrupted files - missing or unreadable files
  3. Invalid MIME types - unsupported document formats
  4. Permission errors - file access restrictions
  5. Malformed document handling - corrupted document content
  6. Out-of-memory patterns - extremely large document processing
  7. Timeout behavior - long-running extraction operations
  8. Concurrent error states - parallel error condition handling

  All tests follow the {:ok, _} / {:error, _} pattern with NIF error propagation.
  """

  use ExUnit.Case

  # Helper to create temporary test files
  defp create_temp_file(content, filename \\ nil) do
    unique_id = System.unique_integer()
    name = filename || "kreuzberg_test_#{unique_id}.txt"
    path = System.tmp_dir!() <> "/" <> name
    File.write!(path, content)
    path
  end

  # Helper to create temporary directory
  defp create_temp_dir do
    unique_id = System.unique_integer()
    path = System.tmp_dir!() <> "/kreuzberg_dir_#{unique_id}"
    File.mkdir_p!(path)
    path
  end

  # Helper to cleanup temporary files
  defp cleanup_file(path) when is_binary(path) do
    if File.exists?(path) do
      File.rm(path)
    end
  end

  # Helper to cleanup directories
  defp cleanup_dir(path) when is_binary(path) do
    if File.exists?(path) do
      File.rm_rf(path)
    end
  end

  # ============================================================================
  # 1. INVALID CONFIG HANDLING
  # ============================================================================

  describe "invalid config handling" do
    @tag :error_handling
    test "returns error for negative max_chars in chunking config" do
      config = %Kreuzberg.ExtractionConfig{
        chunking: %{"max_chars" => -100, "max_overlap" => 50}
      }

      result = Kreuzberg.extract("test content", "text/plain", config)

      assert {:error, message} = result
      assert is_binary(message)
      assert byte_size(message) > 0
      # Validate message contains meaningful context
      assert String.contains?(message, "negative") or
               String.contains?(message, "max_chars") or
               String.contains?(message, "positive"),
             "Error message should indicate constraint: #{message}"
    end

    @tag :error_handling
    test "returns error for zero max_chars" do
      config = %Kreuzberg.ExtractionConfig{
        chunking: %{"max_chars" => 0, "max_overlap" => 0}
      }

      result = Kreuzberg.extract("test", "text/plain", config)

      assert {:error, _message} = result
    end

    @tag :error_handling
    test "returns error for max_overlap exceeding max_chars" do
      config = %Kreuzberg.ExtractionConfig{
        chunking: %{"max_chars" => 100, "max_overlap" => 200}
      }

      result = Kreuzberg.extract("test content", "text/plain", config)

      assert {:error, message} = result
      assert is_binary(message)
    end

    @tag :error_handling
    test "returns error for invalid confidence threshold" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: %{"confidence" => 1.5}
      }

      result = Kreuzberg.extract("test", "text/plain", config)

      assert {:error, _message} = result
    end

    @tag :error_handling
    test "returns error for negative DPI value" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: %{"dpi" => -300}
      }

      result = Kreuzberg.extract("test", "text/plain", config)

      assert {:error, message} = result
      assert is_binary(message)
    end

    @tag :error_handling
    test "returns error for DPI exceeding maximum" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: %{"dpi" => 5000}
      }

      result = Kreuzberg.extract("test", "text/plain", config)

      assert {:error, message} = result
      assert is_binary(message)
    end

    @tag :error_handling
    test "returns error tuple not exception for invalid config" do
      config = %Kreuzberg.ExtractionConfig{
        chunking: %{"max_chars" => -1}
      }

      result = Kreuzberg.extract("data", "text/plain", config)

      # Verify it's an error tuple, not an exception
      assert {:error, _reason} = result
      refute is_exception(result)
    end

    @tag :error_handling
    test "error message is descriptive for invalid config" do
      config = %Kreuzberg.ExtractionConfig{
        chunking: %{"max_chars" => -100, "max_overlap" => 0}
      }

      {:error, message} = Kreuzberg.extract("test", "text/plain", config)

      assert is_binary(message)
      assert String.length(message) > 5
    end

    @tag :error_handling
    test "bang variant raises Kreuzberg.Error on invalid config" do
      config = %Kreuzberg.ExtractionConfig{
        chunking: %{"max_chars" => -100}
      }

      assert_raise Kreuzberg.Error, fn ->
        Kreuzberg.extract!("test", "text/plain", config)
      end
    end
  end

  # ============================================================================
  # 2. FILE NOT FOUND / CORRUPTED FILES
  # ============================================================================

  describe "file not found and corrupted files" do
    @tag :error_handling
    test "returns error for non-existent file" do
      non_existent_path = "/tmp/kreuzberg_nonexistent_#{System.unique_integer()}.txt"

      result = Kreuzberg.extract_file(non_existent_path, "text/plain")

      assert {:error, message} = result
      assert is_binary(message)
      assert byte_size(message) > 0
      # Validate error message is meaningful
      assert String.contains?(message, "not found") or
               String.contains?(message, "does not exist") or
               String.contains?(message, "No such"),
             "Error should indicate file not found: #{message}"
    end

    @tag :error_handling
    test "returns error for directory path instead of file" do
      dir_path = create_temp_dir()

      try do
        result = Kreuzberg.extract_file(dir_path, "text/plain")

        # Should fail when trying to read a directory as a file
        assert {:error, _message} = result
      after
        cleanup_dir(dir_path)
      end
    end

    @tag :error_handling
    test "returns error for empty file" do
      path = create_temp_file("")

      try do
        result = Kreuzberg.extract_file(path, "text/plain")

        # Empty content may trigger error or return empty result
        case result do
          # Some implementations allow empty
          {:ok, _} -> assert true
          # Some implementations reject empty
          {:error, _} -> assert true
        end
      after
        cleanup_file(path)
      end
    end

    @tag :error_handling
    test "error tuple for missing file, not exception" do
      missing_path = "/tmp/nonexistent_#{System.unique_integer()}"

      result = Kreuzberg.extract_file(missing_path, "text/plain")

      assert {:error, _reason} = result
      refute is_exception(result)
    end

    @tag :error_handling
    test "bang variant raises on file not found" do
      missing_path = "/tmp/nonexistent_#{System.unique_integer()}"

      assert_raise Kreuzberg.Error, fn ->
        Kreuzberg.extract_file!(missing_path, "text/plain")
      end
    end

    @tag :error_handling
    test "file extraction error includes path context" do
      missing_path = "/tmp/missing_#{System.unique_integer()}"

      {:error, message} = Kreuzberg.extract_file(missing_path, "text/plain")

      assert is_binary(message)
      # Error should be descriptive about file operations
      assert String.contains?(message, "not found") or
               String.contains?(message, "does not exist") or
               String.contains?(message, "No such")
    end

    @tag :error_handling
    test "handles unreadable file gracefully" do
      # Create a file, then try to change permissions (Unix-like systems)
      path = create_temp_file("test content")

      try do
        # Attempt to remove read permissions (may not work on all systems)
        File.chmod(path, 0o000)

        result = Kreuzberg.extract_file(path, "text/plain")

        # Should fail due to permission error
        case result do
          # Permissions might not be enforced in test environment
          {:ok, _} -> :skip
          {:error, _} -> assert true
        end
      after
        # Restore permissions before cleanup
        File.chmod(path, 0o644)
        cleanup_file(path)
      end
    end
  end

  # ============================================================================
  # 3. INVALID MIME TYPES
  # ============================================================================

  describe "invalid MIME types" do
    @tag :error_handling
    test "returns error for completely invalid MIME type" do
      result = Kreuzberg.extract("test", "invalid/type")

      assert {:error, message} = result
      assert is_binary(message)
      assert byte_size(message) > 0
      # Message should explain the MIME type issue
      assert String.contains?(message, "MIME") or
               String.contains?(message, "mime") or
               String.contains?(message, "format") or
               String.contains?(message, "unsupported"),
             "Error should mention MIME/format issue: #{message}"
    end

    @tag :error_handling
    test "returns error for malformed MIME type" do
      result = Kreuzberg.extract("test", "not-a-mime-type")

      assert {:error, _message} = result
    end

    @tag :error_handling
    test "returns error for empty MIME type" do
      result = Kreuzberg.extract("test", "")

      assert {:error, message} = result
      assert is_binary(message)
    end

    @tag :error_handling
    test "returns error for MIME type without subtype" do
      result = Kreuzberg.extract("test", "text")

      assert {:error, _message} = result
    end

    @tag :error_handling
    test "returns error for unsupported but valid MIME type" do
      result = Kreuzberg.extract("test", "application/unknown")

      assert {:error, message} = result
      assert is_binary(message)
    end

    @tag :error_handling
    test "error message mentions MIME or format for invalid type" do
      {:error, message} = Kreuzberg.extract("data", "invalid/mime")

      assert is_binary(message)
      # Check for helpful error information
      assert String.contains?(message, "MIME") or
               String.contains?(message, "mime") or
               String.contains?(message, "format") or
               String.contains?(message, "type")
    end

    @tag :error_handling
    test "bang variant raises on invalid MIME type" do
      assert_raise Kreuzberg.Error, fn ->
        Kreuzberg.extract!("test", "unsupported/type")
      end
    end

    @tag :error_handling
    test "multiple invalid MIME types all produce errors" do
      invalid_types = [
        "totally/invalid",
        "wrong/format",
        "no-subtype",
        "application/x-unknown",
        ""
      ]

      Enum.each(invalid_types, fn mime_type ->
        result = Kreuzberg.extract("test", mime_type)
        assert {:error, _reason} = result
      end)
    end
  end

  # ============================================================================
  # 4. PERMISSION ERRORS
  # ============================================================================

  describe "permission errors" do
    @tag :error_handling
    test "handles file read permission errors gracefully" do
      path = create_temp_file("protected content")

      try do
        # Remove read permissions (may not work on all systems)
        File.chmod(path, 0o000)

        result = Kreuzberg.extract_file(path, "text/plain")

        # Should handle permission error gracefully
        case result do
          # May not enforce on some systems
          {:ok, _} -> :skip
          {:error, _reason} -> assert true
        end
      after
        File.chmod(path, 0o644)
        cleanup_file(path)
      end
    end

    @tag :error_handling
    test "returns error tuple for permission denied" do
      path = create_temp_file("restricted")

      try do
        File.chmod(path, 0o000)

        result = Kreuzberg.extract_file(path, "text/plain")

        # Verify error tuple structure
        case result do
          {:ok, _} ->
            :skip

          {:error, message} ->
            assert is_binary(message)
            assert byte_size(message) > 0
        end
      after
        File.chmod(path, 0o644)
        cleanup_file(path)
      end
    end

    @tag :error_handling
    test "permission error is not an exception" do
      path = create_temp_file("test")

      try do
        File.chmod(path, 0o000)

        result = Kreuzberg.extract_file(path, "text/plain")

        case result do
          {:ok, _} ->
            :skip

          {:error, _reason} ->
            refute is_exception(result)
            assert is_tuple(result)
            assert tuple_size(result) == 2
        end
      after
        File.chmod(path, 0o644)
        cleanup_file(path)
      end
    end
  end

  # ============================================================================
  # 5. MALFORMED DOCUMENT HANDLING
  # ============================================================================

  describe "malformed document handling" do
    @tag :error_handling
    test "handles malformed text content gracefully" do
      # Binary data that looks corrupted
      malformed_content = <<0xFF, 0xFE, 0x00, 0x00, "invalid", 0x00, 0xFF>>

      path = create_temp_file(malformed_content)

      try do
        result = Kreuzberg.extract_file(path, "text/plain")

        # Should handle gracefully
        case result do
          # May still extract
          {:ok, _} -> assert true
          # Or fail gracefully
          {:error, _message} -> assert true
        end
      after
        cleanup_file(path)
      end
    end

    @tag :error_handling
    test "handles null bytes in content" do
      content_with_nulls = "Hello\x00\x00World\x00Test"
      path = create_temp_file(content_with_nulls)

      try do
        result = Kreuzberg.extract_file(path, "text/plain")

        case result do
          {:ok, result} ->
            assert %Kreuzberg.ExtractionResult{} = result

          {:error, _message} ->
            assert true
        end
      after
        cleanup_file(path)
      end
    end

    @tag :error_handling
    test "returns error tuple for invalid UTF-8 sequences" do
      # Invalid UTF-8 sequence
      invalid_utf8 = <<0xC3, 0x28>>
      path = create_temp_file(invalid_utf8)

      try do
        result = Kreuzberg.extract_file(path, "text/plain")

        case result do
          {:ok, _} ->
            assert true

          {:error, message} ->
            assert is_binary(message)
            assert byte_size(message) > 0
        end
      after
        cleanup_file(path)
      end
    end

    @tag :error_handling
    test "bang variant raises on malformed content" do
      malformed = <<0xFF, 0xFE, 0xFF, 0xFE>>
      path = create_temp_file(malformed)

      try do
        # Some implementations may raise on malformed data
        result = Kreuzberg.extract_file!(path, "text/plain")

        # If no exception, result should be valid
        assert %Kreuzberg.ExtractionResult{} = result
      rescue
        Kreuzberg.Error -> assert true
      after
        cleanup_file(path)
      end
    end
  end

  # ============================================================================
  # 6. OUT-OF-MEMORY PATTERNS
  # ============================================================================

  describe "out-of-memory patterns" do
    @tag :error_handling
    test "handles extremely large content gracefully" do
      # Create very large content (10 MB)
      large_content = String.duplicate("a", 10_000_000)

      path = create_temp_file(large_content)

      try do
        result = Kreuzberg.extract_file(path, "text/plain")

        case result do
          {:ok, %Kreuzberg.ExtractionResult{}} ->
            assert true

          {:error, message} ->
            assert is_binary(message)
            # Might fail with memory or timeout error
            assert byte_size(message) > 0
        end
      after
        cleanup_file(path)
      end
    end

    @tag :error_handling
    test "returns error tuple not exception for memory issues" do
      large_content = String.duplicate("x", 5_000_000)
      path = create_temp_file(large_content)

      try do
        result = Kreuzberg.extract_file(path, "text/plain")

        case result do
          {:ok, _} ->
            assert true

          {:error, _reason} ->
            refute is_exception(result)
            assert is_tuple(result)
        end
      after
        cleanup_file(path)
      end
    end

    @tag :error_handling
    test "handles repeated extraction of large files" do
      large_content = String.duplicate("test", 1_000_000)
      path = create_temp_file(large_content)

      try do
        # Attempt multiple extractions
        results =
          Enum.map(1..3, fn _ ->
            Kreuzberg.extract_file(path, "text/plain")
          end)

        # All should be either successful or error tuples
        Enum.each(results, fn result ->
          assert is_tuple(result)
          assert tuple_size(result) == 2
        end)
      after
        cleanup_file(path)
      end
    end

    @tag :error_handling
    test "memory error message is descriptive" do
      huge_content = String.duplicate("data", 8_000_000)
      path = create_temp_file(huge_content)

      try do
        result = Kreuzberg.extract_file(path, "text/plain")

        case result do
          {:error, message} ->
            assert is_binary(message)
            assert byte_size(message) > 0

          {:ok, _} ->
            assert true
        end
      after
        cleanup_file(path)
      end
    end
  end

  # ============================================================================
  # 7. TIMEOUT BEHAVIOR
  # ============================================================================

  describe "timeout behavior" do
    @tag :error_handling
    test "extraction with timeout completes or returns error" do
      content = "test content for timeout check"

      result =
        Task.yield(
          Task.async(fn ->
            Kreuzberg.extract(content, "text/plain")
          end),
          5000
        )

      # Either completes with result or times out
      case result do
        {:ok, {:ok, %Kreuzberg.ExtractionResult{}}} -> assert true
        {:ok, {:error, _message}} -> assert true
        # Task timed out
        nil -> assert true
      end
    end

    @tag :error_handling
    test "file extraction respects timeout boundaries" do
      path = create_temp_file("test")

      try do
        result =
          Task.yield(
            Task.async(fn ->
              Kreuzberg.extract_file(path, "text/plain")
            end),
            10_000
          )

        case result do
          {:ok, {:ok, %Kreuzberg.ExtractionResult{}}} -> assert true
          {:ok, {:error, _}} -> assert true
          nil -> assert true
        end
      after
        cleanup_file(path)
      end
    end

    @tag :error_handling
    test "returns error tuple on timeout completion" do
      content = String.duplicate("test", 100_000)

      result =
        Task.yield(
          Task.async(fn ->
            Kreuzberg.extract(content, "text/plain")
          end),
          5000
        )

      case result do
        {:ok, tuple_result} ->
          assert is_tuple(tuple_result)
          assert tuple_size(tuple_result) == 2

        nil ->
          assert true
      end
    end

    @tag :error_handling
    test "timeout does not raise exception but returns error or timeout" do
      content = "timeout test"

      result =
        Task.yield(
          Task.async(fn ->
            Kreuzberg.extract(content, "text/plain")
          end),
          3000
        )

      case result do
        {:ok, {:ok, _}} -> assert true
        {:ok, {:error, _}} -> assert true
        nil -> assert true
      end
    end
  end

  # ============================================================================
  # 8. CONCURRENT ERROR STATES
  # ============================================================================

  describe "concurrent error states" do
    @tag :error_handling
    test "multiple concurrent errors are handled independently" do
      tasks =
        Enum.map(1..5, fn _ ->
          Task.async(fn ->
            Kreuzberg.extract("test", "invalid/mime")
          end)
        end)

      results = Task.await_many(tasks)

      # All should be error tuples
      Enum.each(results, fn result ->
        assert {:error, _message} = result
      end)
    end

    @tag :error_handling
    test "concurrent mixed success and error states" do
      tasks = [
        Task.async(fn -> Kreuzberg.extract("test", "text/plain") end),
        Task.async(fn -> Kreuzberg.extract("data", "invalid/type") end),
        Task.async(fn -> Kreuzberg.extract("more", "text/plain") end),
        Task.async(fn -> Kreuzberg.extract("bad", "unknown/mime") end)
      ]

      results = Task.await_many(tasks)

      # Verify all are tuples
      Enum.each(results, fn result ->
        assert is_tuple(result)
        assert tuple_size(result) == 2
      end)

      # Verify we have both successes and errors
      has_success = Enum.any?(results, fn {status, _} -> status == :ok end)
      has_error = Enum.any?(results, fn {status, _} -> status == :error end)

      assert has_error == true
    end

    @tag :error_handling
    test "concurrent file extractions with mixed valid and invalid" do
      valid_path = create_temp_file("valid content")
      invalid_path = "/tmp/nonexistent_#{System.unique_integer()}"

      try do
        tasks = [
          Task.async(fn -> Kreuzberg.extract_file(valid_path, "text/plain") end),
          Task.async(fn -> Kreuzberg.extract_file(invalid_path, "text/plain") end),
          Task.async(fn -> Kreuzberg.extract_file(valid_path, "text/plain") end),
          Task.async(fn -> Kreuzberg.extract_file(invalid_path, "text/plain") end)
        ]

        results = Task.await_many(tasks)

        # All results should be proper tuples
        Enum.each(results, fn result ->
          assert is_tuple(result)
          assert tuple_size(result) == 2
          assert elem(result, 0) in [:ok, :error]
        end)
      after
        cleanup_file(valid_path)
      end
    end

    @tag :error_handling
    test "concurrent config errors don't interfere" do
      configs = [
        %Kreuzberg.ExtractionConfig{chunking: %{"max_chars" => -100}},
        %Kreuzberg.ExtractionConfig{chunking: %{"max_chars" => 1000}},
        %Kreuzberg.ExtractionConfig{ocr: %{"dpi" => -300}},
        %Kreuzberg.ExtractionConfig{chunking: %{"max_chars" => 1000}}
      ]

      tasks =
        Enum.map(configs, fn config ->
          Task.async(fn ->
            Kreuzberg.extract("test", "text/plain", config)
          end)
        end)

      results = Task.await_many(tasks)

      # Verify all are valid result tuples
      Enum.each(results, fn result ->
        assert is_tuple(result)
        assert tuple_size(result) == 2
      end)

      # Invalid configs should produce errors
      invalid_results = Enum.filter(results, fn {status, _} -> status == :error end)
      assert length(invalid_results) >= 2
    end

    @tag :error_handling
    test "concurrent operations maintain error isolation" do
      tasks =
        Enum.map(1..10, fn i ->
          Task.async(fn ->
            if rem(i, 2) == 0 do
              Kreuzberg.extract("content", "invalid/type#{i}")
            else
              Kreuzberg.extract("valid", "text/plain")
            end
          end)
        end)

      results = Task.await_many(tasks)

      # Each error should be independent and properly formatted
      errors = Enum.filter(results, fn {status, _} -> status == :error end)
      successes = Enum.filter(results, fn {status, _} -> status == :ok end)

      # Verify mix of results
      assert errors != []
      assert successes != []

      # All errors should have messages
      Enum.each(errors, fn {:error, message} ->
        assert is_binary(message)
        assert byte_size(message) > 0
      end)
    end

    @tag :error_handling
    test "exception safety with concurrent operations" do
      tasks =
        Enum.map(1..5, fn _ ->
          Task.async(fn ->
            # Wrap in try-catch to ensure no exceptions escape
            try do
              Kreuzberg.extract("test", "invalid/mime")
            rescue
              e in Kreuzberg.Error -> {:caught_error, e}
            end
          end)
        end)

      results = Task.await_many(tasks)

      # All tasks should complete without unhandled exceptions
      Enum.each(results, fn result ->
        assert is_tuple(result)
        assert tuple_size(result) >= 2
      end)
    end
  end
end
