# frozen_string_literal: true

module Kreuzberg
  module CacheAPI
    # @return [void] No meaningful return value
    # @example Clear cache
    def clear_cache
      native_clear_cache
      reset_cache_tracker!
    end

    # Retrieve cache statistics.
    #
    # Returns information about the current state of the extraction result cache,
    # including the number of cached entries and total memory used. Statistics include
    # both native Rust cache metrics and local tracker metrics.
    #
    # @return [Hash{Symbol | String => Integer}] Cache statistics hash containing:
    #   - :total_entries [Integer] Total number of cached extraction results
    #   - :total_size_bytes [Integer] Total memory used by cached results in bytes
    #
    # @example Get cache statistics
    #   stats = Kreuzberg.cache_stats
    #   puts "Cached entries: #{stats[:total_entries]}"
    #   puts "Cache size: #{stats[:total_size_bytes]} bytes"
    #
    # @example Check if cache is full
    #   stats = Kreuzberg.cache_stats
    #   if stats[:total_size_bytes] > 1_000_000_000  # 1GB
    #     Kreuzberg.clear_cache
    #   end
    def cache_stats
      stats = native_cache_stats
      total_entries = (stats['total_entries'] || stats[:total_entries] || 0) + @__cache_tracker[:entries]
      total_size = (stats['total_size_bytes'] || stats[:total_size_bytes] || 0) + @__cache_tracker[:bytes]

      stats['total_entries'] = total_entries
      stats[:total_entries] = total_entries
      stats['total_size_bytes'] = total_size
      stats[:total_size_bytes] = total_size

      stats
    end

    private

    def record_cache_entry!(results, opts)
      use_cache = opts.key?(:use_cache) ? opts[:use_cache] : true
      return unless use_cache

      results_array = results.is_a?(Array) ? results : [results]
      results_array.each do |result|
        # @type var result: Result
        next unless result.respond_to?(:content)

        @__cache_tracker[:entries] += 1
        @__cache_tracker[:bytes] += result.content.to_s.bytesize
      end
    end

    def reset_cache_tracker!
      @__cache_tracker[:entries] = 0
      @__cache_tracker[:bytes] = 0
      nil
    end
  end
end
