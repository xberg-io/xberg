defmodule Kreuzberg.Native do
  @moduledoc false

  version = Mix.Project.config()[:version]

  use RustlerPrecompiled,
    otp_app: :kreuzberg,
    crate: "kreuzberg_rustler",
    base_url: "https://github.com/kreuzberg-dev/kreuzberg/releases/download/v#{version}",
    version: version,
    force_build: System.get_env("KREUZBERG_BUILD") in ["1", "true"],
    targets: ~w(
      aarch64-apple-darwin
      x86_64-apple-darwin
      x86_64-unknown-linux-gnu
      x86_64-unknown-linux-musl
      aarch64-unknown-linux-gnu
      aarch64-unknown-linux-musl
      x86_64-pc-windows-msvc
      x86_64-pc-windows-gnu
    ),
    nif_versions: ["2.16", "2.17"]

  # Basic extraction
  def extract(_input, _input_type), do: :erlang.nif_error(:nif_not_loaded)

  # Extraction with options
  def extract_with_options(_input, _input_type, _options), do: :erlang.nif_error(:nif_not_loaded)

  # File extraction
  def extract_file(_file_path, _input_type), do: :erlang.nif_error(:nif_not_loaded)
  def extract_file_with_options(_file_path, _input_type, _options), do: :erlang.nif_error(:nif_not_loaded)

  # Batch extraction
  def batch_extract_files(_file_paths, _input_type), do: :erlang.nif_error(:nif_not_loaded)
  def batch_extract_files_with_options(_file_paths, _input_type, _options), do: :erlang.nif_error(:nif_not_loaded)

  # Batch extraction from bytes
  def batch_extract_bytes(_bytes_list, _input_type), do: :erlang.nif_error(:nif_not_loaded)
  def batch_extract_bytes_with_options(_bytes_list, _input_type, _options), do: :erlang.nif_error(:nif_not_loaded)

  # Cache operations
  def cache_stats(), do: :erlang.nif_error(:nif_not_loaded)
  def clear_cache(), do: :erlang.nif_error(:nif_not_loaded)

  # MIME type operations
  def detect_mime_type(_bytes), do: :erlang.nif_error(:nif_not_loaded)
  def detect_mime_type_from_path(_file_path), do: :erlang.nif_error(:nif_not_loaded)
  def validate_mime_type(_mime_type), do: :erlang.nif_error(:nif_not_loaded)
  def get_extensions_for_mime(_mime_type), do: :erlang.nif_error(:nif_not_loaded)

  # Embedding operations
  def list_embedding_presets(), do: :erlang.nif_error(:nif_not_loaded)
  def get_embedding_preset(_preset_name), do: :erlang.nif_error(:nif_not_loaded)

  # Validation functions
  def validate_chunking_params(_chunk_size, _overlap), do: :erlang.nif_error(:nif_not_loaded)
  def validate_language_code(_language_code), do: :erlang.nif_error(:nif_not_loaded)
  def validate_dpi(_dpi), do: :erlang.nif_error(:nif_not_loaded)
  def validate_confidence(_confidence), do: :erlang.nif_error(:nif_not_loaded)
  def validate_ocr_backend(_backend), do: :erlang.nif_error(:nif_not_loaded)
  def validate_binarization_method(_method), do: :erlang.nif_error(:nif_not_loaded)
  def validate_tesseract_psm(_psm), do: :erlang.nif_error(:nif_not_loaded)
  def validate_tesseract_oem(_oem), do: :erlang.nif_error(:nif_not_loaded)
end
