defmodule Kreuzberg.CacheAPI do
  @moduledoc """
  Cache management operations for the Kreuzberg extraction library.

  This module provides functions for managing the extraction result cache,
  including retrieving cache statistics and clearing cached data.

  The cache is automatically managed by Kreuzberg during normal extraction
  operations when `use_cache: true` is set in the extraction configuration.

  ## Cache Overview

  Kreuzberg's cache system stores extraction results on disk to avoid re-processing
  identical documents. This is particularly useful for:
  - Batch operations processing the same files multiple times
  - Development and testing workflows
  - Production systems with repeated document analysis needs

  ## Cache Key Format

  Cache keys are generated from:
  - Document binary content (hash-based to minimize storage)
  - MIME type of the document
  - Extraction configuration used
  - OCR settings and parameters

  This ensures that the same document extracted with different configurations
  produces separate cache entries.

  ## Cache Invalidation Triggers

  The cache is automatically invalidated when:
  - Document content changes (detected via hash)
  - Extraction configuration changes
  - OCR settings or backends are modified
  - Plugin configuration changes

  Manual cache clearing is available via `clear_cache/0` or `clear_cache!/0`.

  ## Persistence Details

  - **Location**: Cache data is stored on disk in the system's temporary directory
  - **Format**: Extraction results are serialized in a platform-neutral binary format
  - **Durability**: Cache persists across application restarts
  - **Cleanup**: Old cache entries may be automatically removed based on age/size policies

  ## Multi-Process Safety

  The cache system is designed to be safe for concurrent access:
  - Multiple processes can read from cache simultaneously
  - Cache writes are atomic to prevent corruption
  - No explicit locking needed from application code
  - GenServer-backed implementation ensures thread safety

  ## Performance Considerations

  - **Cache hits**: Retrieval of cached results is typically 100-1000x faster than re-extraction
  - **Cache misses**: Small overhead for cache lookup (typically < 1ms per operation)
  - **Disk I/O**: Large documents with images/embeddings may have significant I/O time
  - **Memory**: Cache operations do not load full results into memory unnecessarily

  ## Usage Examples

      # Enable caching during extraction
      config = %Kreuzberg.ExtractionConfig{use_cache: true}
      {:ok, result} = Kreuzberg.extract(pdf_binary, "application/pdf", config)

      # Check cache statistics - returns map with cache info
      {:ok, cache_info} = Kreuzberg.cache_stats()

      # Clear entire cache when needed
      :ok = Kreuzberg.clear_cache()

      # Monitor cache growth and clear if needed
      case Kreuzberg.cache_stats() do
        {:ok, %{"total_size_mb" => size}} when size > 1000 ->
          Kreuzberg.clear_cache()
        {:ok, _info} ->
          :ok
      end
  """

  alias Kreuzberg.{Error, Native, Helpers}

  @doc """
  Retrieve statistics about the extraction cache.

  Returns a map containing information about the current state of the cache,
  including the number of cached files, total cache size, available disk space,
  and file age information.

  ## Returns

    * `{:ok, stats}` - Map with cache statistics:
      * `total_files` - Number of cached extraction results
      * `total_size_mb` - Total size of cache in megabytes
      * `available_space_mb` - Available disk space in megabytes
      * `oldest_file_age_days` - Age of oldest cached file in days
      * `newest_file_age_days` - Age of newest cached file in days
    * `{:error, reason}` - Error message if retrieval fails

  ## Examples

      iex> {:ok, stats} = Kreuzberg.CacheAPI.cache_stats()
      iex> stats["total_files"]
      42
      iex> stats["total_size_mb"]
      128.5
  """
  @spec cache_stats() :: {:ok, map()} | {:error, String.t()}
  def cache_stats do
    case Native.cache_stats() do
      {:ok, stats_map} when is_map(stats_map) ->
        {:ok, Helpers.normalize_stats_keys(stats_map)}

      {:error, _reason} = err ->
        err
    end
  end

  @doc """
  Retrieve cache statistics, raising on error.

  Same as `cache_stats/0` but raises a `Kreuzberg.Error` exception if
  retrieval fails.

  ## Returns

    * Map with cache statistics keys and values

  ## Raises

    * `Kreuzberg.Error` - If cache statistics retrieval fails

  ## Examples

      iex> stats = Kreuzberg.CacheAPI.cache_stats!()
      iex> is_map(stats)
      true
  """
  @spec cache_stats!() :: map()
  def cache_stats! do
    case cache_stats() do
      {:ok, stats} -> stats
      {:error, reason} -> raise Error, message: reason, reason: Kreuzberg.UtilityAPI.classify_error(reason)
    end
  end

  @doc """
  Clear the extraction cache, removing all cached results.

  Removes all cached extraction results from disk. This is useful for
  reclaiming disk space or resetting the cache when stale data is suspected.

  ## Returns

    * `:ok` - Cache cleared successfully
    * `{:error, reason}` - Error message if clearing fails

  ## Examples

      iex> :ok = Kreuzberg.CacheAPI.clear_cache()
      iex> {:ok, stats} = Kreuzberg.CacheAPI.cache_stats()
      iex> stats["total_files"]
      0
  """
  @spec clear_cache() :: :ok | {:error, String.t()}
  def clear_cache do
    case Native.clear_cache() do
      :ok -> :ok
      {:error, _reason} = err -> err
    end
  end

  @doc """
  Clear the cache, raising on error.

  Same as `clear_cache/0` but raises a `Kreuzberg.Error` exception if
  the cache clearing operation fails.

  ## Raises

    * `Kreuzberg.Error` - If cache clearing fails

  ## Examples

      iex> Kreuzberg.CacheAPI.clear_cache!()
      :ok
  """
  @spec clear_cache!() :: :ok
  def clear_cache! do
    case clear_cache() do
      :ok -> :ok
      {:error, reason} -> raise Error, message: reason, reason: Kreuzberg.UtilityAPI.classify_error(reason)
    end
  end
end
