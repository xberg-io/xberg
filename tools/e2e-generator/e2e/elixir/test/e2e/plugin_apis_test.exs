# Auto-generated from fixtures/plugin_api/ - DO NOT EDIT

# E2E tests for plugin/config/utility APIs.

# To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang elixir

defmodule E2E.ConfigurationTest do
  use ExUnit.Case, async: false

  describe "Configuration" do
    test "Discover configuration from current or parent directories" do
      tmpdir = Path.join(System.tmp_dir!(), "kreuzberg_e2e_#{System.unique_integer([:positive])}")
      File.mkdir_p!(tmpdir)
      on_exit(fn -> File.rm_rf!(tmpdir) end)
      config_path = Path.join(tmpdir, "kreuzberg.toml")

      config_content = """
      [chunking]
      max_chars = 50
      """

      File.write!(config_path, config_content)

      subdir = Path.join(tmpdir, "subdir")
      File.mkdir_p!(subdir)

      prev_cwd = File.cwd!()

      try do
        File.cd!(subdir)
        {:ok, config} = Kreuzberg.ExtractionConfig.discover()

        assert config.chunking != nil
        assert config.chunking["max_chars"] == 50
      after
        File.cd!(prev_cwd)
      end
    end

    test "Load configuration from a TOML file" do
      tmpdir = Path.join(System.tmp_dir!(), "kreuzberg_e2e_#{System.unique_integer([:positive])}")
      File.mkdir_p!(tmpdir)
      on_exit(fn -> File.rm_rf!(tmpdir) end)
      config_path = Path.join(tmpdir, "test_config.toml")

      config_content = """
      [chunking]
      max_chars = 100
      max_overlap = 20

      [language_detection]
      enabled = false
      """

      File.write!(config_path, config_content)

      {:ok, config} = Kreuzberg.ExtractionConfig.from_file(config_path)

      assert config.chunking != nil
      assert config.chunking["max_chars"] == 100
      assert config.chunking["max_overlap"] == 20
      assert config.language_detection != nil
      assert config.language_detection["enabled"] == false
    end
  end
end

defmodule E2E.DocumentExtractorManagementTest do
  use ExUnit.Case, async: false

  describe "Document Extractor Management" do
    test "Clear all document extractors and verify list is empty" do
      Kreuzberg.Plugin.clear_document_extractors()
      {:ok, result} = Kreuzberg.Plugin.list_document_extractors()
      assert Enum.empty?(result)
    end

    test "List all registered document extractors" do
      {:ok, result} = Kreuzberg.Plugin.list_document_extractors()
      assert is_list(result)
      assert Enum.all?(result, &is_binary/1)
    end

    test "Unregister nonexistent document extractor gracefully" do
      Kreuzberg.Plugin.unregister_document_extractor(:"nonexistent-extractor-xyz")
      # Should not raise an error
    end
  end
end

defmodule E2E.MimeUtilitiesTest do
  use ExUnit.Case, async: false

  describe "Mime Utilities" do
    test "Detect MIME type from file bytes" do
      test_bytes = "%PDF-1.4\\n"
      {:ok, result} = Kreuzberg.detect_mime_type(test_bytes)
      assert String.contains?(String.downcase(result), "pdf")
    end

    test "Detect MIME type from file path" do
      tmpdir = Path.join(System.tmp_dir!(), "kreuzberg_e2e_#{System.unique_integer([:positive])}")
      File.mkdir_p!(tmpdir)
      on_exit(fn -> File.rm_rf!(tmpdir) end)
      test_file = Path.join(tmpdir, "test.txt")
      File.write!(test_file, "Hello, world!")

      {:ok, result} = Kreuzberg.detect_mime_type_from_path(test_file)
      assert String.contains?(String.downcase(result), "text")
    end

    test "Get file extensions for a MIME type" do
      {:ok, result} = Kreuzberg.get_extensions_for_mime("application/pdf")
      assert is_list(result)
      assert Enum.member?(result, "pdf")
    end
  end
end

defmodule E2E.OcrBackendManagementTest do
  use ExUnit.Case, async: false

  describe "Ocr Backend Management" do
    test "Clear all OCR backends and verify list is empty" do
      Kreuzberg.Plugin.clear_ocr_backends()
      {:ok, result} = Kreuzberg.Plugin.list_ocr_backends()
      assert Enum.empty?(result)
    end

    test "List all registered OCR backends" do
      {:ok, result} = Kreuzberg.Plugin.list_ocr_backends()
      assert is_list(result)
      assert Enum.all?(result, &is_binary/1)
    end

    test "Unregister nonexistent OCR backend gracefully" do
      Kreuzberg.Plugin.unregister_ocr_backend(:"nonexistent-backend-xyz")
      # Should not raise an error
    end
  end
end

defmodule E2E.PostProcessorManagementTest do
  use ExUnit.Case, async: false

  describe "Post Processor Management" do
    test "Clear all post-processors and verify list is empty" do
      Kreuzberg.Plugin.clear_post_processors()
    end

    test "List all registered post-processors" do
      {:ok, result} = Kreuzberg.Plugin.list_post_processors()
      assert is_list(result)
      assert Enum.all?(result, &is_binary/1)
    end
  end
end

defmodule E2E.ValidatorManagementTest do
  use ExUnit.Case, async: false

  describe "Validator Management" do
    test "Clear all validators and verify list is empty" do
      Kreuzberg.Plugin.clear_validators()
      {:ok, result} = Kreuzberg.Plugin.list_validators()
      assert Enum.empty?(result)
    end

    test "List all registered validators" do
      {:ok, result} = Kreuzberg.Plugin.list_validators()
      assert is_list(result)
      assert Enum.all?(result, &is_binary/1)
    end
  end
end
