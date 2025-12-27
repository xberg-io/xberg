# frozen_string_literal: true

module Kreuzberg
  # @example Implementing a simple post-processor
  # @example Implementing a post-processor that adds metadata
  # @example Using a Proc as a post-processor
  module PostProcessorProtocol
    # @param result [Hash] Extraction result with the following structure:
    # @return [Hash] Modified extraction result with enriched metadata
    # @example
    def call(result)
      raise NotImplementedError, "#{self.class} must implement #call(result)"
    end
  end
end
