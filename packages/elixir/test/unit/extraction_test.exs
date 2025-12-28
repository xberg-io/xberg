defmodule KreuzbergTest.Unit.ExtractionTest do
  @moduledoc """
  Unit tests for Kreuzberg extraction functions.

  Tests cover:
  - extract/2: Basic extraction with success and error cases
  - extract!/2: Bang variant with direct returns and exceptions
  - extract/3: Configuration handling with struct and map inputs
  """

  use ExUnit.Case

  describe "extract/2" do
    @tag :unit
    test "returns success tuple for plain text" do
      {:ok, result} = Kreuzberg.extract("Hello world", "text/plain")

      assert %Kreuzberg.ExtractionResult{} = result
      assert result.content == "Hello world"
      assert result.mime_type == "text/plain"
    end

    @tag :unit
    test "success result has proper structure" do
      {:ok, result} = Kreuzberg.extract("Test content", "text/plain")

      assert %Kreuzberg.ExtractionResult{
               content: content,
               mime_type: mime_type,
               metadata: metadata,
               tables: tables
             } = result

      assert is_binary(content)
      assert is_binary(mime_type)
      assert is_map(metadata)
      assert is_list(tables)
    end

    @tag :unit
    test "returns error for invalid MIME type" do
      {:error, reason} = Kreuzberg.extract("data", "invalid/type")

      # Error should be a non-empty string
      assert is_binary(reason) and byte_size(reason) > 0
    end

    @tag :unit
    test "returns error tuple, not exception" do
      result = Kreuzberg.extract("data", "invalid/type")

      assert {:error, _reason} = result
    end

    @tag :unit
    test "handles empty input" do
      {:error, _reason} = Kreuzberg.extract("", "text/plain")
    end

    @tag :unit
    test "handles multiline text" do
      content = "Line 1\nLine 2\nLine 3"
      {:ok, result} = Kreuzberg.extract(content, "text/plain")

      assert result.content == content
    end

    @tag :unit
    test "handles special characters in text" do
      content = "Special chars: @#$%^&*()\nUnicode: 你好世界"
      {:ok, result} = Kreuzberg.extract(content, "text/plain")

      assert result.content == content
    end

    @tag :unit
    test "accepts binary input" do
      assert {:ok, %Kreuzberg.ExtractionResult{}} = Kreuzberg.extract("text", "text/plain")
    end

    @tag :unit
    test "accepts binary mime_type" do
      assert {:ok, %Kreuzberg.ExtractionResult{}} = Kreuzberg.extract("text", "text/plain")
    end
  end

  describe "extract!/2" do
    @tag :unit
    test "returns result directly on success" do
      result = Kreuzberg.extract!("Hello world", "text/plain")

      assert %Kreuzberg.ExtractionResult{} = result
      assert result.content == "Hello world"
      assert result.mime_type == "text/plain"
    end

    @tag :unit
    test "returns proper result structure" do
      result = Kreuzberg.extract!("Test content", "text/plain")

      assert %Kreuzberg.ExtractionResult{
               content: content,
               mime_type: mime_type,
               metadata: metadata,
               tables: tables
             } = result

      assert is_binary(content)
      assert is_binary(mime_type)
      assert is_map(metadata)
      assert is_list(tables)
    end

    @tag :unit
    test "raises Kreuzberg.Error on error" do
      assert_raise Kreuzberg.Error, fn ->
        Kreuzberg.extract!("data", "invalid/type")
      end
    end

    @tag :unit
    test "raised error contains message" do
      assert_raise Kreuzberg.Error, ~r/.+/, fn ->
        Kreuzberg.extract!("data", "invalid/type")
      end
    end

    @tag :unit
    test "does not return a tuple, returns the struct directly" do
      result = Kreuzberg.extract!("Hello world", "text/plain")

      # Should be a struct, not a tuple
      assert is_struct(result)
      refute is_tuple(result)
    end

    @tag :unit
    test "propagates error message in exception" do
      assert_raise Kreuzberg.Error, ~r/.+/, fn ->
        Kreuzberg.extract!("data", "invalid/type")
      end
    end

    @tag :unit
    test "handles empty input" do
      assert_raise Kreuzberg.Error, ~r/.+/, fn ->
        Kreuzberg.extract!("", "text/plain")
      end
    end

    @tag :unit
    test "handles multiline text" do
      content = "Line 1\nLine 2\nLine 3"
      result = Kreuzberg.extract!(content, "text/plain")

      assert result.content == content
    end
  end

  describe "extract/3 with ExtractionConfig struct" do
    @tag :unit
    test "accepts ExtractionConfig struct" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: %{"enabled" => true}
      }

      assert {:ok, %Kreuzberg.ExtractionResult{}} =
               Kreuzberg.extract("text", "text/plain", config)
    end

    @tag :unit
    test "returns success with struct config" do
      config = %Kreuzberg.ExtractionConfig{
        chunking: %{"enabled" => true, "size" => 256}
      }

      {:ok, result} = Kreuzberg.extract("Hello world", "text/plain", config)

      assert %Kreuzberg.ExtractionResult{} = result
      assert result.content == "Hello world"
    end

    @tag :unit
    test "struct config with multiple options" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: %{"enabled" => true},
        chunking: %{"size" => 512},
        language_detection: %{"enabled" => true}
      }

      {:ok, result} = Kreuzberg.extract("text", "text/plain", config)

      assert result.content == "text"
    end

    @tag :unit
    test "empty struct config works" do
      config = %Kreuzberg.ExtractionConfig{}

      {:ok, result} = Kreuzberg.extract("text", "text/plain", config)

      assert result.content == "text"
    end

    @tag :unit
    test "struct config with nil fields" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: nil,
        chunking: nil,
        language_detection: nil
      }

      {:ok, result} = Kreuzberg.extract("text", "text/plain", config)

      assert result.content == "text"
    end

    @tag :unit
    test "struct config is converted properly" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: %{"backend" => "tesseract"}
      }

      assert {:ok, %Kreuzberg.ExtractionResult{}} =
               Kreuzberg.extract("text", "text/plain", config)
    end
  end

  describe "extract/3 with map config" do
    @tag :unit
    test "accepts map config" do
      assert {:ok, %Kreuzberg.ExtractionResult{}} =
               Kreuzberg.extract("text", "text/plain", %{
                 "ocr" => %{"enabled" => true}
               })
    end

    @tag :unit
    test "returns success with map config" do
      {:ok, result} =
        Kreuzberg.extract("Hello world", "text/plain", %{
          "chunking" => %{"size" => 256}
        })

      assert %Kreuzberg.ExtractionResult{} = result
      assert result.content == "Hello world"
    end

    @tag :unit
    test "map config with string keys" do
      {:ok, result} =
        Kreuzberg.extract("text", "text/plain", %{
          "ocr" => %{"enabled" => true},
          "chunking" => %{"size" => 512}
        })

      assert result.content == "text"
    end

    @tag :unit
    test "map config with atom keys" do
      {:ok, result} =
        Kreuzberg.extract("text", "text/plain", %{
          ocr: %{"enabled" => true},
          chunking: %{"size" => 512}
        })

      assert result.content == "text"
    end

    @tag :unit
    test "empty map config works" do
      {:ok, result} = Kreuzberg.extract("text", "text/plain", %{})

      assert result.content == "text"
    end

    @tag :unit
    test "map config with nested options" do
      {:ok, result} =
        Kreuzberg.extract("text", "text/plain", %{
          "pdf_config" => %{
            "extract_text" => true,
            "preserve_formatting" => true
          }
        })

      assert result.content == "text"
    end

    @tag :unit
    test "map config does not raise error on invalid key" do
      {:ok, result} =
        Kreuzberg.extract("text", "text/plain", %{
          "unknown_option" => "value"
        })

      # Should not raise, just ignore unknown keys
      assert result.content == "text"
    end
  end

  describe "extract!/3 with config" do
    @tag :unit
    test "bang variant works with struct config" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: %{"enabled" => true}
      }

      result = Kreuzberg.extract!("text", "text/plain", config)

      assert %Kreuzberg.ExtractionResult{} = result
      assert result.content == "text"
    end

    @tag :unit
    test "bang variant works with map config" do
      result =
        Kreuzberg.extract!("text", "text/plain", %{
          "ocr" => %{"enabled" => true}
        })

      assert result.content == "text"
    end

    @tag :unit
    test "bang variant raises on error with config" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: %{"enabled" => true}
      }

      assert_raise Kreuzberg.Error, fn ->
        Kreuzberg.extract!("data", "invalid/type", config)
      end
    end
  end

  describe "extract/3 with nil config" do
    @tag :unit
    test "nil config is treated as default" do
      {:ok, result1} = Kreuzberg.extract("text", "text/plain", nil)
      {:ok, result2} = Kreuzberg.extract("text", "text/plain")

      # Both should return the same result structure
      assert result1.content == result2.content
      assert result1.mime_type == result2.mime_type
    end
  end

  describe "configuration conversion" do
    @tag :unit
    test "Config.to_map returns string keys" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: %{"enabled" => true}
      }

      map = Kreuzberg.ExtractionConfig.to_map(config)

      # Should have string keys
      assert is_map_key(map, "ocr")
      assert Map.get(map, "ocr") == %{"enabled" => true}
    end

    @tag :unit
    test "Config.to_map includes all fields" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: %{"enabled" => true},
        chunking: %{"size" => 512}
      }

      map = Kreuzberg.ExtractionConfig.to_map(config)

      # All fields should be present in the map
      assert Map.has_key?(map, "ocr")
      assert Map.has_key?(map, "chunking")
      assert Map.has_key?(map, "language_detection")
      assert Map.has_key?(map, "postprocessor")
      assert Map.has_key?(map, "images")
      assert Map.has_key?(map, "pages")
      assert Map.has_key?(map, "token_reduction")
      assert Map.has_key?(map, "keywords")
      assert Map.has_key?(map, "pdf_options")
      assert Map.has_key?(map, "use_cache")
      assert Map.has_key?(map, "enable_quality_processing")
      assert Map.has_key?(map, "force_ocr")
    end

    @tag :unit
    test "Config.to_map handles nil fields" do
      config = %Kreuzberg.ExtractionConfig{
        ocr: nil,
        chunking: nil
      }

      map = Kreuzberg.ExtractionConfig.to_map(config)

      assert map["ocr"] == nil
      assert map["chunking"] == nil
    end
  end

  describe "result structure validation" do
    @tag :unit
    test "result contains expected fields" do
      {:ok, result} = Kreuzberg.extract("text", "text/plain")

      # Verify all expected fields are present
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
    test "result content is always a binary" do
      {:ok, result} = Kreuzberg.extract("text", "text/plain")

      assert is_binary(result.content)
    end

    @tag :unit
    test "result mime_type is always a binary" do
      {:ok, result} = Kreuzberg.extract("text", "text/plain")

      assert is_binary(result.mime_type)
    end

    @tag :unit
    test "result metadata is a map" do
      {:ok, result} = Kreuzberg.extract("text", "text/plain")

      assert is_map(result.metadata)
    end

    @tag :unit
    test "result tables is a list" do
      {:ok, result} = Kreuzberg.extract("text", "text/plain")

      assert is_list(result.tables)
    end
  end

  describe "error handling and messages" do
    @tag :unit
    test "error message is descriptive" do
      {:error, reason} = Kreuzberg.extract("data", "invalid/type")

      assert is_binary(reason) and byte_size(reason) > 0
    end

    @tag :unit
    test "different invalid mime types produce errors" do
      invalid_types = [
        "invalid",
        "not/supported",
        "random/mime",
        ""
      ]

      Enum.each(invalid_types, fn mime_type ->
        result = Kreuzberg.extract("data", mime_type)
        assert {:error, _reason} = result
      end)
    end

    @tag :unit
    test "extract! error is a Kreuzberg.Error" do
      assert_raise Kreuzberg.Error, fn ->
        Kreuzberg.extract!("data", "invalid/type")
      end
    end
  end

  describe "cache operations via main Kreuzberg module" do
    @tag :unit
    test "cache_stats via Kreuzberg.cache_stats/0" do
      result = Kreuzberg.cache_stats()

      case result do
        {:ok, _} -> assert true
        {:error, _} -> assert true
        _ -> flunk("Expected tuple result")
      end
    end

    @tag :unit
    test "cache_stats! via Kreuzberg.cache_stats!/0" do
      result = Kreuzberg.cache_stats!()
      assert is_map(result)
    end

    @tag :unit
    test "clear_cache via Kreuzberg.clear_cache/0" do
      result = Kreuzberg.clear_cache()
      assert result == :ok or (is_tuple(result) and elem(result, 0) == :error)
    end

    @tag :unit
    test "clear_cache! via Kreuzberg.clear_cache!/0" do
      result = Kreuzberg.clear_cache!()
      assert result == :ok
    end
  end
end
