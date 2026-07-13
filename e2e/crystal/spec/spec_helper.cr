require "spec"

# Environment variables set before loading the binding
ENV["CRAWLBERG_ALLOW_PRIVATE_NETWORK"] ||= "true"

require "xberg"
