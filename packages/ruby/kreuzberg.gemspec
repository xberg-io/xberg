# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name = 'kreuzberg'
  spec.version = '5.0.0.pre.rc.1'
  spec.authors       = ["Na'aman Hirschfeld <naaman@kreuzberg.dev>"]
  spec.summary       = 'High-performance document intelligence library'
  spec.description   = 'High-performance document intelligence library'
  spec.homepage      = 'https://github.com/kreuzberg-dev/kreuzberg'
  spec.license       = 'Elastic-2.0'
  spec.required_ruby_version = '>= 3.2.0'
  spec.metadata['keywords'] = %w[document extraction pdf ocr text].join(',')
  spec.metadata['rubygems_mfa_required'] = 'true'

  spec.files         = Dir.glob(%w[lib/**/* ext/**/* sig/**/* Steepfile])
  spec.require_paths = ['lib']
  spec.extensions    = ['ext/kreuzberg_rb/extconf.rb']

  spec.add_dependency 'rb_sys', '~> 0.9'
  spec.add_dependency 'sorbet-runtime', '~> 0.5'
end
