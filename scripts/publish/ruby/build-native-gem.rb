#!/usr/bin/env ruby
# frozen_string_literal: true

# Build a platform-specific pre-compiled gem.
#
# Usage: ruby build-native-gem.rb <platform>
#
# Platforms:
#   x86_64-linux
#   aarch64-linux
#   arm64-darwin

require 'rubygems'
require 'rubygems/package'
require 'fileutils'

platform = ARGV[0] or abort "Usage: #{$PROGRAM_NAME} <platform>"

VALID_PLATFORMS = %w[x86_64-linux aarch64-linux arm64-darwin].freeze
unless VALID_PLATFORMS.include?(platform)
  abort "ERROR: Invalid platform '#{platform}'. Valid: #{VALID_PLATFORMS.join(', ')}"
end

# Work from the Ruby package directory
gem_dir = File.expand_path('../../../packages/ruby', __dir__)
Dir.chdir(gem_dir)

# Validate compiled native library exists
native_extensions = Dir.glob('lib/**/*.{so,bundle,dylib}')
if native_extensions.empty?
  abort "ERROR: No compiled native extensions found in lib/. Run 'rake compile' first."
end

puts "Found native extensions: #{native_extensions.join(', ')}"

# Load the gemspec
spec = Gem::Specification.load('kreuzberg.gemspec')
abort 'ERROR: Could not load kreuzberg.gemspec' unless spec

# Set platform (transforms source gem into platform gem)
spec.platform = Gem::Platform.new(platform)

# Remove extensions field — pre-compiled gems skip install-time compilation
spec.extensions = []

# Ensure native artifacts are in the file list
native_extensions.each do |ext|
  spec.files << ext unless spec.files.include?(ext)
end

# Remove vendor/ and ext/ source files — not needed in platform gems
spec.files.reject! { |f| f.start_with?('vendor/') || f.start_with?('ext/') }

# Remove rb_sys runtime dependency — only needed for source compilation
spec.dependencies.reject! { |d| d.name == 'rb_sys' }

spec.files.uniq!

puts "Building gem: #{spec.name}-#{spec.version}-#{spec.platform}"
puts "Files: #{spec.files.length} (native: #{native_extensions.length})"

# Build the gem
FileUtils.mkdir_p('pkg')
gem_file = Gem::Package.build(spec)
FileUtils.mv(gem_file, "pkg/#{gem_file}") if File.exist?(gem_file) && !File.exist?("pkg/#{gem_file}")

puts "Built: pkg/#{gem_file}"
