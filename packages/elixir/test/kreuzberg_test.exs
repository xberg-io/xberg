defmodule KreuzbergTest do
  use ExUnit.Case
  doctest Kreuzberg

  test "module is loaded" do
    assert is_atom(Kreuzberg)
  end

  test "Native module is loaded" do
    assert is_atom(Kreuzberg.Native)
  end

  # =============================================================================
  # Delegate Tests - Batch Operations
  # =============================================================================

  describe "batch_extract_files/2 delegate" do
    test "delegates to BatchAPI.batch_extract_files/2" do
      # Test actual delegation
      result = Kreuzberg.batch_extract_files([])
      assert match?({:error, _}, result)
    end
  end

  describe "batch_extract_files/3 delegate" do
    test "delegates to BatchAPI.batch_extract_files/3" do
      assert function_exported?(Kreuzberg, :batch_extract_files, 3)
    end
  end

  describe "batch_extract_files!/2 delegate" do
    test "delegates to BatchAPI.batch_extract_files!/2" do
      assert function_exported?(Kreuzberg, :batch_extract_files!, 2)
    end
  end

  describe "batch_extract_files!/3 delegate" do
    test "delegates to BatchAPI.batch_extract_files!/3" do
      assert function_exported?(Kreuzberg, :batch_extract_files!, 3)
    end
  end

  describe "batch_extract_bytes/2 delegate" do
    test "delegates to BatchAPI.batch_extract_bytes/2" do
      # Test actual delegation with empty list
      result = Kreuzberg.batch_extract_bytes([], [])
      assert match?({:error, _}, result)
    end
  end

  describe "batch_extract_bytes/3 delegate" do
    test "delegates to BatchAPI.batch_extract_bytes/3" do
      assert function_exported?(Kreuzberg, :batch_extract_bytes, 3)
    end
  end

  describe "batch_extract_bytes!/2 delegate" do
    test "delegates to BatchAPI.batch_extract_bytes!/2" do
      assert function_exported?(Kreuzberg, :batch_extract_bytes!, 2)
    end
  end

  describe "batch_extract_bytes!/3 delegate" do
    test "delegates to BatchAPI.batch_extract_bytes!/3" do
      assert function_exported?(Kreuzberg, :batch_extract_bytes!, 3)
    end
  end

  # =============================================================================
  # Delegate Tests - Async Operations
  # =============================================================================

  describe "extract_async/2 delegate" do
    test "delegates to AsyncAPI.extract_async/2" do
      assert function_exported?(Kreuzberg, :extract_async, 2)
    end
  end

  describe "extract_async/3 delegate" do
    test "delegates to AsyncAPI.extract_async/3" do
      assert function_exported?(Kreuzberg, :extract_async, 3)
    end
  end

  describe "extract_file_async/1 delegate" do
    test "delegates to AsyncAPI.extract_file_async/1" do
      assert function_exported?(Kreuzberg, :extract_file_async, 1)
    end
  end

  describe "extract_file_async/2 delegate" do
    test "delegates to AsyncAPI.extract_file_async/2" do
      assert function_exported?(Kreuzberg, :extract_file_async, 2)
    end
  end

  describe "extract_file_async/3 delegate" do
    test "delegates to AsyncAPI.extract_file_async/3" do
      assert function_exported?(Kreuzberg, :extract_file_async, 3)
    end
  end

  describe "batch_extract_files_async/1 delegate" do
    test "delegates to AsyncAPI.batch_extract_files_async/1" do
      assert function_exported?(Kreuzberg, :batch_extract_files_async, 1)
    end
  end

  describe "batch_extract_files_async/2 delegate" do
    test "delegates to AsyncAPI.batch_extract_files_async/2" do
      assert function_exported?(Kreuzberg, :batch_extract_files_async, 2)
    end
  end

  describe "batch_extract_files_async/3 delegate" do
    test "delegates to AsyncAPI.batch_extract_files_async/3" do
      assert function_exported?(Kreuzberg, :batch_extract_files_async, 3)
    end
  end

  describe "batch_extract_bytes_async/2 delegate" do
    test "delegates to AsyncAPI.batch_extract_bytes_async/2" do
      assert function_exported?(Kreuzberg, :batch_extract_bytes_async, 2)
    end
  end

  describe "batch_extract_bytes_async/3 delegate" do
    test "delegates to AsyncAPI.batch_extract_bytes_async/3" do
      assert function_exported?(Kreuzberg, :batch_extract_bytes_async, 3)
    end
  end

  # =============================================================================
  # extract_with_plugins/4 Error Paths
  # =============================================================================

  describe "extract_with_plugins/4 error handling" do
    # Helper modules for testing
    defmodule ValidatorThatFails do
      @behaviour Kreuzberg.Plugin.Validator

      def name, do: "validator_that_fails"
      def version, do: "1.0.0"
      def priority, do: 50
      def initialize, do: :ok
      def shutdown, do: :ok
      def should_validate?(_), do: true
      def validate(_), do: {:error, "Validation failed"}
    end

    defmodule ValidatorThatRaisesException do
      @behaviour Kreuzberg.Plugin.Validator

      def name, do: "validator_that_raises"
      def version, do: "1.0.0"
      def priority, do: 50
      def initialize, do: :ok
      def shutdown, do: :ok
      def should_validate?(_), do: true
      def validate(_), do: raise("Validator exception")
    end

    defmodule PostProcessorThatFails do
      @behaviour Kreuzberg.Plugin.PostProcessor

      def name, do: "processor_that_fails"
      def version, do: "1.0.0"
      def processing_stage, do: :early
      def initialize, do: :ok
      def shutdown, do: :ok
      def process(_result, _config), do: {:error, "Processing failed"}
    end

    defmodule PostProcessorThatRaisesException do
      @behaviour Kreuzberg.Plugin.PostProcessor

      def name, do: "processor_that_raises"
      def version, do: "1.0.0"
      def processing_stage, do: :middle
      def initialize, do: :ok
      def shutdown, do: :ok
      def process(_result, _config), do: raise("Processor exception")
    end

    defmodule FinalValidatorThatFails do
      @behaviour Kreuzberg.Plugin.Validator

      def name, do: "final_validator_that_fails"
      def version, do: "1.0.0"
      def priority, do: 50
      def initialize, do: :ok
      def shutdown, do: :ok
      def should_validate?(_), do: true
      def validate(_), do: {:error, "Final validation failed"}
    end

    defmodule FinalValidatorThatRaisesException do
      @behaviour Kreuzberg.Plugin.Validator

      def name, do: "final_validator_that_raises"
      def version, do: "1.0.0"
      def priority, do: 50
      def initialize, do: :ok
      def shutdown, do: :ok
      def should_validate?(_), do: true
      def validate(_), do: raise("Final validator exception")
    end

    test "validator failure aborts extraction" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        validators: [ValidatorThatFails]
      )

      assert {:error, error_msg} = result
      assert error_msg =~ "Validator"
      assert error_msg =~ "failed"
    end

    test "validator exception is caught and returned as error" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        validators: [ValidatorThatRaisesException]
      )

      assert {:error, error_msg} = result
      assert error_msg =~ "raised exception"
    end

    test "post-processor failure stops pipeline" do
      # This will succeed extraction but fail in post-processing
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        post_processors: %{early: [PostProcessorThatFails]}
      )

      assert {:error, error_msg} = result
      assert error_msg =~ "PostProcessor"
      assert error_msg =~ "failed"
    end

    test "post-processor exception is caught and returned as error" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        post_processors: %{middle: [PostProcessorThatRaisesException]}
      )

      assert {:error, error_msg} = result
      assert error_msg =~ "raised exception"
    end

    test "final validator failure after successful processing" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        final_validators: [FinalValidatorThatFails]
      )

      assert {:error, error_msg} = result
      assert error_msg =~ "Final validator"
      assert error_msg =~ "failed"
    end

    test "final validator exception is caught and returned as error" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        final_validators: [FinalValidatorThatRaisesException]
      )

      assert {:error, error_msg} = result
      assert error_msg =~ "raised exception"
    end

    test "extraction error is propagated when validators pass" do
      # Use invalid MIME type to cause extraction to fail
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "invalid/mime-type",
        nil,
        validators: []
      )

      assert {:error, _error_msg} = result
    end

    test "with all stages - validator failure short-circuits" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        validators: [ValidatorThatFails],
        post_processors: %{early: [], middle: [], late: []},
        final_validators: []
      )

      assert {:error, error_msg} = result
      assert error_msg =~ "Validator"
    end

    test "empty validators list passes validation stage" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        validators: []
      )

      assert {:ok, _extraction_result} = result
    end

    test "empty post_processors map passes processing stage" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        post_processors: %{}
      )

      assert {:ok, _extraction_result} = result
    end

    test "empty final_validators list passes final validation" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        final_validators: []
      )

      assert {:ok, _extraction_result} = result
    end

    test "no plugin_opts provided works correctly" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        []
      )

      assert {:ok, _extraction_result} = result
    end

    test "default plugin_opts works correctly" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain"
      )

      assert {:ok, _extraction_result} = result
    end

    test "with config and no plugins" do
      config = %Kreuzberg.ExtractionConfig{}
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        config,
        []
      )

      assert {:ok, _extraction_result} = result
    end

    test "post_processors as non-map is handled" do
      # When post_processors is not a map, it should be handled gracefully
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        post_processors: []
      )

      assert {:ok, _extraction_result} = result
    end

    test "post_processors with empty stage processors" do
      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        post_processors: %{early: [], middle: [], late: []}
      )

      assert {:ok, _extraction_result} = result
    end

    test "multiple post-processors in single stage, first fails" do
      defmodule ProcessorSuccess do
        @behaviour Kreuzberg.Plugin.PostProcessor
        def name, do: "success"
        def version, do: "1.0.0"
        def processing_stage, do: :early
        def initialize, do: :ok
        def shutdown, do: :ok
        def process(result), do: result
        def process(result, _config), do: result
      end

      result = Kreuzberg.extract_with_plugins(
        "test content",
        "text/plain",
        nil,
        post_processors: %{early: [PostProcessorThatFails, ProcessorSuccess]}
      )

      assert {:error, error_msg} = result
      assert error_msg =~ "PostProcessor"
    end
  end

  # =============================================================================
  # Additional Delegate Coverage Tests
  # =============================================================================

  describe "utility delegates" do
    test "detect_mime_type delegate works" do
      {:ok, result} = Kreuzberg.detect_mime_type("test content")
      assert is_binary(result)
    end

    test "detect_mime_type_from_path delegate works" do
      # Create a temp file
      path = Path.join(System.tmp_dir!(), "test_#{:erlang.unique_integer([:positive])}.txt")
      File.write!(path, "test")
      {:ok, result} = Kreuzberg.detect_mime_type_from_path(path)
      assert is_binary(result)
      File.rm!(path)
    end

    test "validate_mime_type delegate works" do
      {:ok, result} = Kreuzberg.validate_mime_type("text/plain")
      assert is_binary(result)
    end

    test "get_extensions_for_mime delegate works" do
      {:ok, result} = Kreuzberg.get_extensions_for_mime("text/plain")
      assert is_list(result)
    end

    test "list_embedding_presets delegate works" do
      {:ok, result} = Kreuzberg.list_embedding_presets()
      assert is_list(result)
    end

    test "get_embedding_preset delegate works" do
      result = Kreuzberg.get_embedding_preset("default")
      # Should return either ok or error tuple
      assert is_tuple(result)
      assert tuple_size(result) == 2
      assert elem(result, 0) in [:ok, :error]
    end

    test "classify_error delegate works" do
      result = Kreuzberg.classify_error("some error")
      assert is_atom(result)
    end

    test "get_error_details delegate works" do
      {:ok, result} = Kreuzberg.get_error_details()
      assert is_map(result)
    end
  end

  describe "cache delegates" do
    test "cache_stats delegate works" do
      result = Kreuzberg.cache_stats()
      assert match?({:ok, _}, result)
    end

    test "cache_stats! delegate works" do
      result = Kreuzberg.cache_stats!()
      assert is_map(result)
    end

    test "clear_cache delegate works" do
      result = Kreuzberg.clear_cache()
      assert result == :ok
    end

    test "clear_cache! delegate works" do
      result = Kreuzberg.clear_cache!()
      assert result == :ok
    end
  end

  describe "validator delegates" do
    test "validate_language_code delegate works" do
      result = Kreuzberg.validate_language_code("eng")
      assert result == :ok
    end

    test "validate_dpi delegate works" do
      result = Kreuzberg.validate_dpi(300)
      assert result == :ok
    end

    test "validate_confidence delegate works" do
      result = Kreuzberg.validate_confidence(0.8)
      assert result == :ok
    end

    test "validate_ocr_backend delegate works" do
      result = Kreuzberg.validate_ocr_backend("tesseract")
      assert result == :ok
    end

    test "validate_binarization_method delegate works" do
      result = Kreuzberg.validate_binarization_method("otsu")
      assert result == :ok
    end

    test "validate_tesseract_psm delegate works" do
      result = Kreuzberg.validate_tesseract_psm(3)
      assert result == :ok
    end

    test "validate_tesseract_oem delegate works" do
      result = Kreuzberg.validate_tesseract_oem(3)
      assert result == :ok
    end

    test "validate_chunking_params delegate works" do
      # Need to provide all required chunking params
      params = %{
        "max_chars" => 1000,
        "max_overlap" => 100
      }
      result = Kreuzberg.validate_chunking_params(params)
      assert result == :ok
    end
  end

  describe "extract/extract! error conversion" do
    test "extract! raises on invalid mime type" do
      assert_raise Kreuzberg.Error, fn ->
        Kreuzberg.extract!("content", "invalid/type")
      end
    end

    test "extract_file! raises on missing file" do
      assert_raise Kreuzberg.Error, fn ->
        Kreuzberg.extract_file!("/nonexistent/file.txt", "text/plain")
      end
    end
  end
end
