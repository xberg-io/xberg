# frozen_string_literal: true

require "fileutils"
require "rbconfig"
require "rubygems/package"
require "tmpdir"

platform, version, keep_source = ARGV
abort("Expected platform, version and source policy") unless platform && version && %w[true false].include?(keep_source)
abort("Ruby host platform does not match #{platform}") unless Gem::Platform.new(platform) === Gem::Platform.local

source = File.expand_path("pkg/xberg-#{version}.gem")
native = File.expand_path("pkg/xberg-#{version}-#{platform}.gem")
package = Gem::Package.new(source)
package.verify
spec = package.spec
unless spec.name == "xberg" && spec.version.to_s == version && spec.platform.to_s == "ruby"
  abort("Unexpected source gem metadata")
end

extension = "lib/xberg_rb.#{RbConfig::CONFIG.fetch("DLEXT")}"
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

Dir.mktmpdir("xberg-native-gem") do |directory|
  package.extract_files(directory)
  FileUtils.cp(extension, File.join(directory, extension))
  binary = File.binread(File.join(directory, extension), 4)
  abort("Missing or invalid native extension: #{extension}") unless magic.any? { |prefix| binary.start_with?(prefix) }

  spec.platform = Gem::Platform.new(platform)
  spec.required_ruby_version = Gem::Requirement.new(
    spec.required_ruby_version.as_list + ["~> #{RUBY_VERSION.split(".").take(2).join(".")}.0"]
  )
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
