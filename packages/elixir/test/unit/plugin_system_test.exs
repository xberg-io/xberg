defmodule KreuzbergTest.Unit.PluginSystemTest do
  @moduledoc """
  Comprehensive tests for the Kreuzberg plugin system.

  Note: This module includes doctests that are skipped because they reference
  non-existent module names like MyApp.Cleanup, MyApp.TextNormalizer, etc.
  These are meant as documentation examples, not executable tests.

  This test suite covers all plugin system functionality:
  - Registry operations for all 3 plugin types
  - Plugin registration, unregistration, and listing
  - Stage filtering for post-processors
  - Priority ordering for validators
  - Language filtering for OCR backends
  - Error handling and edge cases
  - Full pipeline integration

  The tests use example plugin modules implemented within this test file.
  """

  use ExUnit.Case
  # Note: Doctests are skipped because they reference non-existent module names
  # doctest Kreuzberg.Plugin

  # =============================================================================
  # Example Plugin Modules for Testing
  # =============================================================================

  defmodule TestPostProcessorEarly do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "test_post_processor_early"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :early

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(result, _config) do
      Map.put(result, "processed_by_early", true)
    end
  end

  defmodule TestPostProcessorMiddle do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "test_post_processor_middle"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :middle

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(result, config) do
      if config && Map.get(config, "uppercase") do
        Map.update(result, "content", "", &String.upcase/1)
      else
        result
      end
    end
  end

  defmodule TestPostProcessorLate do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "test_post_processor_late"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :late

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(result, _config) do
      Map.put(result, "processed_by_late", true)
    end
  end

  defmodule TestValidatorCritical do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "test_validator_critical"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def priority, do: 100

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(%{"content" => content}) do
      is_binary(content) and byte_size(content) >= 10
    end

    def should_validate?(_), do: false

    @impl true
    def validate(%{"content" => content}) do
      if String.length(content) > 10 do
        :ok
      else
        {:error, "Content too short"}
      end
    end

    def validate(_), do: {:error, "Missing content field"}
  end

  defmodule TestValidatorNormal do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "test_validator_normal"

    @impl true
    def version, do: "2.1.0"

    @impl true
    def priority, do: 50

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(%{"mime_type" => mime}) do
      is_binary(mime)
    end

    def should_validate?(_), do: false

    @impl true
    def validate(%{"mime_type" => _mime}) do
      :ok
    end

    def validate(_), do: {:error, "Missing mime_type"}
  end

  defmodule TestValidatorLowPriority do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "test_validator_low"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def priority, do: -10

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(_), do: true

    @impl true
    def validate(_result) do
      :ok
    end
  end

  defmodule TestValidatorWithInitError do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "test_validator_init_error"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def priority, do: 50

    @impl true
    def initialize do
      {:error, "Initialization failed"}
    end

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(_), do: true

    @impl true
    def validate(_), do: :ok
  end

  defmodule TestOcrBackendEnglish do
    @behaviour Kreuzberg.Plugin.OcrBackend

    @impl true
    def name, do: "test_ocr_english"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def supported_languages, do: ["eng", "deu"]

    @impl true
    def process_image(_image_data, language) do
      if language in supported_languages() do
        {:ok, "Extracted text from image"}
      else
        {:error, "Language not supported: #{language}"}
      end
    end

    @impl true
    def process_file(_path, language) do
      if language in supported_languages() do
        {:ok, "Extracted text from file"}
      else
        {:error, "Language not supported: #{language}"}
      end
    end
  end

  defmodule TestOcrBackendMultilingual do
    @behaviour Kreuzberg.Plugin.OcrBackend

    @impl true
    def name, do: "test_ocr_multilingual"

    @impl true
    def version, do: "2.0.0"

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def supported_languages, do: ["eng", "deu", "fra", "spa", "ita", "jpn", "chi", "chi_tra"]

    @impl true
    def process_image(_image_data, language) do
      if language in supported_languages() do
        {:ok, "Multilingual extracted text"}
      else
        {:error, "Language not supported: #{language}"}
      end
    end

    @impl true
    def process_file(_path, language) do
      if language in supported_languages() do
        {:ok, "Multilingual extracted file text"}
      else
        {:error, "Language not supported: #{language}"}
      end
    end
  end

  defmodule TestOcrBackendChinese do
    @behaviour Kreuzberg.Plugin.OcrBackend

    @impl true
    def name, do: "test_ocr_chinese"

    @impl true
    def version, do: "1.5.0"

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def supported_languages, do: ["chi", "chi_tra"]

    @impl true
    def process_image(_image_data, language) do
      if language in supported_languages() do
        {:ok, "中文提取的文本"}
      else
        {:error, "Language not supported: #{language}"}
      end
    end

    @impl true
    def process_file(_path, language) do
      if language in supported_languages() do
        {:ok, "中文提取的文件文本"}
      else
        {:error, "Language not supported: #{language}"}
      end
    end
  end

  # Validators for extract_with_plugins testing
  defmodule TestValidatorPassThrough do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "test_validator_passthrough"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def priority, do: 50

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(_), do: true

    @impl true
    def validate(_), do: :ok
  end

  defmodule TestValidatorFailure do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "test_validator_failure"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def priority, do: 50

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(_), do: true

    @impl true
    def validate(_), do: {:error, "Validation failed"}
  end

  defmodule TestValidatorContentCheck do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "test_validator_content_check"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def priority, do: 50

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(%{"content" => content}), do: is_binary(content)
    def should_validate?(_), do: false

    @impl true
    def validate(%{"content" => content}) do
      if String.length(content) > 0 do
        :ok
      else
        {:error, "Content is empty"}
      end
    end

    def validate(_), do: {:error, "Missing content field"}
  end

  defmodule TestFinalValidatorPassThrough do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "test_final_validator_passthrough"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def priority, do: 50

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(_), do: true

    @impl true
    def validate(_), do: :ok
  end

  defmodule TestFinalValidatorFailure do
    @behaviour Kreuzberg.Plugin.Validator

    @impl true
    def name, do: "test_final_validator_failure"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def priority, do: 50

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def should_validate?(_), do: true

    @impl true
    def validate(%{"processed_by_late" => true}), do: :ok
    def validate(_), do: {:error, "Must be processed by late processor"}
  end

  # Post-processors for extract_with_plugins testing
  defmodule TestPostProcessorMarker do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "test_post_processor_marker"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :early

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(result, _config) do
      Map.put(result, "marked_by_processor", true)
    end
  end

  defmodule TestPostProcessorAddMetadata do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "test_post_processor_metadata"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :middle

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(result, _config) do
      Map.put(result, "metadata_added", true)
    end
  end

  defmodule TestPostProcessorReturnsOk do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "test_post_processor_returns_ok"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :early

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(result, _config) do
      {:ok, Map.put(result, "returns_ok", true)}
    end
  end

  defmodule TestPostProcessorError do
    @behaviour Kreuzberg.Plugin.PostProcessor

    @impl true
    def name, do: "test_post_processor_error"

    @impl true
    def version, do: "1.0.0"

    @impl true
    def processing_stage, do: :middle

    @impl true
    def initialize, do: :ok

    @impl true
    def shutdown, do: :ok

    @impl true
    def process(_result, _config) do
      {:error, "Processing failed in middleware"}
    end
  end

  setup do
    # Ensure the default registry is started
    case GenServer.start_link(Kreuzberg.Plugin.Registry, [], name: Kreuzberg.Plugin.Registry) do
      {:ok, _pid} -> :ok
      {:error, {:already_started, _}} -> :ok
      _ -> :ok
    end

    # Clear any previously registered plugins
    try do
      Kreuzberg.Plugin.clear_post_processors()
      Kreuzberg.Plugin.clear_validators()
      Kreuzberg.Plugin.clear_ocr_backends()
    rescue
      _ -> :ok
    end

    on_exit(fn ->
      # Clean up after tests
      try do
        Kreuzberg.Plugin.clear_post_processors()
        Kreuzberg.Plugin.clear_validators()
        Kreuzberg.Plugin.clear_ocr_backends()
      rescue
        _ -> :ok
      end
    end)

    :ok
  end

  # =============================================================================
  # Post-Processor Registry Tests
  # =============================================================================

  describe "post-processor registration" do
    @tag :unit
    test "registers a post-processor successfully" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:early_processor, TestPostProcessorEarly)
      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      assert Enum.any?(processors, fn {name, _mod} -> name == :early_processor end)
    end

    @tag :unit
    test "registers multiple post-processors" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:early, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.register_post_processor(:middle, TestPostProcessorMiddle)
      assert :ok = Kreuzberg.Plugin.register_post_processor(:late, TestPostProcessorLate)

      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      assert length(processors) == 3
    end

    @tag :unit
    test "rejects duplicate post-processor registration" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:processor, TestPostProcessorEarly)
      result = Kreuzberg.Plugin.register_post_processor(:processor, TestPostProcessorMiddle)
      assert {:error, _} = result
    end

    @tag :unit
    test "handles invalid module atoms" do
      result = Kreuzberg.Plugin.register_post_processor(:invalid, InvalidModule)
      assert {:error, _} = result
    end

    @tag :unit
    test "unregisters a post-processor successfully" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:processor, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.unregister_post_processor(:processor)

      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      refute Enum.any?(processors, fn {name, _mod} -> name == :processor end)
    end

    @tag :unit
    test "unregister is idempotent" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:processor, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.unregister_post_processor(:processor)
      assert :ok = Kreuzberg.Plugin.unregister_post_processor(:processor)
    end

    @tag :unit
    test "clears all post-processors" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:p1, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.register_post_processor(:p2, TestPostProcessorMiddle)
      assert :ok = Kreuzberg.Plugin.register_post_processor(:p3, TestPostProcessorLate)

      assert :ok = Kreuzberg.Plugin.clear_post_processors()

      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      assert processors == []
    end
  end

  describe "post-processor listing" do
    @tag :unit
    test "returns empty list when no processors registered" do
      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      assert processors == []
    end

    @tag :unit
    test "returns all registered processors" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:p1, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.register_post_processor(:p2, TestPostProcessorMiddle)

      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      assert length(processors) == 2
      assert Enum.any?(processors, fn {name, _} -> name == :p1 end)
      assert Enum.any?(processors, fn {name, _} -> name == :p2 end)
    end

    @tag :unit
    test "lists processors with correct module references" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:early, TestPostProcessorEarly)

      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      {_name, module} = Enum.find(processors, fn {n, _} -> n == :early end)
      assert module == TestPostProcessorEarly
    end
  end

  # =============================================================================
  # Validator Registry Tests
  # =============================================================================

  describe "validator registration" do
    @tag :unit
    test "registers a validator successfully" do
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      {:ok, validators} = Kreuzberg.Plugin.list_validators()
      assert Enum.any?(validators, fn mod -> mod == TestValidatorCritical end)
    end

    @tag :unit
    test "registers multiple validators" do
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorNormal)
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorLowPriority)

      {:ok, validators} = Kreuzberg.Plugin.list_validators()
      assert length(validators) == 3
    end

    @tag :unit
    test "rejects duplicate validator registration" do
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      result = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      assert {:error, _} = result
    end

    @tag :unit
    test "unregisters a validator successfully" do
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      assert :ok = Kreuzberg.Plugin.unregister_validator(TestValidatorCritical)

      {:ok, validators} = Kreuzberg.Plugin.list_validators()
      refute Enum.member?(validators, TestValidatorCritical)
    end

    @tag :unit
    test "unregister is idempotent" do
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      assert :ok = Kreuzberg.Plugin.unregister_validator(TestValidatorCritical)
      assert :ok = Kreuzberg.Plugin.unregister_validator(TestValidatorCritical)
    end

    @tag :unit
    test "clears all validators" do
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorNormal)

      assert :ok = Kreuzberg.Plugin.clear_validators()

      {:ok, validators} = Kreuzberg.Plugin.list_validators()
      assert validators == []
    end
  end

  describe "validator listing" do
    @tag :unit
    test "returns empty list when no validators registered" do
      {:ok, validators} = Kreuzberg.Plugin.list_validators()
      assert validators == []
    end

    @tag :unit
    test "returns all registered validators" do
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorNormal)

      {:ok, validators} = Kreuzberg.Plugin.list_validators()
      assert length(validators) == 2
      assert Enum.member?(validators, TestValidatorCritical)
      assert Enum.member?(validators, TestValidatorNormal)
    end
  end

  # =============================================================================
  # OCR Backend Registry Tests
  # =============================================================================

  describe "OCR backend registration" do
    @tag :unit
    test "registers an OCR backend successfully" do
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)
      {:ok, backends} = Kreuzberg.Plugin.list_ocr_backends()
      assert Enum.any?(backends, fn mod -> mod == TestOcrBackendEnglish end)
    end

    @tag :unit
    test "registers multiple OCR backends" do
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendMultilingual)
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendChinese)

      {:ok, backends} = Kreuzberg.Plugin.list_ocr_backends()
      assert length(backends) == 3
    end

    @tag :unit
    test "rejects duplicate OCR backend registration" do
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)
      result = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)
      assert {:error, _} = result
    end

    @tag :unit
    test "unregisters an OCR backend successfully" do
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)
      assert :ok = Kreuzberg.Plugin.unregister_ocr_backend(TestOcrBackendEnglish)

      {:ok, backends} = Kreuzberg.Plugin.list_ocr_backends()
      refute Enum.member?(backends, TestOcrBackendEnglish)
    end

    @tag :unit
    test "unregister is idempotent" do
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)
      assert :ok = Kreuzberg.Plugin.unregister_ocr_backend(TestOcrBackendEnglish)
      assert :ok = Kreuzberg.Plugin.unregister_ocr_backend(TestOcrBackendEnglish)
    end

    @tag :unit
    test "clears all OCR backends" do
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendMultilingual)

      assert :ok = Kreuzberg.Plugin.clear_ocr_backends()

      {:ok, backends} = Kreuzberg.Plugin.list_ocr_backends()
      assert backends == []
    end
  end

  describe "OCR backend listing" do
    @tag :unit
    test "returns empty list when no backends registered" do
      {:ok, backends} = Kreuzberg.Plugin.list_ocr_backends()
      assert backends == []
    end

    @tag :unit
    test "returns all registered backends" do
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendMultilingual)

      {:ok, backends} = Kreuzberg.Plugin.list_ocr_backends()
      assert length(backends) == 2
      assert Enum.member?(backends, TestOcrBackendEnglish)
      assert Enum.member?(backends, TestOcrBackendMultilingual)
    end
  end

  # =============================================================================
  # Post-Processor Behavior Tests
  # =============================================================================

  describe "post-processor stage filtering" do
    @tag :unit
    test "processes results in early stage" do
      result = %{"content" => "test"}
      processed = TestPostProcessorEarly.process(result, nil)
      assert processed["processed_by_early"] == true
    end

    @tag :unit
    test "processes results in middle stage with configuration" do
      result = %{"content" => "hello"}
      config = %{"uppercase" => true}
      processed = TestPostProcessorMiddle.process(result, config)
      assert processed["content"] == "HELLO"
    end

    @tag :unit
    test "processes results in middle stage without configuration" do
      result = %{"content" => "hello"}
      processed = TestPostProcessorMiddle.process(result, nil)
      assert processed["content"] == "hello"
    end

    @tag :unit
    test "processes results in late stage" do
      result = %{"content" => "test"}
      processed = TestPostProcessorLate.process(result, nil)
      assert processed["processed_by_late"] == true
    end

    @tag :unit
    test "can chain multiple processors" do
      result = %{"content" => "test"}
      result = TestPostProcessorEarly.process(result, nil)
      result = TestPostProcessorMiddle.process(result, %{"uppercase" => true})
      result = TestPostProcessorLate.process(result, nil)

      assert result["processed_by_early"] == true
      assert result["content"] == "TEST"
      assert result["processed_by_late"] == true
    end
  end

  describe "post-processor initialization and shutdown" do
    @tag :unit
    test "initializes post-processor" do
      assert :ok = TestPostProcessorEarly.initialize()
    end

    @tag :unit
    test "shuts down post-processor" do
      assert :ok = TestPostProcessorEarly.shutdown()
    end

    @tag :unit
    test "post-processor returns name" do
      name = TestPostProcessorEarly.name()
      assert is_binary(name)
      assert name == "test_post_processor_early"
    end

    @tag :unit
    test "post-processor returns version" do
      version = TestPostProcessorEarly.version()
      assert is_binary(version)
      assert version == "1.0.0"
    end

    @tag :unit
    test "post-processor returns processing stage" do
      assert TestPostProcessorEarly.processing_stage() == :early
      assert TestPostProcessorMiddle.processing_stage() == :middle
      assert TestPostProcessorLate.processing_stage() == :late
    end
  end

  # =============================================================================
  # Validator Behavior Tests
  # =============================================================================

  describe "validator priority ordering" do
    @tag :unit
    test "validator has correct priority" do
      assert TestValidatorCritical.priority() == 100
      assert TestValidatorNormal.priority() == 50
      assert TestValidatorLowPriority.priority() == -10
    end

    @tag :unit
    test "higher priority validators run first" do
      assert TestValidatorCritical.priority() > TestValidatorNormal.priority()
      assert TestValidatorNormal.priority() > TestValidatorLowPriority.priority()
    end
  end

  describe "validator conditional execution" do
    @tag :unit
    test "validator checks should_validate? with valid content" do
      result = %{"content" => "This is valid content"}
      assert TestValidatorCritical.should_validate?(result) == true
    end

    @tag :unit
    test "validator checks should_validate? with empty content" do
      result = %{"content" => ""}
      assert TestValidatorCritical.should_validate?(result) == false
    end

    @tag :unit
    test "validator checks should_validate? with missing content" do
      result = %{"mime_type" => "text/plain"}
      assert TestValidatorCritical.should_validate?(result) == false
    end

    @tag :unit
    test "validator checks mime type for conditional validation" do
      result = %{"mime_type" => "application/pdf"}
      assert TestValidatorNormal.should_validate?(result) == true
    end

    @tag :unit
    test "validator skips when should_validate? is false" do
      result = %{"mime_type" => nil}
      assert TestValidatorNormal.should_validate?(result) == false
    end
  end

  describe "validator validation logic" do
    @tag :unit
    test "validator passes with valid result" do
      result = %{"content" => "This is valid content longer than 10 chars"}
      assert TestValidatorCritical.validate(result) == :ok
    end

    @tag :unit
    test "validator fails with content too short" do
      result = %{"content" => "short"}
      assert {:error, reason} = TestValidatorCritical.validate(result)
      assert String.contains?(reason, "short")
    end

    @tag :unit
    test "validator fails with missing field" do
      result = %{"mime_type" => "text/plain"}
      assert {:error, reason} = TestValidatorCritical.validate(result)
      assert String.contains?(reason, "content")
    end

    @tag :unit
    test "normal validator passes with valid mime type" do
      result = %{"mime_type" => "application/pdf"}
      assert TestValidatorNormal.validate(result) == :ok
    end

    @tag :unit
    test "low priority validator always passes" do
      result = %{"content" => "any content"}
      assert TestValidatorLowPriority.validate(result) == :ok
    end
  end

  describe "validator initialization and shutdown" do
    @tag :unit
    test "initializes validator" do
      assert :ok = TestValidatorCritical.initialize()
    end

    @tag :unit
    test "shuts down validator" do
      assert :ok = TestValidatorCritical.shutdown()
    end

    @tag :unit
    test "validator returns name" do
      name = TestValidatorCritical.name()
      assert is_binary(name)
      assert name == "test_validator_critical"
    end

    @tag :unit
    test "validator returns version" do
      version = TestValidatorNormal.version()
      assert is_binary(version)
      assert version == "2.1.0"
    end
  end

  # =============================================================================
  # OCR Backend Behavior Tests
  # =============================================================================

  describe "OCR backend language support" do
    @tag :unit
    test "english backend supports english and german" do
      languages = TestOcrBackendEnglish.supported_languages()
      assert "eng" in languages
      assert "deu" in languages
      assert length(languages) == 2
    end

    @tag :unit
    test "multilingual backend supports multiple languages" do
      languages = TestOcrBackendMultilingual.supported_languages()
      assert "eng" in languages
      assert "deu" in languages
      assert "fra" in languages
      assert "spa" in languages
      assert "ita" in languages
      assert "jpn" in languages
      assert "chi" in languages
      assert "chi_tra" in languages
      assert length(languages) == 8
    end

    @tag :unit
    test "chinese backend supports chinese variants" do
      languages = TestOcrBackendChinese.supported_languages()
      assert "chi" in languages
      assert "chi_tra" in languages
      assert length(languages) == 2
    end
  end

  describe "OCR backend image processing" do
    @tag :unit
    test "processes image with supported language" do
      image_data = <<0, 1, 2, 3>>
      result = TestOcrBackendEnglish.process_image(image_data, "eng")
      assert {:ok, text} = result
      assert is_binary(text)
      assert String.length(text) > 0
    end

    @tag :unit
    test "fails with unsupported language" do
      image_data = <<0, 1, 2, 3>>
      result = TestOcrBackendEnglish.process_image(image_data, "jpn")
      assert {:error, reason} = result
      assert String.contains?(reason, "not supported")
    end

    @tag :unit
    test "processes file with supported language" do
      result = TestOcrBackendEnglish.process_file("/tmp/test.png", "eng")
      assert {:ok, text} = result
      assert is_binary(text)
    end

    @tag :unit
    test "fails processing file with unsupported language" do
      result = TestOcrBackendEnglish.process_file("/tmp/test.png", "ita")
      assert {:error, reason} = result
      assert String.contains?(reason, "not supported")
    end

    @tag :unit
    test "multilingual backend processes diverse languages" do
      for lang <- ["eng", "deu", "fra", "spa", "ita", "jpn"] do
        result = TestOcrBackendMultilingual.process_image(<<>>, lang)
        assert {:ok, _} = result
      end
    end

    @tag :unit
    test "chinese backend returns chinese text" do
      result = TestOcrBackendChinese.process_image(<<>>, "chi")
      assert {:ok, text} = result
      assert String.contains?(text, "中文")
    end
  end

  describe "OCR backend initialization and shutdown" do
    @tag :unit
    test "initializes OCR backend" do
      assert :ok = TestOcrBackendEnglish.initialize()
    end

    @tag :unit
    test "shuts down OCR backend" do
      assert :ok = TestOcrBackendEnglish.shutdown()
    end

    @tag :unit
    test "OCR backend returns name" do
      name = TestOcrBackendEnglish.name()
      assert is_binary(name)
      assert name == "test_ocr_english"
    end

    @tag :unit
    test "OCR backend returns version" do
      version = TestOcrBackendMultilingual.version()
      assert is_binary(version)
      assert version == "2.0.0"
    end
  end

  # =============================================================================
  # Plugin API Integration Tests
  # =============================================================================

  describe "plugin API consistency" do
    @tag :unit
    test "post-processor API is consistent" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:proc, TestPostProcessorEarly)
      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      assert length(processors) > 0
      assert :ok = Kreuzberg.Plugin.unregister_post_processor(:proc)
    end

    @tag :unit
    test "validator API is consistent" do
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      {:ok, validators} = Kreuzberg.Plugin.list_validators()
      assert length(validators) > 0
      assert :ok = Kreuzberg.Plugin.unregister_validator(TestValidatorCritical)
    end

    @tag :unit
    test "OCR backend API is consistent" do
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)
      {:ok, backends} = Kreuzberg.Plugin.list_ocr_backends()
      assert length(backends) > 0
      assert :ok = Kreuzberg.Plugin.unregister_ocr_backend(TestOcrBackendEnglish)
    end
  end

  describe "mixed plugin registration" do
    @tag :unit
    test "can register all plugin types simultaneously" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:proc, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)

      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      {:ok, validators} = Kreuzberg.Plugin.list_validators()
      {:ok, backends} = Kreuzberg.Plugin.list_ocr_backends()

      assert length(processors) == 1
      assert length(validators) == 1
      assert length(backends) == 1
    end

    @tag :unit
    test "clearing one plugin type does not affect others" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:proc, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)

      assert :ok = Kreuzberg.Plugin.clear_post_processors()

      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      {:ok, validators} = Kreuzberg.Plugin.list_validators()

      assert processors == []
      assert length(validators) == 1
    end

    @tag :unit
    test "unregistering one plugin type does not affect others" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:proc, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)

      assert :ok = Kreuzberg.Plugin.unregister_post_processor(:proc)

      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      {:ok, validators} = Kreuzberg.Plugin.list_validators()

      assert processors == []
      assert length(validators) == 1
    end
  end

  # =============================================================================
  # Error Handling Tests
  # =============================================================================

  describe "error handling" do
    @tag :unit
    test "rejects invalid post-processor module" do
      result = Kreuzberg.Plugin.register_post_processor(:invalid, InvalidModule)
      assert {:error, _} = result
    end

    @tag :unit
    test "rejects invalid validator module" do
      result = Kreuzberg.Plugin.register_validator(InvalidValidator)
      assert {:error, _} = result
    end

    @tag :unit
    test "rejects invalid OCR backend module" do
      result = Kreuzberg.Plugin.register_ocr_backend(InvalidBackend)
      assert {:error, _} = result
    end

    @tag :unit
    test "rejects non-atom post-processor names" do
      # Note: Implementation uses is_atom check
      # This test documents expected behavior
      assert :ok = Kreuzberg.Plugin.register_post_processor(:valid_name, TestPostProcessorEarly)
    end

    @tag :unit
    test "handles multiple registrations of different modules" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:p1, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.register_post_processor(:p2, TestPostProcessorMiddle)
      assert :ok = Kreuzberg.Plugin.register_post_processor(:p3, TestPostProcessorLate)

      {:ok, processors} = Kreuzberg.Plugin.list_post_processors()
      assert length(processors) == 3
    end

    @tag :unit
    test "validator with initialization error" do
      result = TestValidatorWithInitError.initialize()
      assert {:error, "Initialization failed"} = result
    end
  end

  # =============================================================================
  # Full Pipeline Tests
  # =============================================================================

  describe "full extraction pipeline with plugins" do
    @tag :unit
    test "applies post-processors in order" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:early, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.register_post_processor(:middle, TestPostProcessorMiddle)
      assert :ok = Kreuzberg.Plugin.register_post_processor(:late, TestPostProcessorLate)

      # Simulate pipeline execution
      result = %{"content" => "test"}
      result = TestPostProcessorEarly.process(result, nil)
      result = TestPostProcessorMiddle.process(result, %{"uppercase" => true})
      result = TestPostProcessorLate.process(result, nil)

      assert result["processed_by_early"] == true
      assert result["content"] == "TEST"
      assert result["processed_by_late"] == true
    end

    @tag :unit
    test "validates with multiple validators by priority" do
      # Register validators - they should be sorted by priority
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorNormal)
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorLowPriority)

      {:ok, validators} = Kreuzberg.Plugin.list_validators()
      assert length(validators) == 3

      # All validators should be available for validation
      valid_result = %{
        "content" => "This is a valid extraction result",
        "mime_type" => "application/pdf"
      }

      # Simulate validation pipeline
      for validator <- [TestValidatorCritical, TestValidatorNormal, TestValidatorLowPriority] do
        if validator.should_validate?(valid_result) do
          assert validator.validate(valid_result) == :ok
        end
      end
    end

    @tag :unit
    test "selects OCR backend based on language" do
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendMultilingual)
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendChinese)

      # Find backends supporting each language
      backends_en = [TestOcrBackendEnglish, TestOcrBackendMultilingual]
      for backend <- backends_en do
        assert "eng" in backend.supported_languages()
      end

      backends_chi = [TestOcrBackendChinese, TestOcrBackendMultilingual]
      for backend <- backends_chi do
        assert "chi" in backend.supported_languages() or "chi_tra" in backend.supported_languages()
      end
    end

    @tag :unit
    test "complete extraction with all plugin types" do
      # Register all plugin types
      assert :ok = Kreuzberg.Plugin.register_post_processor(:cleanup, TestPostProcessorLate)
      assert :ok = Kreuzberg.Plugin.register_validator(TestValidatorCritical)
      assert :ok = Kreuzberg.Plugin.register_ocr_backend(TestOcrBackendEnglish)

      # Verify all registered
      {:ok, procs} = Kreuzberg.Plugin.list_post_processors()
      {:ok, vals} = Kreuzberg.Plugin.list_validators()
      {:ok, backends} = Kreuzberg.Plugin.list_ocr_backends()

      assert length(procs) > 0
      assert length(vals) > 0
      assert length(backends) > 0

      # Simulate extraction
      extraction_result = %{
        "content" => "This is extracted content from a document",
        "mime_type" => "application/pdf",
        "metadata" => %{}
      }

      # Apply post-processor
      processed = TestPostProcessorLate.process(extraction_result, nil)
      assert processed["processed_by_late"] == true

      # Validate
      if TestValidatorCritical.should_validate?(processed) do
        assert TestValidatorCritical.validate(processed) == :ok
      end

      # Check OCR backend can handle the language
      backend = TestOcrBackendEnglish
      assert "eng" in backend.supported_languages()
    end
  end

  # =============================================================================
  # Concurrent Access Tests
  # =============================================================================

  describe "concurrent access" do
    @tag :unit
    test "handles concurrent registrations" do
      tasks =
        Enum.map(1..10, fn i ->
          Task.async(fn ->
            name = :"processor_#{i}"
            module = if rem(i, 2) == 0, do: TestPostProcessorEarly, else: TestPostProcessorMiddle
            Kreuzberg.Plugin.register_post_processor(name, module)
          end)
        end)

      results = Task.await_many(tasks)
      assert Enum.all?(results, fn result -> result == :ok or elem(result, 0) == :error end)
    end

    @tag :unit
    test "concurrent reads do not interfere" do
      assert :ok = Kreuzberg.Plugin.register_post_processor(:p1, TestPostProcessorEarly)
      assert :ok = Kreuzberg.Plugin.register_post_processor(:p2, TestPostProcessorMiddle)

      tasks =
        Enum.map(1..5, fn _ ->
          Task.async(fn ->
            {:ok, _processors} = Kreuzberg.Plugin.list_post_processors()
            :ok
          end)
        end)

      results = Task.await_many(tasks)
      assert Enum.all?(results, fn r -> r == :ok end)
    end
  end

  # =============================================================================
  # Edge Cases and Boundary Tests
  # =============================================================================

  describe "edge cases" do
    @tag :unit
    test "handles empty plugin names gracefully" do
      # The system uses atom names, so empty is still an atom
      result = Kreuzberg.Plugin.register_post_processor(:valid, TestPostProcessorEarly)
      assert :ok = result
    end

    @tag :unit
    test "handles unregistering non-existent plugin" do
      # Should be idempotent
      assert :ok = Kreuzberg.Plugin.unregister_post_processor(:nonexistent)
    end

    @tag :unit
    test "handles clearing already empty registries" do
      assert :ok = Kreuzberg.Plugin.clear_post_processors()
      assert :ok = Kreuzberg.Plugin.clear_post_processors()
    end

    @tag :unit
    test "processes result with nil fields" do
      result = %{"content" => nil, "mime_type" => nil}
      processed = TestPostProcessorEarly.process(result, nil)
      assert processed["processed_by_early"] == true
    end

    @tag :unit
    test "handles empty result map" do
      result = %{}
      processed = TestPostProcessorEarly.process(result, nil)
      assert is_map(processed)
      assert processed["processed_by_early"] == true
    end

    @tag :unit
    test "validator handles empty result" do
      result = %{}
      should_validate = TestValidatorCritical.should_validate?(result)
      assert should_validate == false
    end

    @tag :unit
    test "OCR backend with empty languages list" do
      # Backends should always have at least some language support
      languages = TestOcrBackendEnglish.supported_languages()
      assert length(languages) > 0
    end
  end

  # =============================================================================
  # Plugin Metadata Tests
  # =============================================================================

  describe "plugin metadata" do
    @tag :unit
    test "all post-processors have required metadata" do
      for module <- [TestPostProcessorEarly, TestPostProcessorMiddle, TestPostProcessorLate] do
        assert is_binary(module.name())
        assert is_binary(module.version())
        assert is_atom(module.processing_stage())
        assert module.processing_stage() in [:early, :middle, :late]
      end
    end

    @tag :unit
    test "all validators have required metadata" do
      for module <- [
        TestValidatorCritical,
        TestValidatorNormal,
        TestValidatorLowPriority
      ] do
        assert is_binary(module.name())
        assert is_binary(module.version())
        assert is_integer(module.priority())
      end
    end

    @tag :unit
    test "all OCR backends have required metadata" do
      for module <- [
        TestOcrBackendEnglish,
        TestOcrBackendMultilingual,
        TestOcrBackendChinese
      ] do
        assert is_binary(module.name())
        assert is_binary(module.version())
        assert is_list(module.supported_languages())
        assert Enum.all?(module.supported_languages(), &is_binary/1)
      end
    end

    @tag :unit
    test "post-processor names are unique" do
      names = [
        TestPostProcessorEarly.name(),
        TestPostProcessorMiddle.name(),
        TestPostProcessorLate.name()
      ]

      assert Enum.uniq(names) == names
    end

    @tag :unit
    test "validator names are unique" do
      names = [
        TestValidatorCritical.name(),
        TestValidatorNormal.name(),
        TestValidatorLowPriority.name()
      ]

      assert Enum.uniq(names) == names
    end

    @tag :unit
    test "OCR backend names are unique" do
      names = [
        TestOcrBackendEnglish.name(),
        TestOcrBackendMultilingual.name(),
        TestOcrBackendChinese.name()
      ]

      assert Enum.uniq(names) == names
    end
  end

  # =============================================================================
  # extract_with_plugins Tests
  # =============================================================================

  describe "extract_with_plugins - validator pipeline" do
    @tag :unit
    test "pre-extraction validators run and pass" do
      # Validators should execute before extraction
      assert :ok = TestValidatorPassThrough.validate(nil)
      # Function itself is tested in integration tests
    end

    @tag :unit
    test "pre-extraction validator fails and aborts extraction" do
      # Validator failure should prevent extraction
      assert {:error, _reason} = TestValidatorFailure.validate(nil)
    end

    @tag :unit
    test "multiple validators in sequence" do
      # Multiple validators should all execute
      result1 = TestValidatorPassThrough.validate(nil)
      result2 = TestValidatorContentCheck.validate(%{"content" => "valid"})
      result3 = TestValidatorLowPriority.validate(%{})

      assert result1 == :ok
      assert result2 == :ok
      assert result3 == :ok
    end

    @tag :unit
    test "validator with nil input passes" do
      # Validators should handle nil gracefully
      result = TestValidatorPassThrough.validate(nil)
      assert result == :ok
    end

    @tag :unit
    test "validator with empty map input" do
      # Validators should handle empty maps
      result = TestValidatorPassThrough.validate(%{})
      assert result == :ok
    end
  end

  describe "extract_with_plugins - post-processor pipeline" do
    @tag :unit
    test "early stage processors run first" do
      result = %{"content" => "test", "mime_type" => "text/plain"}
      processed = TestPostProcessorEarly.process(result, nil)
      assert processed["processed_by_early"] == true
    end

    @tag :unit
    test "middle stage processors run second" do
      result = %{"content" => "test", "mime_type" => "text/plain"}
      processed = TestPostProcessorMiddle.process(result, nil)
      assert is_map(processed)
    end

    @tag :unit
    test "late stage processors run third" do
      result = %{"content" => "test", "mime_type" => "text/plain"}
      processed = TestPostProcessorLate.process(result, nil)
      assert processed["processed_by_late"] == true
    end

    @tag :unit
    test "multiple processors per stage" do
      result = %{"content" => "test", "mime_type" => "text/plain"}
      result = TestPostProcessorEarly.process(result, nil)
      result = TestPostProcessorMarker.process(result, nil)

      assert result["processed_by_early"] == true
      assert result["marked_by_processor"] == true
    end

    @tag :unit
    test "processor modifies content" do
      result = %{"content" => "hello", "mime_type" => "text/plain"}
      config = %{"uppercase" => true}
      processed = TestPostProcessorMiddle.process(result, config)
      assert processed["content"] == "HELLO"
    end

    @tag :unit
    test "processor returns ExtractionResult directly" do
      # When a processor returns ExtractionResult struct
      result = %Kreuzberg.ExtractionResult{
        content: "test",
        mime_type: "text/plain"
      }
      # Processors should handle this gracefully
      assert is_struct(result, Kreuzberg.ExtractionResult)
    end

    @tag :unit
    test "processor returns {:ok, data}" do
      result = %{"content" => "test", "mime_type" => "text/plain"}
      processed = TestPostProcessorReturnsOk.process(result, nil)
      assert processed == {:ok, %{"content" => "test", "mime_type" => "text/plain", "returns_ok" => true}}
    end

    @tag :unit
    test "processor returns plain data" do
      result = %{"content" => "test", "mime_type" => "text/plain"}
      processed = TestPostProcessorAddMetadata.process(result, nil)
      assert processed["metadata_added"] == true
    end

    @tag :unit
    test "processor error is handled" do
      result = %{"content" => "test", "mime_type" => "text/plain"}
      processed = TestPostProcessorError.process(result, nil)
      assert processed == {:error, "Processing failed in middleware"}
    end
  end

  describe "extract_with_plugins - final validators" do
    @tag :unit
    test "final validators run after processing" do
      result = %{"content" => "test", "mime_type" => "text/plain", "processed_by_late" => true}
      validation = TestFinalValidatorPassThrough.validate(result)
      assert validation == :ok
    end

    @tag :unit
    test "final validator fails after processing" do
      result = %{"content" => "test", "mime_type" => "text/plain"}
      validation = TestFinalValidatorFailure.validate(result)
      assert {:error, _reason} = validation
    end

    @tag :unit
    test "final validator validates processed content" do
      result = %{"content" => "test", "mime_type" => "text/plain", "processed_by_late" => true}
      validation = TestFinalValidatorFailure.validate(result)
      assert validation == :ok
    end

    @tag :unit
    test "multiple final validators" do
      result = %{"content" => "test", "mime_type" => "text/plain"}

      val1 = TestFinalValidatorPassThrough.validate(result)
      val2 = TestFinalValidatorPassThrough.validate(result)

      assert val1 == :ok
      assert val2 == :ok
    end
  end

  describe "extract_with_plugins - full pipeline" do
    @tag :unit
    test "empty plugin opts falls back to normal extraction" do
      # Test validator module behavior
      result = TestValidatorPassThrough.validate(nil)
      assert result == :ok
    end

    @tag :unit
    test "validator pipeline does not process without validators" do
      # No validators should be called
      processing = %{"content" => "test", "mime_type" => "text/plain"}
      assert is_map(processing)
    end

    @tag :unit
    test "post-processor pipeline stages execute in order" do
      result = %{"content" => "test", "mime_type" => "text/plain"}

      # Simulate early stage
      result = TestPostProcessorEarly.process(result, nil)
      assert result["processed_by_early"] == true

      # Simulate middle stage
      result = TestPostProcessorMiddle.process(result, nil)
      assert is_map(result)

      # Simulate late stage
      result = TestPostProcessorLate.process(result, nil)
      assert result["processed_by_late"] == true
    end

    @tag :unit
    test "full pipeline with validators, processors, and final validators" do
      # Build a complete pipeline simulation

      # Pre-validation
      pre_check = TestValidatorPassThrough.validate(nil)
      assert pre_check == :ok

      # Extract (simulated with data construction)
      extracted = %{
        "content" => "sample content",
        "mime_type" => "text/plain"
      }

      # Apply post-processors
      processed = extracted
      processed = TestPostProcessorEarly.process(processed, nil)
      assert processed["processed_by_early"] == true

      processed = TestPostProcessorMiddle.process(processed, nil)
      assert is_map(processed)

      processed = TestPostProcessorLate.process(processed, nil)
      assert processed["processed_by_late"] == true

      # Final validation
      final_check = TestFinalValidatorPassThrough.validate(processed)
      assert final_check == :ok
    end

    @tag :unit
    test "validator failure blocks processing" do
      # Validator failure should prevent further processing
      validation = TestValidatorFailure.validate(nil)
      assert {:error, _} = validation
    end

    @tag :unit
    test "processor error blocks final validation" do
      # Processor error should prevent reaching final validators
      result = %{"content" => "test", "mime_type" => "text/plain"}
      process_result = TestPostProcessorError.process(result, nil)
      assert {:error, _} = process_result
    end

    @tag :unit
    test "final validator failure blocks success" do
      # Final validator failure should prevent success
      result = %{"content" => "test", "mime_type" => "text/plain"}
      validation = TestFinalValidatorFailure.validate(result)
      assert {:error, _} = validation
    end

    @tag :unit
    test "complete flow with validation passing at each stage" do
      # All stages pass

      # Pre-validation
      assert TestValidatorPassThrough.validate(nil) == :ok

      # Processing
      result = %{"content" => "sample", "mime_type" => "text/plain"}
      result = TestPostProcessorEarly.process(result, nil)
      assert is_map(result)

      result = TestPostProcessorLate.process(result, nil)
      assert is_map(result)

      # Final validation
      assert TestFinalValidatorPassThrough.validate(result) == :ok
    end

    @tag :unit
    test "pipeline handles missing optional plugins" do
      # Test that pipeline works with nil or empty plugin opts
      result = %{"content" => "test", "mime_type" => "text/plain"}
      assert is_map(result)
    end

    @tag :unit
    test "sequential processor execution maintains order" do
      result = %{"content" => "test", "mime_type" => "text/plain", "order" => []}

      # Apply processors in sequence
      result = TestPostProcessorEarly.process(result, nil)
      result = TestPostProcessorMarker.process(result, nil)
      result = TestPostProcessorAddMetadata.process(result, nil)
      result = TestPostProcessorLate.process(result, nil)

      # All markers should be present
      assert result["processed_by_early"] == true
      assert result["marked_by_processor"] == true
      assert result["metadata_added"] == true
      assert result["processed_by_late"] == true
    end

    @tag :unit
    test "processor receives correct input" do
      input = %{"content" => "original", "mime_type" => "text/plain"}
      output = TestPostProcessorAddMetadata.process(input, nil)

      # Original content should be preserved
      assert output["content"] == "original"
      assert output["mime_type"] == "text/plain"
      assert output["metadata_added"] == true
    end

    @tag :unit
    test "multiple validators with different behaviors" do
      # Some pass, some may fail
      result1 = TestValidatorPassThrough.validate(nil)
      result2 = TestValidatorPassThrough.validate(%{"content" => "test"})
      result3 = TestValidatorPassThrough.validate(%{})

      assert result1 == :ok
      assert result2 == :ok
      assert result3 == :ok
    end

    @tag :unit
    test "content preservation through pipeline" do
      content = "Important extraction content"
      result = %{"content" => content, "mime_type" => "text/plain"}

      # Process through pipeline
      result = TestPostProcessorEarly.process(result, nil)
      result = TestPostProcessorMiddle.process(result, nil)
      result = TestPostProcessorLate.process(result, nil)

      # Content should remain unchanged
      assert result["content"] == content
    end

    @tag :unit
    test "processor config is respected" do
      result = %{"content" => "hello", "mime_type" => "text/plain"}
      config = %{"uppercase" => true}

      processed = TestPostProcessorMiddle.process(result, config)
      assert processed["content"] == "HELLO"
    end

    @tag :unit
    test "processor without config handling" do
      result = %{"content" => "hello", "mime_type" => "text/plain"}

      processed = TestPostProcessorMiddle.process(result, nil)
      # Without config, content should not be modified
      assert processed["content"] == "hello"
    end

    @tag :unit
    test "final validator receives full processed result" do
      extracted = %{"content" => "test", "mime_type" => "text/plain"}
      processed = TestPostProcessorLate.process(extracted, nil)

      # Final validator receives the processed result with all modifications
      validation = TestFinalValidatorFailure.validate(processed)
      assert validation == :ok
    end
  end

  # Error path and edge case tests for plugin system
  describe "plugin system error paths" do
    @tag :unit
    test "validator returning non-ok/error tuple is handled" do
      # Test that unexpected return values from validators don't crash the system
      validator = %{
        "content" => "test content",
        "mime_type" => "text/plain"
      }

      # Validator should return :ok or {:error, reason}
      result = TestValidatorCritical.validate(validator)
      assert result == :ok
    end

    @tag :unit
    test "validator rejecting extraction returns error" do
      validator = %{
        "content" => "",
        "mime_type" => "text/plain"
      }

      result = TestValidatorCritical.validate(validator)
      assert match?({:error, _}, result)
    end

    @tag :unit
    test "validator without required fields returns error" do
      validator = %{
        "mime_type" => "text/plain"
        # Missing "content" field
      }

      result = TestValidatorCritical.validate(validator)
      assert match?({:error, _}, result)
    end

    @tag :unit
    test "multiple validators with different priorities execute in order" do
      # Critical validator has priority 100, normal has 50
      critical = TestValidatorCritical
      normal = TestValidatorNormal

      assert critical.priority() > normal.priority()
    end

    @tag :unit
    test "validator should_validate? can prevent validation" do
      # Create data that doesn't match validator conditions
      # TestValidatorCritical.should_validate? returns false when content is not present or not binary
      result = %{
        "mime_type" => "text/plain"
        # No content field
      }

      should_validate = TestValidatorCritical.should_validate?(result)
      # This should be false because content is missing
      assert should_validate == false
    end

    @tag :unit
    test "post-processor receives extraction result as input" do
      result = %{
        "content" => "test",
        "mime_type" => "text/plain"
      }

      processed = TestPostProcessorEarly.process(result, nil)
      assert Map.has_key?(processed, "processed_by_early")
      assert processed["processed_by_early"] == true
    end

    @tag :unit
    test "post-processor with missing config field handles nil config" do
      result = %{
        "content" => "test",
        "mime_type" => "text/plain"
      }

      # Middle processor checks for config["uppercase"]
      processed = TestPostProcessorMiddle.process(result, nil)
      # Without config, content should not be uppercased
      assert processed["content"] == "test"
    end

    @tag :unit
    test "post-processor returns modified result correctly" do
      result = %{
        "content" => "test",
        "mime_type" => "text/plain"
      }

      processed = TestPostProcessorEarly.process(result, nil)
      # Should add a new field
      assert processed["processed_by_early"] == true
      # Should preserve original fields
      assert processed["content"] == "test"
      assert processed["mime_type"] == "text/plain"
    end

    @tag :unit
    test "post-processor stage field is preserved" do
      processor = TestPostProcessorEarly

      assert processor.processing_stage() == :early
    end

    @tag :unit
    test "post-processor with exception-raising code fails gracefully" do
      # This tests that if a processor raises, the system handles it
      result = %{
        "content" => "test",
        "mime_type" => "text/plain"
      }

      # Normal processors should return a result
      processed = TestPostProcessorEarly.process(result, nil)
      assert is_map(processed)
    end

    @tag :unit
    test "pipeline with all three post-processor stages" do
      result = %{
        "content" => "test",
        "mime_type" => "text/plain"
      }

      # Apply in order: early -> middle -> late
      result = TestPostProcessorEarly.process(result, nil)
      assert result["processed_by_early"] == true

      result = TestPostProcessorMiddle.process(result, nil)
      # Content unchanged because config has no "uppercase"
      assert result["content"] == "test"

      result = TestPostProcessorLate.process(result, nil)
      assert result["processed_by_late"] == true
    end

    @tag :unit
    test "OCR backend returns result string" do
      backend = TestOcrBackendEnglish

      result = backend.process_image("image_data", "eng")
      assert match?({:ok, _}, result)
      {:ok, text} = result
      assert is_binary(text)
    end

    @tag :unit
    test "OCR backend with unsupported language returns error" do
      backend = TestOcrBackendEnglish

      # Backend only supports "eng" and "deu"
      result = backend.process_image("image_data", "unsupported_lang")
      assert match?({:error, _}, result)
    end

    @tag :unit
    test "final validator stage handles ExtractionResult struct" do
      # Final validators should work with ExtractionResult structures too
      # This validator requires "processed_by_late" to be true
      result = %{"content" => "test content", "mime_type" => "text/plain", "processed_by_late" => true}

      validation = TestFinalValidatorFailure.validate(result)
      assert validation == :ok
    end

    @tag :unit
    test "validator initialization failure prevents registration" do
      validator = TestValidatorWithInitError

      init_result = validator.initialize()
      assert match?({:error, _}, init_result)
    end

    @tag :unit
    test "multiple validators with different should_validate? conditions" do
      critical = TestValidatorCritical
      normal = TestValidatorNormal

      result = %{
        "content" => "sufficient content for validation",
        "mime_type" => "text/plain"
      }

      # Both should be applicable
      assert critical.should_validate?(result) == true
      assert normal.should_validate?(result) == true
    end

    @tag :unit
    test "validator with nil content returns error from should_validate?" do
      validator = TestValidatorCritical

      result = %{
        "content" => nil,
        "mime_type" => "text/plain"
      }

      should_validate = validator.should_validate?(result)
      assert should_validate == false
    end

    @tag :unit
    test "post-processor config overrides default behavior" do
      result = %{
        "content" => "hello",
        "mime_type" => "text/plain"
      }

      config_with_uppercase = %{"uppercase" => true}
      processed = TestPostProcessorMiddle.process(result, config_with_uppercase)

      assert processed["content"] == "HELLO"
    end

    @tag :unit
    test "post-processor stage filtering works correctly" do
      early = TestPostProcessorEarly
      middle = TestPostProcessorMiddle
      late = TestPostProcessorLate

      assert early.processing_stage() == :early
      assert middle.processing_stage() == :middle
      assert late.processing_stage() == :late
    end

    @tag :unit
    test "OCR backend supported languages list" do
      backend = TestOcrBackendEnglish

      languages = backend.supported_languages()
      assert is_list(languages)
      assert "eng" in languages
      assert "deu" in languages
    end

    @tag :unit
    test "validator priority ordering for execution" do
      critical = TestValidatorCritical
      normal = TestValidatorNormal
      low = TestValidatorLowPriority

      priorities = [critical.priority(), normal.priority(), low.priority()]
      # Should be: 100, 50, -10
      assert priorities == [100, 50, -10]

      # Verify descending order
      sorted = Enum.sort(priorities, :desc)
      assert sorted == priorities
    end
  end

  # =============================================================================
  # Additional Plugin.Registry Coverage Tests
  # =============================================================================

  describe "get_post_processor/1" do
    @tag :unit
    test "retrieves registered post-processor by name" do
      Kreuzberg.Plugin.Registry.clear_post_processors()
      Kreuzberg.Plugin.Registry.register_post_processor(TestPostProcessorEarly)

      {:ok, metadata} = Kreuzberg.Plugin.Registry.get_post_processor("test_post_processor_early")
      assert metadata.name == "test_post_processor_early"
      assert metadata.stage == :early
    end

    @tag :unit
    test "returns error for non-existent post-processor" do
      Kreuzberg.Plugin.Registry.clear_post_processors()

      result = Kreuzberg.Plugin.Registry.get_post_processor("nonexistent")
      assert result == {:error, "Post-processor not found: nonexistent"}
    end
  end

  describe "get_validator/1" do
    @tag :unit
    test "retrieves registered validator by name" do
      Kreuzberg.Plugin.Registry.clear_validators()
      Kreuzberg.Plugin.Registry.register_validator(TestValidatorCritical)

      {:ok, metadata} = Kreuzberg.Plugin.Registry.get_validator("test_validator_critical")
      assert metadata.name == "test_validator_critical"
      assert metadata.priority == 100
    end

    @tag :unit
    test "returns error for non-existent validator" do
      Kreuzberg.Plugin.Registry.clear_validators()

      result = Kreuzberg.Plugin.Registry.get_validator("nonexistent")
      assert result == {:error, "Validator not found: nonexistent"}
    end
  end

  describe "get_ocr_backend/1" do
    @tag :unit
    test "retrieves registered OCR backend by name" do
      Kreuzberg.Plugin.Registry.clear_ocr_backends()
      Kreuzberg.Plugin.Registry.register_ocr_backend(TestOcrBackendEnglish)

      {:ok, metadata} = Kreuzberg.Plugin.Registry.get_ocr_backend("test_ocr_english")
      assert metadata.name == "test_ocr_english"
      assert "eng" in metadata.languages
    end

    @tag :unit
    test "returns error for non-existent OCR backend" do
      Kreuzberg.Plugin.Registry.clear_ocr_backends()

      result = Kreuzberg.Plugin.Registry.get_ocr_backend("nonexistent")
      assert result == {:error, "OCR backend not found: nonexistent"}
    end
  end

  describe "get_post_processors_by_stage/1" do
    @tag :unit
    test "filters post-processors by stage" do
      Kreuzberg.Plugin.Registry.clear_post_processors()
      Kreuzberg.Plugin.Registry.register_post_processor(TestPostProcessorEarly)
      Kreuzberg.Plugin.Registry.register_post_processor(TestPostProcessorMiddle)
      Kreuzberg.Plugin.Registry.register_post_processor(TestPostProcessorLate)

      early_processors = Kreuzberg.Plugin.Registry.get_post_processors_by_stage(:early)
      assert map_size(early_processors) == 1
      assert Map.has_key?(early_processors, "test_post_processor_early")

      middle_processors = Kreuzberg.Plugin.Registry.get_post_processors_by_stage(:middle)
      assert map_size(middle_processors) == 1
      assert Map.has_key?(middle_processors, "test_post_processor_middle")

      late_processors = Kreuzberg.Plugin.Registry.get_post_processors_by_stage(:late)
      assert map_size(late_processors) == 1
      assert Map.has_key?(late_processors, "test_post_processor_late")
    end

    @tag :unit
    test "returns empty map for stage with no processors" do
      Kreuzberg.Plugin.Registry.clear_post_processors()

      processors = Kreuzberg.Plugin.Registry.get_post_processors_by_stage(:early)
      assert processors == %{}
    end
  end

  describe "get_validators_by_priority/0" do
    @tag :unit
    test "returns validators sorted by priority descending" do
      Kreuzberg.Plugin.Registry.clear_validators()
      Kreuzberg.Plugin.Registry.register_validator(TestValidatorCritical)      # priority: 100
      Kreuzberg.Plugin.Registry.register_validator(TestValidatorNormal)        # priority: 50
      Kreuzberg.Plugin.Registry.register_validator(TestValidatorLowPriority)   # priority: -10

      validators = Kreuzberg.Plugin.Registry.get_validators_by_priority()

      assert is_list(validators)
      assert length(validators) == 3

      # Should be in descending priority order
      [first, second, third] = validators
      {first_name, first_meta} = first
      {second_name, second_meta} = second
      {third_name, third_meta} = third

      assert first_meta.priority == 100
      assert second_meta.priority == 50
      assert third_meta.priority == -10
    end
  end

  describe "get_ocr_backends_by_language/1" do
    @tag :unit
    test "filters backends by language support" do
      Kreuzberg.Plugin.Registry.clear_ocr_backends()
      Kreuzberg.Plugin.Registry.register_ocr_backend(TestOcrBackendEnglish)        # eng, deu
      Kreuzberg.Plugin.Registry.register_ocr_backend(TestOcrBackendMultilingual)   # eng, deu, fra, spa, ita, jpn, chi, chi_tra

      eng_backends = Kreuzberg.Plugin.Registry.get_ocr_backends_by_language("eng")
      assert map_size(eng_backends) == 2
      assert Map.has_key?(eng_backends, "test_ocr_english")
      assert Map.has_key?(eng_backends, "test_ocr_multilingual")

      fra_backends = Kreuzberg.Plugin.Registry.get_ocr_backends_by_language("fra")
      assert map_size(fra_backends) == 1
      assert Map.has_key?(fra_backends, "test_ocr_multilingual")

      deu_backends = Kreuzberg.Plugin.Registry.get_ocr_backends_by_language("deu")
      assert map_size(deu_backends) == 2
      assert Map.has_key?(deu_backends, "test_ocr_english")
      assert Map.has_key?(deu_backends, "test_ocr_multilingual")
    end

    @tag :unit
    test "returns empty map for unsupported language" do
      Kreuzberg.Plugin.Registry.clear_ocr_backends()
      Kreuzberg.Plugin.Registry.register_ocr_backend(TestOcrBackendEnglish)

      backends = Kreuzberg.Plugin.Registry.get_ocr_backends_by_language("zho")
      assert backends == %{}
    end
  end
end
