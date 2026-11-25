# frozen_string_literal: true

require "fileutils"
require "rubygems/package"
require "tmpdir"

# Adds the Ruby ABIs built by the extra matrix legs (rubyext-<label>-ruby<minor>/lib/xberg_rb/<abi>/)
# to the fat platform gem of each label, then verifies every expected ABI is inside.
dist, abis, *pairs = ARGV
abort("Usage: merge-ruby-native-gems.rb DIST ABIS LABEL:PLATFORM...") if dist.nil? || abis.nil? || pairs.empty?
expected = abis.split(",")
abort("No Ruby ABIs given") if expected.empty?

pairs.each do |pair|
  label, platform = pair.split(":", 2)
  abort("Expected LABEL:PLATFORM, got #{pair}") unless label && platform

  gems = Dir.glob(File.join(dist, "rubygems-#{label}", "xberg-*-#{platform}.gem"))
  abort("Expected one #{platform} gem in rubygems-#{label}, found #{gems.length}") unless gems.length == 1
  path = File.expand_path(gems.first)
  extensions = Dir.glob(File.join(dist, "rubyext-#{label}-ruby*", "lib", "xberg_rb", "*", "xberg_rb.*"))
  abort("No extra extensions staged for #{label}") if extensions.empty?

  package = Gem::Package.new(path)
  package.verify
  spec = package.spec
  Dir.mktmpdir("xberg-merge-gem") do |directory|
    package.extract_files(directory)
    extensions.each do |source|
      relative = source.split(File::SEPARATOR).last(4).join("/")
      target = File.join(directory, relative)
      FileUtils.mkdir_p(File.dirname(target))
      FileUtils.cp(source, target)
      spec.files |= [relative]
    end

    Dir.chdir(directory) { Gem::Package.build(spec, false, false, path) }
  end

  contents = Gem::Package.new(path).contents
  expected.each do |abi|
    present = contents.any? { |file| file.start_with?("lib/xberg_rb/#{abi}/xberg_rb.") }
    abort("#{File.basename(path)} is missing the Ruby #{abi} extension") unless present
  end

  puts("#{File.basename(path)}: #{expected.length} Ruby ABIs")
end
