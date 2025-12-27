# frozen_string_literal: true

module Kreuzberg
  # @example Implementing a minimum length validator
  # @example Implementing a content quality validator
  # @example Using a Proc as a validator
  module ValidatorProtocol
    # @param result [Hash] Extraction result to validate with the following structure:
    # @return [void]
    # @raise [Kreuzberg::Errors::ValidationError] if validation fails
    # @example
    def call(result)
      raise NotImplementedError, "#{self.class} must implement #call(result)"
    end
  end
end
