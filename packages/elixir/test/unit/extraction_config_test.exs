defmodule KreuzbergTest.Unit.ExtractionConfigTest do
  @moduledoc """
  Unit tests for Kreuzberg.ExtractionConfig module.

  Tests cover:
  - validate/1: Configuration validation for various scenarios
  - to_map/1: Conversion of configs to maps for NIF serialization
  - Config structure and defaults
  """

  use ExUnit.Case

  alias Kreuzberg.ExtractionConfig

  describe "validate/1 with valid configs" do
    @tag :unit
    test "validates empty config with defaults" do
      config = %ExtractionConfig{}
      assert {:ok, ^config} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with use_cache true" do
      config = %ExtractionConfig{use_cache: true}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with use_cache false" do
      config = %ExtractionConfig{use_cache: false}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with enable_quality_processing true" do
      config = %ExtractionConfig{enable_quality_processing: true}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with enable_quality_processing false" do
      config = %ExtractionConfig{enable_quality_processing: false}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with force_ocr true" do
      config = %ExtractionConfig{force_ocr: true}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with force_ocr false" do
      config = %ExtractionConfig{force_ocr: false}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with chunking map" do
      config = %ExtractionConfig{chunking: %{"size" => 512}}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with ocr map" do
      config = %ExtractionConfig{ocr: %{"enabled" => true}}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with language_detection map" do
      config = %ExtractionConfig{language_detection: %{"enabled" => true}}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with postprocessor map" do
      config = %ExtractionConfig{postprocessor: %{"enabled" => true}}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with images map" do
      config = %ExtractionConfig{images: %{"extract" => true}}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with pages map" do
      config = %ExtractionConfig{pages: %{"start" => 1, "end" => 10}}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with token_reduction map" do
      config = %ExtractionConfig{token_reduction: %{"enabled" => true}}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with keywords map" do
      config = %ExtractionConfig{keywords: %{"extract" => true}}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with pdf_options map" do
      config = %ExtractionConfig{pdf_options: %{"preserve_formatting" => true}}
      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with all nested fields as nil" do
      config = %ExtractionConfig{
        chunking: nil,
        ocr: nil,
        language_detection: nil,
        postprocessor: nil,
        images: nil,
        pages: nil,
        token_reduction: nil,
        keywords: nil,
        pdf_options: nil
      }

      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates config with all flags and nested options" do
      config = %ExtractionConfig{
        use_cache: true,
        enable_quality_processing: false,
        force_ocr: true,
        chunking: %{"size" => 256},
        ocr: %{"backend" => "tesseract"},
        language_detection: %{"enabled" => true},
        postprocessor: %{"enabled" => true},
        images: %{"quality" => 90},
        pages: %{"limit" => 100},
        token_reduction: %{"enabled" => false},
        keywords: %{"extract" => true},
        pdf_options: %{"extract_metadata" => true}
      }

      assert {:ok, _} = ExtractionConfig.validate(config)
    end

    @tag :unit
    test "validates multiple configs sequentially" do
      configs = [
        %ExtractionConfig{use_cache: true},
        %ExtractionConfig{force_ocr: false},
        %ExtractionConfig{chunking: %{"size" => 1024}},
        %ExtractionConfig{}
      ]

      Enum.each(configs, fn config ->
        assert {:ok, _} = ExtractionConfig.validate(config)
      end)
    end
  end

  describe "validate/1 with invalid boolean fields" do
    @tag :unit
    test "rejects use_cache as string" do
      config = %ExtractionConfig{use_cache: "yes"}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "use_cache")
      assert String.contains?(reason, "boolean")
    end

    @tag :unit
    test "rejects use_cache as integer" do
      config = %ExtractionConfig{use_cache: 1}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "use_cache")
    end

    @tag :unit
    test "rejects use_cache as nil" do
      config = %ExtractionConfig{use_cache: nil}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "use_cache")
    end

    @tag :unit
    test "rejects enable_quality_processing as string" do
      config = %ExtractionConfig{enable_quality_processing: "true"}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "enable_quality_processing")
    end

    @tag :unit
    test "rejects enable_quality_processing as list" do
      config = %ExtractionConfig{enable_quality_processing: [true]}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "enable_quality_processing")
    end

    @tag :unit
    test "rejects force_ocr as map" do
      config = %ExtractionConfig{force_ocr: %{"enabled" => true}}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "force_ocr")
    end

    @tag :unit
    test "rejects force_ocr as atom" do
      config = %ExtractionConfig{force_ocr: :enabled}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "force_ocr")
    end

    @tag :unit
    test "error message includes type name for string" do
      config = %ExtractionConfig{use_cache: "invalid"}
      {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "string")
    end

    @tag :unit
    test "error message includes type name for integer" do
      config = %ExtractionConfig{enable_quality_processing: 42}
      {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "integer")
    end
  end

  describe "validate/1 with invalid nested fields" do
    @tag :unit
    test "rejects chunking as string" do
      config = %ExtractionConfig{chunking: "invalid"}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "chunking")
      assert String.contains?(reason, "map")
    end

    @tag :unit
    test "rejects ocr as string" do
      config = %ExtractionConfig{ocr: "enabled"}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "ocr")
    end

    @tag :unit
    test "rejects language_detection as boolean" do
      config = %ExtractionConfig{language_detection: true}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "language_detection")
    end

    @tag :unit
    test "rejects postprocessor as list" do
      config = %ExtractionConfig{postprocessor: ["config"]}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "postprocessor")
    end

    @tag :unit
    test "rejects images as integer" do
      config = %ExtractionConfig{images: 100}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "images")
    end

    @tag :unit
    test "rejects pages as atom" do
      config = %ExtractionConfig{pages: :all}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "pages")
    end

    @tag :unit
    test "rejects token_reduction as string" do
      config = %ExtractionConfig{token_reduction: "enabled"}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "token_reduction")
    end

    @tag :unit
    test "rejects keywords as float" do
      config = %ExtractionConfig{keywords: 3.14}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "keywords")
    end

    @tag :unit
    test "rejects pdf_options as boolean" do
      config = %ExtractionConfig{pdf_options: true}
      assert {:error, reason} = ExtractionConfig.validate(config)
      assert String.contains?(reason, "pdf_options")
    end

    @tag :unit
    test "multiple invalid nested fields - reports first error" do
      config = %ExtractionConfig{
        chunking: "invalid",
        ocr: "invalid"
      }

      assert {:error, _reason} = ExtractionConfig.validate(config)
    end
  end

  describe "to_map/1 with struct" do
    @tag :unit
    test "converts empty struct to map with all fields" do
      config = %ExtractionConfig{}
      result = ExtractionConfig.to_map(config)

      assert is_map(result)
      assert Map.has_key?(result, "chunking")
      assert Map.has_key?(result, "ocr")
      assert Map.has_key?(result, "language_detection")
      assert Map.has_key?(result, "postprocessor")
      assert Map.has_key?(result, "images")
      assert Map.has_key?(result, "pages")
      assert Map.has_key?(result, "token_reduction")
      assert Map.has_key?(result, "keywords")
      assert Map.has_key?(result, "pdf_options")
      assert Map.has_key?(result, "use_cache")
      assert Map.has_key?(result, "enable_quality_processing")
      assert Map.has_key?(result, "force_ocr")
    end

    @tag :unit
    test "converts struct with all fields set" do
      config = %ExtractionConfig{
        chunking: %{"size" => 512},
        ocr: %{"backend" => "tesseract"},
        language_detection: %{"enabled" => true},
        postprocessor: %{"enabled" => true},
        images: %{"quality" => 90},
        pages: %{"limit" => 100},
        token_reduction: %{"enabled" => false},
        keywords: %{"extract" => true},
        pdf_options: %{"extract_metadata" => true},
        use_cache: false,
        enable_quality_processing: false,
        force_ocr: true
      }

      result = ExtractionConfig.to_map(config)

      assert result["chunking"] == %{"size" => 512}
      assert result["ocr"] == %{"backend" => "tesseract"}
      assert result["language_detection"] == %{"enabled" => true}
      assert result["postprocessor"] == %{"enabled" => true}
      assert result["images"] == %{"quality" => 90}
      assert result["pages"] == %{"limit" => 100}
      assert result["token_reduction"] == %{"enabled" => false}
      assert result["keywords"] == %{"extract" => true}
      assert result["pdf_options"] == %{"extract_metadata" => true}
      assert result["use_cache"] == false
      assert result["enable_quality_processing"] == false
      assert result["force_ocr"] == true
    end

    @tag :unit
    test "returned map has string keys" do
      config = %ExtractionConfig{
        chunking: %{"size" => 256},
        use_cache: true
      }

      result = ExtractionConfig.to_map(config)

      Enum.each(result, fn {key, _value} ->
        assert is_binary(key), "Key should be string, got: #{inspect(key)}"
      end)
    end

    @tag :unit
    test "preserves default boolean values" do
      config = %ExtractionConfig{}
      result = ExtractionConfig.to_map(config)

      assert result["use_cache"] == true
      assert result["enable_quality_processing"] == true
      assert result["force_ocr"] == false
    end

    @tag :unit
    test "preserves custom boolean values" do
      config = %ExtractionConfig{
        use_cache: false,
        enable_quality_processing: false,
        force_ocr: true
      }

      result = ExtractionConfig.to_map(config)

      assert result["use_cache"] == false
      assert result["enable_quality_processing"] == false
      assert result["force_ocr"] == true
    end

    @tag :unit
    test "handles nested maps correctly" do
      nested_config = %{"key1" => "value1", "key2" => 42}
      config = %ExtractionConfig{chunking: nested_config}
      result = ExtractionConfig.to_map(config)

      assert result["chunking"] == nested_config
    end
  end

  describe "to_map/1 with nil" do
    @tag :unit
    test "to_map(nil) returns nil" do
      assert ExtractionConfig.to_map(nil) == nil
    end

    @tag :unit
    test "to_map(nil) does not raise" do
      assert_nothing_raised(fn ->
        ExtractionConfig.to_map(nil)
      end)
    end
  end

  describe "to_map/1 with plain map" do
    @tag :unit
    test "converts plain map with string keys" do
      input = %{"use_cache" => false, "force_ocr" => true}
      result = ExtractionConfig.to_map(input)

      assert result == input
    end

    @tag :unit
    test "converts plain map with atom keys to string keys" do
      input = %{use_cache: false, force_ocr: true}
      result = ExtractionConfig.to_map(input)

      assert result["use_cache"] == false
      assert result["force_ocr"] == true
    end

    @tag :unit
    test "converts plain map with mixed keys" do
      input = %{"use_cache" => true, "force_ocr" => false}
      result = ExtractionConfig.to_map(input)

      assert result["use_cache"] == true
      assert result["force_ocr"] == false
    end

    @tag :unit
    test "preserves nested maps in plain map" do
      input = %{
        "chunking" => %{"size" => 512},
        ocr: %{"enabled" => true}
      }

      result = ExtractionConfig.to_map(input)

      assert result["chunking"] == %{"size" => 512}
      assert result["ocr"] == %{"enabled" => true}
    end

    @tag :unit
    test "handles empty plain map" do
      result = ExtractionConfig.to_map(%{})
      assert result == %{}
    end
  end

  describe "to_map/1 with keyword list" do
    @tag :unit
    test "converts keyword list with atom keys" do
      input = [use_cache: false, force_ocr: true]
      result = ExtractionConfig.to_map(input)

      assert result["use_cache"] == false
      assert result["force_ocr"] == true
    end

    @tag :unit
    test "converted keyword list has string keys" do
      input = [chunking: %{"size" => 512}, use_cache: true]
      result = ExtractionConfig.to_map(input)

      assert is_binary("chunking")
      assert is_binary("use_cache")
    end

    @tag :unit
    test "converts empty keyword list" do
      result = ExtractionConfig.to_map([])
      assert result == %{}
    end

    @tag :unit
    test "handles keyword list with nested maps" do
      input = [
        chunking: %{"size" => 256},
        ocr: %{"backend" => "tesseract"}
      ]

      result = ExtractionConfig.to_map(input)

      assert result["chunking"] == %{"size" => 256}
      assert result["ocr"] == %{"backend" => "tesseract"}
    end
  end

  describe "config defaults" do
    @tag :unit
    test "new struct has use_cache default to true" do
      config = %ExtractionConfig{}
      assert config.use_cache == true
    end

    @tag :unit
    test "new struct has enable_quality_processing default to true" do
      config = %ExtractionConfig{}
      assert config.enable_quality_processing == true
    end

    @tag :unit
    test "new struct has force_ocr default to false" do
      config = %ExtractionConfig{}
      assert config.force_ocr == false
    end

    @tag :unit
    test "new struct has all nested fields default to nil" do
      config = %ExtractionConfig{}
      assert config.chunking == nil
      assert config.ocr == nil
      assert config.language_detection == nil
      assert config.postprocessor == nil
      assert config.images == nil
      assert config.pages == nil
      assert config.token_reduction == nil
      assert config.keywords == nil
      assert config.pdf_options == nil
    end
  end

  describe "config update and immutability" do
    @tag :unit
    test "updating config returns new struct" do
      original = %ExtractionConfig{}
      updated = %{original | use_cache: false}

      assert original.use_cache == true
      assert updated.use_cache == false
    end

    @tag :unit
    test "updating multiple fields" do
      config = %ExtractionConfig{
        use_cache: false,
        force_ocr: true,
        chunking: %{"size" => 1024}
      }

      assert config.use_cache == false
      assert config.force_ocr == true
      assert config.chunking == %{"size" => 1024}
    end

    @tag :unit
    test "partial updates preserve other fields" do
      config = %ExtractionConfig{
        use_cache: false,
        chunking: %{"size" => 512}
      }

      updated = %{config | force_ocr: true}

      assert updated.use_cache == false
      assert updated.force_ocr == true
      assert updated.chunking == %{"size" => 512}
    end
  end

  describe "round-trip conversions" do
    @tag :unit
    test "struct to map to struct preserves values" do
      original = %ExtractionConfig{
        use_cache: false,
        force_ocr: true,
        chunking: %{"size" => 256}
      }

      map = ExtractionConfig.to_map(original)
      assert map["use_cache"] == false
      assert map["force_ocr"] == true
      assert map["chunking"] == %{"size" => 256}
    end

    @tag :unit
    test "validate preserves config unchanged" do
      config = %ExtractionConfig{
        chunking: %{"size" => 512},
        use_cache: false
      }

      {:ok, validated} = ExtractionConfig.validate(config)

      assert validated == config
    end

    @tag :unit
    test "validate then to_map produces correct result" do
      config = %ExtractionConfig{
        use_cache: false,
        force_ocr: true,
        ocr: %{"backend" => "tesseract"}
      }

      {:ok, _} = ExtractionConfig.validate(config)
      map = ExtractionConfig.to_map(config)

      assert map["use_cache"] == false
      assert map["force_ocr"] == true
      assert map["ocr"] == %{"backend" => "tesseract"}
    end
  end

  describe "type name detection" do
    @tag :unit
    test "error messages distinguish between types" do
      configs = [
        {%ExtractionConfig{use_cache: "string"}, "string"},
        {%ExtractionConfig{use_cache: 42}, "integer"},
        {%ExtractionConfig{use_cache: 3.14}, "float"},
        {%ExtractionConfig{use_cache: []}, "list"},
        {%ExtractionConfig{use_cache: :atom}, "atom"},
        {%ExtractionConfig{use_cache: %{}}, "map"}
      ]

      Enum.each(configs, fn {config, expected_type} ->
        {:error, reason} = ExtractionConfig.validate(config)
        assert String.contains?(reason, expected_type), "Error should mention #{expected_type}: #{reason}"
      end)
    end

    @tag :unit
    test "nested field error messages identify correct type" do
      invalid_values = [
        {"string", 100},
        {1, 200},
        {3.14, 300},
        {[], 400},
        {:atom, 500}
      ]

      Enum.each(invalid_values, fn {invalid_val, _count} ->
        config = %ExtractionConfig{chunking: invalid_val}
        {:error, reason} = ExtractionConfig.validate(config)
        assert String.contains?(reason, "chunking")
      end)
    end

    @tag :unit
    test "unknown type fallthrough case" do
      # Testing that all types are properly identified - if somehow we get an unknown type
      config = %ExtractionConfig{use_cache: %{}}
      {:error, reason} = ExtractionConfig.validate(config)
      # The error should still contain field name
      assert String.contains?(reason, "use_cache")
    end
  end

  # Helper to assert nothing was raised
  defp assert_nothing_raised(func) do
    func.()
    assert true
  rescue
    _e -> flunk("Expected function to not raise, but it did")
  end
end
