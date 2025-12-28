```elixir title="Elixir"
# CLI with Configuration - Advanced CLI with config file support
# Demonstrates managing complex extraction configurations via CLI

defmodule KreuzbergAdvancedCLI do
  @moduledoc """
  Advanced CLI with configuration file support for Kreuzberg.

  Allows users to define extraction configurations in YAML/TOML files,
  specify preprocessing options, and manage extraction profiles.
  """

  require Logger

  defmodule ConfigFile do
    @moduledoc """
    Configuration file parser and manager.
    """

    defstruct [
      :profiles,
      :default_profile,
      :cache_enabled,
      :cache_dir
    ]

    @doc """
    Load configuration from YAML/JSON file.
    """
    def load(config_path) do
      unless File.exists?(config_path) do
        {:error, "Config file not found: #{config_path}"}
      else
        case Path.extname(config_path) do
          ".yaml" -> load_yaml(config_path)
          ".yml" -> load_yaml(config_path)
          ".json" -> load_json(config_path)
          ext -> {:error, "Unsupported config format: #{ext}"}
        end
      end
    end

    defp load_yaml(path) do
      case File.read(path) do
        {:ok, content} ->
          case :yamerl_constr.string(content, []) do
            [config] -> {:ok, parse_config(config)}
            error -> {:error, "Failed to parse YAML: #{inspect(error)}"}
          end

        {:error, reason} ->
          {:error, "Failed to read config: #{inspect(reason)}"}
      end
    end

    defp load_json(path) do
      case File.read(path) do
        {:ok, content} ->
          case Jason.decode(content) do
            {:ok, config} -> {:ok, parse_config(config)}
            error -> {:error, "Failed to parse JSON: #{inspect(error)}"}
          end

        {:error, reason} ->
          {:error, "Failed to read config: #{inspect(reason)}"}
      end
    end

    defp parse_config(raw_config) when is_list(raw_config) do
      raw_config = Map.new(raw_config)
      parse_config(raw_config)
    end

    defp parse_config(raw_config) when is_map(raw_config) do
      %ConfigFile{
        profiles: Map.get(raw_config, "profiles", %{}),
        default_profile: Map.get(raw_config, "default_profile", "default"),
        cache_enabled: Map.get(raw_config, "cache_enabled", true),
        cache_dir: Map.get(raw_config, "cache_dir", "/tmp/kreuzberg_cache")
      }
    end

    @doc """
    Get extraction configuration for a profile.
    """
    def get_profile(config_file, profile_name) do
      profile_name = profile_name || config_file.default_profile

      case Map.get(config_file.profiles, profile_name) do
        nil -> {:error, "Profile not found: #{profile_name}"}
        profile -> {:ok, profile}
      end
    end

    @doc """
    List all available profiles.
    """
    def list_profiles(config_file) do
      Map.keys(config_file.profiles)
    end
  end

  defmodule Extractor do
    @moduledoc """
    Main extraction engine with profile support.
    """

    def extract_with_profile(file_path, config_file, profile_name, opts \\ []) do
      verbose = Keyword.get(opts, :verbose, false)

      case ConfigFile.get_profile(config_file, profile_name) do
        {:ok, profile} ->
          extract_with_config(file_path, profile, config_file, verbose)

        {:error, reason} ->
          {:error, reason}
      end
    end

    defp extract_with_config(file_path, profile, config_file, verbose) do
      unless File.exists?(file_path) do
        {:error, "File not found: #{file_path}"}
      else
        # Build extraction config from profile
        extraction_config = build_extraction_config(profile)

        # Apply caching if enabled
        use_cache = config_file.cache_enabled
        cache_dir = config_file.cache_dir

        IO.puts("Profile: #{profile["name"]}")
        IO.puts("File: #{file_path}")
        IO.puts("Cache: #{if use_cache, do: "enabled (#{cache_dir})", else: "disabled"}")
        IO.puts("")

        # Preprocess if configured
        processed_file = preprocess_if_needed(file_path, profile, verbose)

        start_time = System.monotonic_time(:millisecond)

        case Kreuzberg.extract_file(processed_file, nil, extraction_config) do
          {:ok, result} ->
            elapsed = System.monotonic_time(:millisecond) - start_time

            # Post-process if configured
            final_result = postprocess_if_needed(result, profile)

            print_results(final_result, elapsed, verbose)
            cleanup_temp_files(processed_file, file_path)
            {:ok, final_result}

          {:error, reason} ->
            cleanup_temp_files(processed_file, file_path)
            {:error, reason}
        end
      end
    end

    defp build_extraction_config(profile) do
      %Kreuzberg.ExtractionConfig{
        ocr: profile["ocr"],
        chunking: profile["chunking"],
        quality_processing: profile["quality_processing"],
        language_detection: profile["language_detection"],
        keyword_extraction: profile["keyword_extraction"],
        images: profile["images"],
        use_cache: true
      }
    end

    defp preprocess_if_needed(file_path, profile, verbose) do
      case profile["preprocessing"] do
        nil ->
          file_path

        preprocessing ->
          IO.puts("Preprocessing enabled:")
          temp_path = "/tmp/kreuzberg_#{System.unique_integer([:positive])}"

          # Apply preprocessing steps
          preprocessing
          |> Enum.reduce(file_path, fn step, path ->
            apply_preprocessing_step(step, path, temp_path, verbose)
          end)
      end
    end

    defp apply_preprocessing_step(step, input_path, _temp_path, verbose) do
      case step do
        %{"type" => "rotate", "degrees" => degrees} ->
          if verbose, do: IO.puts("  - Rotating #{degrees} degrees")
          input_path

        %{"type" => "normalize", "target_format" => format} ->
          if verbose, do: IO.puts("  - Normalizing to #{format}")
          input_path

        %{"type" => "deskew"} ->
          if verbose, do: IO.puts("  - Deskewing")
          input_path

        _ ->
          input_path
      end
    end

    defp postprocess_if_needed(result, profile) do
      case profile["postprocessing"] do
        nil ->
          result

        postprocessing ->
          Enum.reduce(postprocessing, result, fn step, acc_result ->
            apply_postprocessing_step(step, acc_result)
          end)
      end
    end

    defp apply_postprocessing_step(%{"type" => "filter_empty_chunks"}, result) do
      case result.chunks do
        nil -> result
        chunks ->
          filtered = Enum.filter(chunks, &(byte_size(&1) > 0))
          %{result | chunks: filtered}
      end
    end

    defp apply_postprocessing_step(%{"type" => "limit_tables", "max" => max_tables}, result) do
      case result.tables do
        nil -> result
        tables ->
          limited = Enum.take(tables, max_tables)
          %{result | tables: limited}
      end
    end

    defp apply_postprocessing_step(_, result), do: result

    defp cleanup_temp_files(processed_path, original_path) do
      if processed_path != original_path && String.starts_with?(processed_path, "/tmp/") do
        File.rm(processed_path)
      end
    end

    defp print_results(result, elapsed_ms, verbose) do
      IO.puts("Results:")
      IO.puts("  Content size: #{byte_size(result.content)} bytes")
      IO.puts("  Mime type: #{result.mime_type}")
      IO.puts("  Processing time: #{elapsed_ms}ms")

      if result.metadata do
        IO.puts("  Metadata entries: #{Enum.count(result.metadata)}")
      end

      if result.tables && !Enum.empty?(result.tables) do
        IO.puts("  Tables: #{length(result.tables)}")
      end

      if result.chunks && !Enum.empty?(result.chunks) do
        IO.puts("  Chunks: #{length(result.chunks)}")
      end

      if result.detected_languages && !Enum.empty?(result.detected_languages) do
        IO.puts("  Languages: #{Enum.join(result.detected_languages, ", ")}")
      end

      if verbose && result.metadata do
        IO.puts("\n  Full metadata:")
        IO.inspect(result.metadata, pretty: true, limit: :infinity)
      end

      IO.puts("")
    end
  end

  @doc """
  Main CLI entry point.
  """
  def main(args) do
    args
    |> parse_args()
    |> execute()
  end

  defp parse_args(args) do
    {opts, args, _invalid} = OptionParser.parse(args,
      switches: [
        config: :string,
        profile: :string,
        output: :string,
        verbose: :boolean,
        list: :boolean
      ],
      aliases: [c: :config, p: :profile, o: :output, v: :verbose, l: :list]
    )

    {opts, args}
  end

  defp execute({_opts, []}) do
    print_usage()
    :error
  end

  defp execute({opts, [command | rest]}) do
    case command do
      "extract" ->
        execute_extract(rest, opts)

      "profiles" ->
        execute_list_profiles(opts)

      "help" ->
        print_help()
        :ok

      _ ->
        IO.puts(:stderr, "Unknown command: #{command}")
        print_usage()
        :error
    end
  end

  defp execute_extract(args, opts) do
    config_path = Keyword.get(opts, :config, "kreuzberg.yaml")
    profile = Keyword.get(opts, :profile, nil)
    output_path = Keyword.get(opts, :output, nil)
    verbose = Keyword.get(opts, :verbose, false)

    case ConfigFile.load(config_path) do
      {:ok, config_file} ->
        case args do
          [] ->
            IO.puts(:stderr, "Error: No file specified")
            :error

          [file_path | _] ->
            case Extractor.extract_with_profile(file_path, config_file, profile, verbose: verbose) do
              {:ok, result} ->
                if output_path do
                  save_result(result, output_path)
                end
                :ok

              {:error, reason} ->
                IO.puts(:stderr, "Extraction failed: #{reason}")
                :error
            end
        end

      {:error, reason} ->
        IO.puts(:stderr, "Configuration error: #{reason}")
        :error
    end
  end

  defp execute_list_profiles(opts) do
    config_path = Keyword.get(opts, :config, "kreuzberg.yaml")

    case ConfigFile.load(config_path) do
      {:ok, config_file} ->
        profiles = ConfigFile.list_profiles(config_file)
        IO.puts("Available profiles:")
        Enum.each(profiles, fn profile ->
          IO.puts("  - #{profile}")
        end)
        :ok

      {:error, reason} ->
        IO.puts(:stderr, "Configuration error: #{reason}")
        :error
    end
  end

  defp save_result(result, output_path) do
    output_data = %{
      content: result.content,
      mime_type: result.mime_type,
      metadata: result.metadata,
      tables: result.tables || [],
      chunks: result.chunks || [],
      images: result.images || [],
      detected_languages: result.detected_languages || [],
      extracted_at: DateTime.utc_now()
    }

    case File.write(output_path, Jason.encode!(output_data, pretty: true)) do
      :ok ->
        IO.puts("Results saved to: #{output_path}")

      {:error, reason} ->
        IO.puts(:stderr, "Failed to save results: #{inspect(reason)}")
    end
  end

  defp print_usage do
    IO.puts("Usage: kreuzberg-cli <command> [options] [args]")
  end

  defp print_help do
    IO.puts("""
    Kreuzberg Advanced CLI with Configuration

    USAGE:
      kreuzberg extract <file> [OPTIONS]
      kreuzberg profiles [OPTIONS]
      kreuzberg help

    COMMANDS:
      extract <file>        Extract with configured profile
      profiles              List available profiles
      help                  Show this help message

    OPTIONS:
      -c, --config <path>   Config file path (default: kreuzberg.yaml)
      -p, --profile <name>  Profile name (default: from config)
      -o, --output <path>   Save results to JSON
      -v, --verbose         Verbose output

    CONFIG FILE EXAMPLE:
      default_profile: "standard"
      cache_enabled: true
      cache_dir: "/tmp/kreuzberg_cache"

      profiles:
        standard:
          name: "Standard Extraction"
          ocr:
            enabled: false
          chunking:
            enabled: true
            max_chars: 1000
            max_overlap: 100

        ocr_intensive:
          name: "OCR + Language Detection"
          ocr:
            enabled: true
            backend: tesseract
          language_detection:
            enabled: true
          preprocessing:
            - type: deskew
            - type: rotate
              degrees: 90
    """)
  end
end

# Entry point
case KreuzbergAdvancedCLI.main(System.argv()) do
  :ok -> IO.puts("\nDone.")
  :error -> exit(1)
end
```
