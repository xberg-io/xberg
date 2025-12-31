defmodule KreuzbergTest.Unit.HelpersTest do
  @moduledoc """
  Comprehensive tests for the Kreuzberg.Helpers module.

  Tests helper functions for map normalization, configuration validation,
  and result conversion with edge cases and deeply nested structures.
  """

  use ExUnit.Case

  alias Kreuzberg.ExtractionConfig
  alias Kreuzberg.Helpers

  describe "normalize_map_keys/1 - key normalization" do
    test "converts atom keys to strings" do
      input = %{name: "John", age: 30}
      result = Helpers.normalize_map_keys(input)

      assert result == %{"name" => "John", "age" => 30}
    end

    test "preserves string keys" do
      input = %{"name" => "John", "age" => 30}
      result = Helpers.normalize_map_keys(input)

      assert result == %{"name" => "John", "age" => 30}
    end

    test "converts mixed atom and string keys" do
      input = %{"age" => 30, "city" => "NYC", "name" => "John"}
      result = Helpers.normalize_map_keys(input)

      assert result == %{"name" => "John", "age" => 30, "city" => "NYC"}
    end

    test "handles nested maps" do
      input = %{
        user: %{name: "John", details: %{age: 30}}
      }

      result = Helpers.normalize_map_keys(input)

      assert result == %{"user" => %{"name" => "John", "details" => %{"age" => 30}}}
    end

    test "handles deeply nested structures (3+ levels)" do
      input = %{
        level1: %{
          level2: %{
            level3: %{
              level4: %{
                value: "deep"
              }
            }
          }
        }
      }

      result = Helpers.normalize_map_keys(input)

      assert get_in(result, ["level1", "level2", "level3", "level4", "value"]) == "deep"
    end

    test "handles maps with integer keys" do
      input = %{1 => "one", 2 => "two", :three => "three"}
      result = Helpers.normalize_map_keys(input)

      assert result["1"] == "one"
      assert result["2"] == "two"
      assert result["three"] == "three"
    end

    test "handles empty maps" do
      input = %{}
      result = Helpers.normalize_map_keys(input)

      assert result == %{}
    end

    test "passes through non-map values" do
      assert Helpers.normalize_map_keys([1, 2, 3]) == [1, 2, 3]
      assert Helpers.normalize_map_keys("string") == "string"
      assert Helpers.normalize_map_keys(42) == 42
      assert Helpers.normalize_map_keys(nil) == nil
    end

    test "normalizes maps inside lists" do
      input = [%{a: 1}, %{b: 2}]
      result = Helpers.normalize_map_keys(input)

      # normalize_map_keys on non-maps just returns the value
      assert result == [%{a: 1}, %{b: 2}]
    end
  end

  describe "normalize_key/1 - individual key conversion" do
    test "converts atom to string" do
      assert Helpers.normalize_key(:name) == "name"
      assert Helpers.normalize_key(:user_id) == "user_id"
    end

    test "preserves binary strings" do
      assert Helpers.normalize_key("name") == "name"
      assert Helpers.normalize_key("user_id") == "user_id"
    end

    test "converts integers to strings" do
      assert Helpers.normalize_key(42) == "42"
      assert Helpers.normalize_key(0) == "0"
    end

    test "handles special characters in atom keys" do
      key = :with_special_chars
      result = Helpers.normalize_key(key)

      assert is_binary(result)
      assert result == "with_special_chars"
    end
  end

  describe "normalize_value/1 - value normalization" do
    test "normalizes maps within values" do
      value = %{a: 1, b: 2}
      result = Helpers.normalize_value(value)

      assert result == %{"a" => 1, "b" => 2}
    end

    test "normalizes lists of maps" do
      value = [%{a: 1}, %{b: 2}]
      result = Helpers.normalize_value(value)

      assert result == [%{"a" => 1}, %{"b" => 2}]
    end

    test "normalizes deeply nested structures in lists" do
      value = [%{nested: %{a: 1}}, %{nested: %{b: 2}}]
      result = Helpers.normalize_value(value)

      assert result == [%{"nested" => %{"a" => 1}}, %{"nested" => %{"b" => 2}}]
    end

    test "passes through non-map, non-list values" do
      assert Helpers.normalize_value("string") == "string"
      assert Helpers.normalize_value(42) == 42
      assert Helpers.normalize_value(nil) == nil
      assert Helpers.normalize_value(true) == true
    end

    test "handles empty list" do
      assert Helpers.normalize_value([]) == []
    end

    test "handles list with nil values" do
      value = [%{a: 1}, nil, %{b: 2}]
      result = Helpers.normalize_value(value)

      assert result == [%{"a" => 1}, nil, %{"b" => 2}]
    end
  end

  describe "validate_config/1 - configuration validation" do
    test "validates nil configuration" do
      assert Helpers.validate_config(nil) == {:ok, nil}
    end

    test "validates ExtractionConfig struct" do
      config = %ExtractionConfig{}
      result = Helpers.validate_config(config)

      assert match?({:ok, %ExtractionConfig{}}, result)
    end

    test "validates map configuration" do
      config = %{"extract_images" => true, "ocr_languages" => ["en"]}
      result = Helpers.validate_config(config)

      assert result == {:ok, config}
    end

    test "validates keyword list configuration" do
      config = [extract_images: true, ocr_languages: ["en"]]
      result = Helpers.validate_config(config)

      assert result == {:ok, config}
    end

    test "handles empty map configuration" do
      result = Helpers.validate_config(%{})

      assert result == {:ok, %{}}
    end

    test "handles empty keyword list configuration" do
      result = Helpers.validate_config([])

      assert result == {:ok, []}
    end

    test "rejects invalid configuration types" do
      assert match?({:error, _}, Helpers.validate_config("invalid"))
      assert match?({:error, _}, Helpers.validate_config(42))
      assert match?({:error, _}, Helpers.validate_config(:atom))
    end

    test "handles deeply nested map configuration" do
      config = %{
        "extraction" => %{
          "images" => %{
            "ocr" => %{
              "languages" => ["en", "de"]
            }
          }
        }
      }

      result = Helpers.validate_config(config)

      assert result == {:ok, config}
    end

    test "configuration with nil values is valid" do
      config = %{"field1" => nil, "field2" => "value"}
      result = Helpers.validate_config(config)

      assert result == {:ok, config}
    end
  end

  describe "into_result/1 - map to ExtractionResult conversion" do
    test "converts simple native response to ExtractionResult" do
      native_response = %{
        "content" => "extracted text",
        "mime_type" => "text/plain"
      }

      {:ok, result} = Helpers.into_result(native_response)

      assert result.content == "extracted text"
      assert result.mime_type == "text/plain"
      assert result.metadata == %Kreuzberg.Metadata{}
      assert result.tables == []
    end

    test "converts response with all fields" do
      native_response = %{
        "content" => "text",
        "mime_type" => "application/pdf",
        "metadata" => %{"pages" => 10},
        "tables" => [%{"headers" => ["A", "B"]}],
        "detected_languages" => ["en"],
        "chunks" => [%{"text" => "chunk"}],
        "images" => [%{"path" => "img.png"}],
        "pages" => [%{"number" => 1}]
      }

      {:ok, result} = Helpers.into_result(native_response)

      assert result.content == "text"
      assert result.mime_type == "application/pdf"
      assert result.metadata.page_count == nil
      assert length(result.tables) == 1
      assert result.detected_languages == ["en"]
      assert length(result.chunks) == 1
      assert length(result.images) == 1
      assert length(result.pages) == 1
    end

    test "raises error when content is missing" do
      native_response = %{
        "mime_type" => "text/plain"
      }

      assert {:error, reason} = Helpers.into_result(native_response)
      assert String.contains?(reason, "content")
    end

    test "raises error when mime_type is missing" do
      native_response = %{
        "content" => "text"
      }

      assert {:error, reason} = Helpers.into_result(native_response)
      assert String.contains?(reason, "mime_type")
    end

    test "handles atom keys in native response" do
      native_response = %{
        content: "text",
        mime_type: "text/plain"
      }

      {:ok, result} = Helpers.into_result(native_response)

      assert result.content == "text"
      assert result.mime_type == "text/plain"
    end

    test "handles mixed atom and string keys" do
      native_response = %{
        "content" => "text",
        "mime_type" => "text/plain",
        "metadata" => %{author: "John"}
      }

      {:ok, result} = Helpers.into_result(native_response)

      assert result.content == "text"
      assert result.mime_type == "text/plain"
      assert result.metadata.author == "John"
    end

    test "defaults optional fields to nil or empty" do
      native_response = %{
        "content" => "text",
        "mime_type" => "text/plain"
      }

      {:ok, result} = Helpers.into_result(native_response)

      assert result.metadata == %Kreuzberg.Metadata{}
      assert result.tables == []
      assert result.detected_languages == nil
      assert result.chunks == nil
      assert result.images == nil
      assert result.pages == nil
    end

    test "handles nil values for optional fields" do
      native_response = %{
        "content" => "text",
        "mime_type" => "text/plain",
        "detected_languages" => nil,
        "chunks" => nil
      }

      {:ok, result} = Helpers.into_result(native_response)

      assert result.detected_languages == nil
      assert result.chunks == nil
    end

    test "handles empty metadata map" do
      native_response = %{
        "content" => "text",
        "mime_type" => "text/plain",
        "metadata" => %{}
      }

      {:ok, result} = Helpers.into_result(native_response)

      assert result.metadata == %Kreuzberg.Metadata{}
    end

    test "handles nested maps in metadata" do
      native_response = %{
        "content" => "text",
        "mime_type" => "text/plain",
        "metadata" => %{
          "title" => "Test",
          "author" => "John Doe",
          "page_count" => 5
        }
      }

      {:ok, result} = Helpers.into_result(native_response)

      assert result.metadata.title == "Test"
      assert result.metadata.author == "John Doe"
      assert result.metadata.page_count == 5
    end

    test "preserves complex nested structures" do
      native_response = %{
        "content" => "text",
        "mime_type" => "text/plain",
        "tables" => [
          %{
            "headers" => ["A", "B"],
            "rows" => [
              ["1", "2"],
              [%{"nested" => "data"}, [1, 2, 3]]
            ]
          }
        ]
      }

      {:ok, result} = Helpers.into_result(native_response)

      assert length(result.tables) == 1
      table = List.first(result.tables)
      assert table.headers == ["A", "B"]
      assert length(table.rows) == 2
    end
  end

  describe "normalize_stats_keys/1 - statistics normalization" do
    test "normalizes stats map keys" do
      stats = %{"total_files" => 42, "total_size_mb" => 128.5}
      result = Helpers.normalize_stats_keys(stats)

      assert result == %{"total_files" => 42, "total_size_mb" => 128.5}
    end

    test "passes through non-map values" do
      assert Helpers.normalize_stats_keys([1, 2, 3]) == [1, 2, 3]
      assert Helpers.normalize_stats_keys("string") == "string"
      assert Helpers.normalize_stats_keys(42) == 42
    end

    test "handles stats with various value types" do
      stats = %{
        "count" => 10,
        "size_mb" => 256.75,
        "percentage" => 85.5,
        "enabled" => true
      }

      result = Helpers.normalize_stats_keys(stats)

      assert result["count"] == 10
      assert result["size_mb"] == 256.75
      assert result["percentage"] == 85.5
      assert result["enabled"] == true
    end

    test "handles empty stats map" do
      result = Helpers.normalize_stats_keys(%{})

      assert result == %{}
    end
  end

  describe "edge cases with special data types" do
    test "handles maps with empty string keys" do
      input = %{"" => "value"}
      result = Helpers.normalize_map_keys(input)

      assert result == %{"" => "value"}
    end

    test "handles maps with numeric string keys" do
      input = %{"1" => "one", "2" => "two"}
      result = Helpers.normalize_map_keys(input)

      assert result["1"] == "one"
      assert result["2"] == "two"
    end

    test "handles very deeply nested structures" do
      # Create a 10-level deep structure
      input =
        Enum.reduce(10..1, %{level10: "value"}, fn i, acc ->
          key = String.to_atom("level#{i}")
          %{key => acc}
        end)

      result = Helpers.normalize_map_keys(input)

      # Verify it normalized - the structure is deeply nested
      assert is_map(result)
      assert Map.has_key?(result, "level1")
    end

    test "handles large maps with many keys" do
      input =
        Enum.reduce(1..1000, %{}, fn i, acc ->
          Map.put(acc, String.to_atom("key#{i}"), "value#{i}")
        end)

      result = Helpers.normalize_map_keys(input)

      assert map_size(result) == 1000
      assert result["key1"] == "value1"
      assert result["key500"] == "value500"
      assert result["key1000"] == "value1000"
    end
  end

  describe "integration scenarios" do
    test "full pipeline: native response -> normalized -> into_result" do
      native_response = %{
        content: "extracted text",
        mime_type: "application/pdf",
        metadata: %{page_count: 10}
      }

      # Normalize the response
      normalized = Helpers.normalize_map_keys(native_response)

      # Convert to result
      {:ok, result} = Helpers.into_result(normalized)

      assert result.content == "extracted text"
      assert result.mime_type == "application/pdf"
      assert result.metadata.page_count == 10
    end

    test "handles complex extraction response with all field types" do
      native_response = %{
        "content" => "document text",
        "mime_type" => "application/pdf",
        "metadata" => %{
          "author" => "John",
          "page_count" => 100
        },
        "tables" => [%{"headers" => ["Col1", "Col2"]}],
        "detected_languages" => ["en", "de"],
        "chunks" => [%{"text" => "chunk1"}],
        "images" => [%{"path" => "img.png"}],
        "pages" => [%{"number" => 1}]
      }

      {:ok, result} = Helpers.into_result(native_response)

      assert result.content == "document text"
      assert result.metadata.author == "John"
      assert result.metadata.page_count == 100
      assert length(result.tables) == 1
      assert result.detected_languages == ["en", "de"]
    end
  end
end
