# frozen_string_literal: true

require "fileutils"
require "rbconfig"
require "rubygems/package"
require "tmpdir"

# Layouts:
#   pinned    one extension at lib/ and a `~> X.Y.0` Ruby pin (the Windows gem).
#   fat       one extension under lib/xberg_rb/<ruby ABI>/ with no Ruby pin; the publish job adds the
#             other Ruby ABIs with merge-ruby-native-gems.rb.
#   extension only stage lib/xberg_rb/<ruby ABI>/ for that merge; no gem is built.
platform, version, keep_source, layout = ARGV
layout ||= "pinned"
abort("Expected platform, version and source policy") unless platform && version && %w[true false].include?(keep_source)
abort("Unknown layout #{layout}") unless %w[pinned fat extension].include?(layout)
abort("Ruby host platform does not match #{platform}") unless Gem::Platform.new(platform) === Gem::Platform.local

built_extension = "lib/xberg_rb.#{RbConfig::CONFIG.fetch("DLEXT")}"
extension = if layout == "pinned"
  built_extension
else
  "lib/xberg_rb/#{RbConfig::CONFIG.fetch("ruby_version")}/xberg_rb.#{RbConfig::CONFIG.fetch("DLEXT")}"
end

magic = case Gem::Platform.local.os
when "linux"
  ["\x7fELF".b]
when "mingw"
  ["MZ".b]
when "darwin"
  ["\xcf\xfa\xed\xfe".b]
else
  abort("Unsupported Ruby native platform: #{platform}")
end

if layout == "extension"
  abort("Missing native extension: #{built_extension}") unless File.file?(built_extension)
  binary = File.binread(built_extension, 4)
  abort("Missing or invalid native extension: #{extension}") unless magic.any? { |prefix| binary.start_with?(prefix) }

  staged = File.expand_path("pkg/ext/#{extension}")
  FileUtils.mkdir_p(File.dirname(staged))
  FileUtils.cp(built_extension, staged)
  exit
end

source = File.expand_path("pkg/xberg-#{version}.gem")
native = File.expand_path("pkg/xberg-#{version}-#{platform}.gem")
package = Gem::Package.new(source)
package.verify
spec = package.spec
unless spec.name == "xberg" && spec.version.to_s == version && spec.platform.to_s == "ruby"
  abort("Unexpected source gem metadata")
end

Dir.mktmpdir("xberg-native-gem") do |directory|
  package.extract_files(directory)
  FileUtils.mkdir_p(File.dirname(File.join(directory, extension)))
  FileUtils.cp(built_extension, File.join(directory, extension))
  binary = File.binread(File.join(directory, extension), 4)
  abort("Missing or invalid native extension: #{extension}") unless magic.any? { |prefix| binary.start_with?(prefix) }

  spec.platform = Gem::Platform.new(platform)
  if layout == "pinned"
    spec.required_ruby_version = Gem::Requirement.new(
      spec.required_ruby_version.as_list + ["~> #{RUBY_VERSION.split(".").take(2).join(".")}.0"]
    )
  end

  spec.extensions = []
  spec.files |= [extension]
  Dir.chdir(directory) do
    Gem::Package.build(spec, false, false, native)
  end

  built = Gem::Package.new(native)
  built.verify
  unless built.spec.full_name == "xberg-#{version}-#{platform}" && built.spec.extensions.empty?
    abort("Unexpected native gem metadata")
  end

  abort("Native gem missing #{extension}") unless built.contents.include?(extension)
end

File.delete(source) if keep_source == "false"
