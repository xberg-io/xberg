defmodule KreuzbergTest.Unit.BatchAPITest do
  @moduledoc """
  Unit tests for batch extraction operations.

  Tests cover:
  - batch_extract_files/2-3: Batch file extraction with success and error cases
  - batch_extract_files!/2-3: Bang variant with direct returns and exceptions
  - batch_extract_bytes/2-3: Batch binary extraction
  - batch_extract_bytes!/2-3: Bang variant for batch binary extraction
  """

  use ExUnit.Case

  alias Kreuzberg.BatchAPI
  alias Kreuzberg.ExtractionConfig

  # Helper function to create temporary directories
  defp create_temp_dir do
    base = System.tmp_dir!()
    dir = Path.join(base, "kreuzberg_test_#{:erlang.unique_integer([:positive])}")
    File.mkdir_p!(dir)
    {:ok, dir}
  end

  describe "batch_extract_files/2" do
    @tag :unit
    test "returns error for empty paths list" do
      {:error, reason} = BatchAPI.batch_extract_files([])
      assert is_binary(reason)
      assert String.contains?(reason, "empty")
    end

    @tag :unit
    test "returns error for empty paths list with mime type" do
      {:error, reason} = BatchAPI.batch_extract_files([], "text/plain")
      assert is_binary(reason)
      assert String.contains?(reason, "empty")
    end

    @tag :unit
    @tag :integration
    test "extracts from multiple text files" do
      # Create temporary test files
      {:ok, dir} = create_temp_dir()

      file1 = Path.join(dir, "test1.txt")
      file2 = Path.join(dir, "test2.txt")
      file3 = Path.join(dir, "test3.txt")

      File.write!(file1, "Content 1")
      File.write!(file2, "Content 2")
      File.write!(file3, "Content 3")

      paths = [file1, file2, file3]
      {:ok, results} = BatchAPI.batch_extract_files(paths, "text/plain")

      assert is_list(results)
      assert length(results) == 3

      [result1, result2, result3] = results

      assert result1.content == "Content 1"
      assert result2.content == "Content 2"
      assert result3.content == "Content 3"
    end

    @tag :unit
    @tag :integration
    test "returns error if any file fails" do
      # Mix valid and invalid files
      {:ok, dir} = create_temp_dir()
      valid_file = Path.join(dir, "valid.txt")
      File.write!(valid_file, "Valid content")

      paths = [valid_file, "/nonexistent/file.txt"]
      {:error, reason} = BatchAPI.batch_extract_files(paths, "text/plain")

      assert is_binary(reason)
    end
  end

  describe "batch_extract_files/3" do
    @tag :unit
    @tag :integration
    test "accepts ExtractionConfig struct" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Test content")

      config = %ExtractionConfig{use_cache: false}
      {:ok, results} = BatchAPI.batch_extract_files([file], "text/plain", config)

      assert length(results) == 1
      assert hd(results).content == "Test content"
    end

    @tag :unit
    @tag :integration
    test "accepts map config" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Test content")

      {:ok, results} =
        BatchAPI.batch_extract_files([file], "text/plain", %{
          "use_cache" => false
        })

      assert length(results) == 1
      assert hd(results).content == "Test content"
    end

    @tag :unit
    @tag :integration
    test "works with nil mime_type for auto-detection" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Test content")

      {:ok, results} = BatchAPI.batch_extract_files([file], nil)

      assert length(results) == 1
      assert hd(results).content == "Test content"
    end
  end

  describe "batch_extract_files!/2" do
    @tag :unit
    test "raises on empty paths list" do
      assert_raise Kreuzberg.Error, fn ->
        BatchAPI.batch_extract_files!([])
      end
    end

    @tag :unit
    @tag :integration
    test "returns results directly on success" do
      {:ok, dir} = create_temp_dir()
      file = Path.join(dir, "test.txt")
      File.write!(file, "Test content")

      results = BatchAPI.batch_extract_files!([file], "text/plain")

      assert is_list(results)
      assert length(results) == 1
      assert hd(results).content == "Test content"
    end

    @tag :unit
    @tag :integration
    test "raises on file not found" do
      assert_raise Kreuzberg.Error, fn ->
        BatchAPI.batch_extract_files!(["/nonexistent/file.txt"], "text/plain")
      end
    end
  end

  describe "batch_extract_bytes/2" do
    @tag :unit
    test "returns error for empty data list" do
      {:error, reason} = BatchAPI.batch_extract_bytes([], [])
      assert is_binary(reason)
      assert String.contains?(reason, "empty")
    end

    @tag :unit
    test "returns error for mismatched lengths" do
      data_list = ["data1", "data2", "data3"]
      mime_types = ["text/plain", "text/plain"]

      {:error, reason} = BatchAPI.batch_extract_bytes(data_list, mime_types)
      assert is_binary(reason)
      assert String.contains?(reason, "Mismatch")
    end

    @tag :unit
    test "accepts single mime type for all inputs" do
      data_list = ["Content 1", "Content 2", "Content 3"]

      {:ok, results} = BatchAPI.batch_extract_bytes(data_list, "text/plain")

      assert is_list(results)
      assert length(results) == 3

      [result1, result2, result3] = results

      assert result1.content == "Content 1"
      assert result2.content == "Content 2"
      assert result3.content == "Content 3"
    end

    @tag :unit
    test "accepts list of mime types" do
      data_list = ["Content 1", "Content 2"]
      mime_types = ["text/plain", "text/plain"]

      {:ok, results} = BatchAPI.batch_extract_bytes(data_list, mime_types)

      assert length(results) == 2
      assert hd(results).content == "Content 1"
    end

    @tag :unit
    test "returns error if any extraction fails" do
      data_list = ["Valid content", "data"]
      mime_types = ["text/plain", "invalid/type"]

      {:error, reason} = BatchAPI.batch_extract_bytes(data_list, mime_types)
      assert is_binary(reason)
    end
  end

  describe "batch_extract_bytes/3" do
    @tag :unit
    test "accepts ExtractionConfig struct" do
      data_list = ["Content"]
      config = %ExtractionConfig{use_cache: false}

      {:ok, results} = BatchAPI.batch_extract_bytes(data_list, "text/plain", config)

      assert length(results) == 1
      assert hd(results).content == "Content"
    end

    @tag :unit
    test "accepts map config" do
      data_list = ["Content"]

      {:ok, results} =
        BatchAPI.batch_extract_bytes(data_list, "text/plain", %{
          "use_cache" => false
        })

      assert length(results) == 1
      assert hd(results).content == "Content"
    end
  end

  describe "batch_extract_bytes!/2" do
    @tag :unit
    test "raises on empty data list" do
      assert_raise Kreuzberg.Error, fn ->
        BatchAPI.batch_extract_bytes!([], [])
      end
    end

    @tag :unit
    test "returns results directly on success" do
      data_list = ["Content"]
      results = BatchAPI.batch_extract_bytes!(data_list, "text/plain")

      assert is_list(results)
      assert length(results) == 1
      assert hd(results).content == "Content"
    end

    @tag :unit
    test "raises on extraction failure" do
      data_list = ["data"]
      mime_types = ["invalid/type"]

      assert_raise Kreuzberg.Error, fn ->
        BatchAPI.batch_extract_bytes!(data_list, mime_types)
      end
    end
  end

  describe "batch_extract_bytes!/3" do
    @tag :unit
    test "accepts ExtractionConfig struct" do
      data_list = ["Content"]
      config = %ExtractionConfig{use_cache: false}

      results = BatchAPI.batch_extract_bytes!(data_list, "text/plain", config)

      assert is_list(results)
      assert length(results) == 1
      assert hd(results).content == "Content"
    end
  end

  describe "edge cases and error paths" do
    @tag :unit
    test "batch_extract_bytes handles mismatched list lengths" do
      data_list = ["Content 1", "Content 2", "Content 3"]
      mime_types = ["text/plain", "text/plain"]  # Only 2 MIME types for 3 inputs

      {:error, reason} = BatchAPI.batch_extract_bytes(data_list, mime_types)
      assert reason =~ "Mismatch"
      assert reason =~ "3"
      assert reason =~ "2"
    end

    @tag :unit
    @tag :integration
    test "batch_extract_files tracks error index correctly" do
      {:ok, dir} = create_temp_dir()
      file1 = Path.join(dir, "valid.txt")
      File.write!(file1, "Valid content")

      # Create a list with valid and invalid files
      paths = [file1, "/definitely/nonexistent/path.txt", file1]
      {:error, reason} = BatchAPI.batch_extract_files(paths, "text/plain")

      # Should report the failing file in the error message
      assert reason =~ "/definitely/nonexistent/path.txt"
      assert reason =~ "does not exist"
    end

    @tag :unit
    test "batch_extract_bytes tracks error index correctly" do
      data_list = ["Valid 1", "Valid 2", "Valid 3"]
      # Use an invalid MIME type for the second item
      mime_types = ["text/plain", "completely/invalid", "text/plain"]

      {:error, reason} = BatchAPI.batch_extract_bytes(data_list, mime_types)
      # Should report the invalid MIME type in error message
      assert reason =~ "completely/invalid"
    end

    @tag :unit
    @tag :integration
    test "batch_extract_files! propagates error with index info" do
      {:ok, dir} = create_temp_dir()
      file1 = Path.join(dir, "valid.txt")
      File.write!(file1, "Valid content")

      paths = [file1, "/nonexistent.txt"]

      assert_raise Kreuzberg.Error, fn ->
        BatchAPI.batch_extract_files!(paths, "text/plain")
      end
    end

    @tag :unit
    test "batch_extract_bytes with single MIME type for all" do
      data_list = ["Content 1", "Content 2", "Content 3"]

      {:ok, results} = BatchAPI.batch_extract_bytes(data_list, "text/plain")

      assert length(results) == 3
      assert Enum.at(results, 0).content == "Content 1"
      assert Enum.at(results, 1).content == "Content 2"
      assert Enum.at(results, 2).content == "Content 3"
    end

    @tag :unit
    @tag :integration
    test "batch_extract_files with multiple successful files" do
      {:ok, dir} = create_temp_dir()

      file1 = Path.join(dir, "test1.txt")
      file2 = Path.join(dir, "test2.txt")
      file3 = Path.join(dir, "test3.txt")

      File.write!(file1, "First file")
      File.write!(file2, "Second file")
      File.write!(file3, "Third file")

      {:ok, results} = BatchAPI.batch_extract_files([file1, file2, file3], "text/plain")

      assert length(results) == 3
      assert Enum.at(results, 0).content == "First file"
      assert Enum.at(results, 1).content == "Second file"
      assert Enum.at(results, 2).content == "Third file"
    end

    @tag :unit
    @tag :integration
    test "batch_extract_files! returns list directly on success" do
      {:ok, dir} = create_temp_dir()

      file1 = Path.join(dir, "test1.txt")
      file2 = Path.join(dir, "test2.txt")

      File.write!(file1, "First")
      File.write!(file2, "Second")

      results = BatchAPI.batch_extract_files!([file1, file2], "text/plain")

      assert is_list(results)
      assert length(results) == 2
      refute match?({:ok, _}, results)  # Should return list directly, not tuple
    end

    @tag :unit
    test "batch_extract_bytes with list of MIME types" do
      data_list = ["Text content", "More text"]
      mime_types = ["text/plain", "text/plain"]

      {:ok, results} = BatchAPI.batch_extract_bytes(data_list, mime_types)

      assert length(results) == 2
      assert Enum.at(results, 0).content == "Text content"
      assert Enum.at(results, 1).content == "More text"
    end
  end

  describe "result structure validation" do
    @tag :unit
    test "batch results contain expected fields" do
      data_list = ["Content"]
      {:ok, results} = BatchAPI.batch_extract_bytes(data_list, "text/plain")

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
    test "all batch results are ExtractionResult structs" do
      data_list = ["Content 1", "Content 2"]
      {:ok, results} = BatchAPI.batch_extract_bytes(data_list, "text/plain")

      Enum.each(results, fn result ->
        assert %Kreuzberg.ExtractionResult{} = result
      end)
    end
  end
end
