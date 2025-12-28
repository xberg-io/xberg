```elixir title="Elixir"
# Disk Caching - Implement persistent disk caching for extraction results
# Demonstrates advanced caching strategies for document extraction

defmodule KreuzbergDiskCache do
  @moduledoc """
  Disk-based caching layer for Kreuzberg extraction results.

  Provides persistent caching of extraction results with features like:
  - TTL-based cache expiration
  - Compression for large results
  - Cache statistics and management
  - Multi-tiered caching (memory + disk)
  """

  require Logger

  defmodule CacheEntry do
    @moduledoc """
    Represents a cached extraction result.
    """

    defstruct [
      :key,
      :result,
      :created_at,
      :accessed_at,
      :ttl_seconds,
      :size_bytes,
      :compressed
    ]

    @doc """
    Create a new cache entry.
    """
    def new(key, result, ttl_seconds \\ 86400) do
      size = calculate_size(result)

      %CacheEntry{
        key: key,
        result: result,
        created_at: System.monotonic_time(:second),
        accessed_at: System.monotonic_time(:second),
        ttl_seconds: ttl_seconds,
        size_bytes: size,
        compressed: false
      }
    end

    @doc """
    Check if entry has expired.
    """
    def expired?(%CacheEntry{created_at: created_at, ttl_seconds: ttl}) do
      now = System.monotonic_time(:second)
      now - created_at > ttl
    end

    @doc """
    Update access time.
    """
    def touch(%CacheEntry{} = entry) do
      %{entry | accessed_at: System.monotonic_time(:second)}
    end

    defp calculate_size(result) do
      case result do
        %{content: content} -> byte_size(content)
        _ -> 0
      end
    end
  end

  defmodule Cache do
    @moduledoc """
    Main disk cache implementation.
    """

    defstruct [
      :cache_dir,
      :max_size_bytes,
      :ttl_seconds,
      :compression_enabled,
      :memory_cache
    ]

    @doc """
    Initialize disk cache.
    """
    def new(cache_dir, opts \\ []) do
      File.mkdir_p!(cache_dir)

      %Cache{
        cache_dir: cache_dir,
        max_size_bytes: Keyword.get(opts, :max_size_bytes, 1_000_000_000),
        ttl_seconds: Keyword.get(opts, :ttl_seconds, 604_800),
        compression_enabled: Keyword.get(opts, :compression_enabled, true),
        memory_cache: %{}
      }
    end

    @doc """
    Get cached result by key.
    """
    def get(cache, key) do
      # Check memory cache first
      case Map.get(cache.memory_cache, key) do
        %CacheEntry{} = entry ->
          if CacheEntry.expired?(entry) do
            Logger.debug("Cache hit (memory) - expired: #{key}")
            :miss
          else
            Logger.debug("Cache hit (memory): #{key}")
            {:hit, CacheEntry.touch(entry).result}
          end

        nil ->
          get_from_disk(cache, key)
      end
    end

    @doc """
    Store result in cache.
    """
    def put(cache, key, result) do
      entry = CacheEntry.new(key, result, cache.ttl_seconds)

      # Store in memory
      new_memory_cache = Map.put(cache.memory_cache, key, entry)

      # Store on disk
      store_on_disk(cache, key, entry)

      # Check cache size and cleanup if needed
      cache = %{cache | memory_cache: new_memory_cache}
      maybe_cleanup(cache)

      Logger.info("Cache stored: #{key}")
      cache
    end

    @doc """
    Delete cache entry.
    """
    def delete(cache, key) do
      new_memory_cache = Map.delete(cache.memory_cache, key)

      cache_file = cache_path(cache, key)
      if File.exists?(cache_file), do: File.rm(cache_file)

      Logger.info("Cache deleted: #{key}")
      %{cache | memory_cache: new_memory_cache}
    end

    @doc """
    Clear all cache entries.
    """
    def clear(cache) do
      # Clear disk cache
      File.rm_rf!(cache.cache_dir)
      File.mkdir_p!(cache.cache_dir)

      Logger.info("Cache cleared")
      %{cache | memory_cache: %{}}
    end

    @doc """
    Get cache statistics.
    """
    def stats(cache) do
      total_size = calculate_total_size(cache)
      entry_count = map_size(cache.memory_cache)
      memory_entries = Enum.count(cache.memory_cache)

      disk_entries =
        case File.ls(cache.cache_dir) do
          {:ok, files} -> length(files)
          {:error, _} -> 0
        end

      %{
        total_entries: entry_count,
        memory_entries: memory_entries,
        disk_entries: disk_entries,
        total_size_bytes: total_size,
        max_size_bytes: cache.max_size_bytes,
        usage_percent: (total_size / cache.max_size_bytes * 100) |> Float.round(2),
        compression_enabled: cache.compression_enabled
      }
    end

    # Private helpers

    defp get_from_disk(cache, key) do
      cache_file = cache_path(cache, key)

      if File.exists?(cache_file) do
        case File.read(cache_file) do
          {:ok, data} ->
            case deserialize(data, cache.compression_enabled) do
              {:ok, entry} ->
                if CacheEntry.expired?(entry) do
                  File.rm(cache_file)
                  Logger.debug("Cache hit (disk) - expired: #{key}")
                  :miss
                else
                  Logger.debug("Cache hit (disk): #{key}")
                  {:hit, CacheEntry.touch(entry).result}
                end

              {:error, reason} ->
                Logger.warn("Failed to deserialize cache: #{inspect(reason)}")
                :miss
            end

          {:error, reason} ->
            Logger.warn("Failed to read cache file: #{inspect(reason)}")
            :miss
        end
      else
        :miss
      end
    end

    defp store_on_disk(cache, key, entry) do
      cache_file = cache_path(cache, key)

      data = serialize(entry, cache.compression_enabled)
      File.write!(cache_file, data)
    end

    defp cache_path(cache, key) do
      Path.join(cache.cache_dir, "#{key}.cache")
    end

    defp serialize(entry, compression_enabled) do
      data = :erlang.term_to_binary(entry)

      if compression_enabled do
        :zlib.compress(data)
      else
        data
      end
    end

    defp deserialize(data, compression_enabled) do
      try do
        uncompressed =
          if compression_enabled do
            :zlib.uncompress(data)
          else
            data
          end

        {:ok, :erlang.binary_to_term(uncompressed)}
      rescue
        e -> {:error, e}
      end
    end

    defp calculate_total_size(cache) do
      cache.memory_cache
      |> Map.values()
      |> Enum.map(& &1.size_bytes)
      |> Enum.sum()
    end

    defp maybe_cleanup(cache) do
      total_size = calculate_total_size(cache)

      if total_size > cache.max_size_bytes do
        Logger.info("Cache size (#{total_size}) exceeds limit, starting cleanup")
        cleanup_lru(cache)
      else
        cache
      end
    end

    defp cleanup_lru(cache) do
      # Remove least recently used entries until under limit
      entries =
        cache.memory_cache
        |> Enum.sort_by(fn {_k, entry} -> entry.accessed_at end)

      target_size = div(cache.max_size_bytes, 2)
      current_size = calculate_total_size(cache)

      entries
      |> Enum.reduce_while({cache, current_size}, fn {key, entry}, {acc_cache, size} ->
        if size <= target_size do
          {:halt, {acc_cache, size}}
        else
          new_cache = delete(acc_cache, key)
          new_size = size - entry.size_bytes
          {:cont, {new_cache, new_size}}
        end
      end)
      |> elem(0)
    end
  end

  @doc """
  Initialize cache and extract with caching.
  """
  def extract_with_cache(file_path, cache_dir, opts \\ []) do
    cache = Cache.new(cache_dir, opts)
    cache_key = compute_cache_key(file_path, opts)

    case Cache.get(cache, cache_key) do
      {:hit, result} ->
        {:ok, result, cache}

      :miss ->
        Logger.info("Cache miss: #{file_path}")

        case Kreuzberg.extract_file(file_path) do
          {:ok, result} ->
            new_cache = Cache.put(cache, cache_key, result)
            {:ok, result, new_cache}

          error ->
            {error, cache}
        end
    end
  end

  @doc """
  Extract multiple files with batch caching.
  """
  def batch_extract_with_cache(file_paths, cache_dir, opts \\ []) do
    cache = Cache.new(cache_dir, opts)

    results =
      file_paths
      |> Enum.map(fn path ->
        case extract_with_cache(path, cache_dir, opts) do
          {:ok, result, _} -> {:ok, path, result}
          {{:error, reason}, _} -> {:error, path, reason}
        end
      end)

    stats = Cache.stats(cache)
    {results, stats}
  end

  @doc """
  Manage cache - get stats, clear, etc.
  """
  def manage_cache(cache_dir, action, opts \\ []) do
    cache = Cache.new(cache_dir, opts)

    case action do
      :stats ->
        Cache.stats(cache)

      :clear ->
        Cache.clear(cache)

      :list ->
        case File.ls(cache_dir) do
          {:ok, files} -> files
          {:error, reason} -> {:error, reason}
        end

      {:delete, key} ->
        Cache.delete(cache, key)

      _ ->
        {:error, "Unknown action: #{action}"}
    end
  end

  # Private helpers

  defp compute_cache_key(file_path, opts) do
    # Include file path and options in key
    content = "#{file_path}|#{inspect(opts)}"
    :crypto.hash(:sha256, content) |> Base.encode16(case: :lower)
  end
end

# Usage examples
IO.puts("=== Kreuzberg Disk Cache ===\n")

cache_dir = "/tmp/kreuzberg_cache"

# Example 1: Single file extraction with caching
IO.puts("Example 1: Single file extraction with caching")
IO.puts("-" <> String.duplicate("-", 40) <> "\n")

case KreuzbergDiskCache.extract_with_cache("document.pdf", cache_dir) do
  {:ok, result, cache} ->
    IO.puts("Extraction successful!")
    IO.puts("Content size: #{byte_size(result.content)} bytes")

    stats = KreuzbergDiskCache.manage_cache(cache_dir, :stats)
    IO.puts("\nCache Statistics:")
    IO.puts("  Entries: #{stats.total_entries}")
    IO.puts("  Size: #{stats.total_size_bytes} bytes")
    IO.puts("  Usage: #{stats.usage_percent}%\n")

  {error, _cache} ->
    IO.puts("Extraction failed: #{inspect(error)}\n")
end

# Example 2: Batch extraction with cache statistics
IO.puts("Example 2: Batch extraction with caching")
IO.puts("-" <> String.duplicate("-", 40) <> "\n")

documents = ["doc1.pdf", "doc2.pdf", "doc3.pdf"]

{results, stats} = KreuzbergDiskCache.batch_extract_with_cache(documents, cache_dir)

successful = Enum.count(results, &match?({:ok, _, _}, &1))
IO.puts("Batch results:")
IO.puts("  Processed: #{length(documents)}")
IO.puts("  Successful: #{successful}")
IO.puts("\nCache Statistics:")
IO.puts("  Total entries: #{stats.total_entries}")
IO.puts("  Memory entries: #{stats.memory_entries}")
IO.puts("  Disk entries: #{stats.disk_entries}")
IO.puts("  Total size: #{stats.total_size_bytes} bytes")
IO.puts("  Usage: #{stats.usage_percent}%\n")

# Example 3: Cache management
IO.puts("Example 3: Cache management")
IO.puts("-" <> String.duplicate("-", 40) <> "\n")

cached_files = KreuzbergDiskCache.manage_cache(cache_dir, :list)
IO.puts("Cached files:")
Enum.each(cached_files, fn file -> IO.puts("  - #{file}") end)

IO.puts("\nCache stats:")
stats = KreuzbergDiskCache.manage_cache(cache_dir, :stats)
IO.inspect(stats, pretty: true)
```
