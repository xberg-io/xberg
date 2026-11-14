# frozen_string_literal: true

require "fileutils"
require "open3"
require "rbconfig"
require "rubygems/package"
require "tmpdir"

script = File.expand_path("package-ruby-native-gem.rb", __dir__)
local = Gem::Platform.local
platform = local.os == "darwin" ? "#{local.cpu}-darwin" : local.to_s
extension = "lib/xberg_rb.#{RbConfig::CONFIG.fetch('DLEXT')}"
magic = { "linux" => "\x7fELF".b, "darwin" => "\xcf\xfa\xed\xfe".b, "mingw" => "MZ".b }.fetch(local.os)
version = "1.4.0"
source = "pkg/xberg-#{version}.gem"
native = "pkg/xberg-#{version}-#{platform}.gem"

Dir.mktmpdir("xberg-gem-test") do |directory|
  Dir.chdir(directory) do
    FileUtils.mkdir_p(%w[pkg lib ext])
    File.write("lib/xberg.rb", "# fixture")
    File.write("ext/extconf.rb", "# fixture")
    spec = Gem::Specification.new do |gem|
      gem.name = "xberg"
      gem.version = version
      gem.summary = "Fixture"
      gem.authors = ["Fixture"]
      gem.files = %w[lib/xberg.rb ext/extconf.rb]
      gem.extensions = ["ext/extconf.rb"]
    end
    Gem::Package.build(spec, false, false, source)
    original = File.binread(source)

    run = ->(keep) { Open3.capture3(RbConfig.ruby, script, platform, version, keep) }
    _, stderr, status = run.call("true")
    raise "missing binary passed" if status.success? || !stderr.include?("xberg_rb") || File.exist?(native)

    File.binwrite(extension, "invalid")
    _, stderr, status = run.call("true")
    raise "invalid binary passed" if status.success? || !stderr.include?("invalid native extension")

    File.binwrite(extension, magic + "fixture")
    %w[true false].each do |keep|
      _, stderr, status = run.call(keep)
      raise stderr unless status.success?
      gem = Gem::Package.new(native)
      raise "wrong native package" unless gem.spec.full_name == "xberg-#{version}-#{platform}" &&
                                          gem.spec.extensions.empty? && gem.contents.include?(extension)
      raise "source policy mismatch" unless File.exist?(source) == (keep == "true")
      raise "source changed" if keep == "true" && File.binread(source) != original
    end
  end
end
puts "Native gem packaging controls passed"
