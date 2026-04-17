# frozen_string_literal: true

require_relative 'lib/kreuzberg/version'

repo_root = File.expand_path('../..', __dir__)

# Collect ruby package files via Dir.glob (no git dependency)
ruby_files = Dir.chdir(__dir__) do
  Dir.glob(
    %w[
      README.md
      LICENSE
      ext/**/*.rs
      ext/**/*.rb
      ext/**/*.toml
      ext/**/*.lock
      ext/**/*.md
      ext/**/build.rs
      ext/**/Cargo.*
      exe/*
      lib/**/*.rb
      sig/**/*.rbs
      spec/**/*.rb
    ],
    File::FNM_DOTMATCH
  )
end

# Collect crate source files for vendoring into the gem
crate_files = Dir.chdir(repo_root) do
  crate_dirs = %w[
    kreuzberg
    kreuzberg-ffi
    kreuzberg-tesseract
    kreuzberg-paddle-ocr
    kreuzberg-pdfium-render
  ]

  crate_dirs.flat_map do |crate|
    Dir.glob("crates/#{crate}/**/*", File::FNM_DOTMATCH)
       .reject { |f| File.directory?(f) }
       .reject { |f| f.include?('/.fastembed_cache/') }
       .reject { |f| f.include?('/target/') }
       .grep_v(/\.(swp|bak|tmp)$/)
       .grep_v(/~$/)
       .map { |path| "vendor/#{path.delete_prefix('crates/')}" }
  end
end

vendor_files = Dir.chdir(__dir__) do
  kreuzberg_files = if Dir.exist?('vendor/kreuzberg')
                      Dir.glob('vendor/kreuzberg/**/*', File::FNM_DOTMATCH)
                         .reject { |f| File.directory?(f) }
                         .reject { |f| f.include?('/.fastembed_cache/') }
                         .reject { |f| f.include?('/.kreuzberg/') }
                         .reject { |f| f.include?('/target/') }
                         .grep_v(/\.(swp|bak|tmp)$/)
                         .grep_v(/~$/)
                    else
                      []
                    end

  kreuzberg_ffi_files = if Dir.exist?('vendor/kreuzberg-ffi')
                          Dir.glob('vendor/kreuzberg-ffi/**/*', File::FNM_DOTMATCH)
                             .reject { |f| File.directory?(f) }
                             .reject { |f| f.include?('/target/') }
                             .grep_v(/\.(swp|bak|tmp)$/)
                             .grep_v(/~$/)
                        else
                          []
                        end

  kreuzberg_tesseract_files = if Dir.exist?('vendor/kreuzberg-tesseract')
                                Dir.glob('vendor/kreuzberg-tesseract/**/*', File::FNM_DOTMATCH)
                                   .reject { |f| File.directory?(f) }
                                   .reject { |f| f.include?('/target/') }
                                   .grep_v(/\.(swp|bak|tmp)$/)
                                   .grep_v(/~$/)
                              else
                                []
                              end

  kreuzberg_paddle_ocr_files = if Dir.exist?('vendor/kreuzberg-paddle-ocr')
                                 Dir.glob('vendor/kreuzberg-paddle-ocr/**/*', File::FNM_DOTMATCH)
                                    .reject { |f| File.directory?(f) }
                                    .reject { |f| f.include?('/target/') }
                                    .grep_v(/\.(swp|bak|tmp)$/)
                                    .grep_v(/~$/)
                               else
                                 []
                               end

  kreuzberg_pdfium_render_files = if Dir.exist?('vendor/kreuzberg-pdfium-render')
                                    Dir.glob('vendor/kreuzberg-pdfium-render/**/*', File::FNM_DOTMATCH)
                                       .reject { |f| File.directory?(f) }
                                       .reject { |f| f.include?('/target/') }
                                       .grep_v(/\.(swp|bak|tmp)$/)
                                       .grep_v(/~$/)
                                  else
                                    []
                                  end

  rb_sys_files = if Dir.exist?('vendor/rb-sys')
                   Dir.glob('vendor/rb-sys/**/*', File::FNM_DOTMATCH)
                      .reject { |f| File.directory?(f) }
                      .reject { |f| f.include?('/target/') }
                      .grep_v(/\.(swp|bak|tmp)$/)
                      .grep_v(/~$/)
                 else
                   []
                 end

  workspace_toml = if File.exist?('vendor/Cargo.toml')
                     ['vendor/Cargo.toml']
                   else
                     []
                   end

  kreuzberg_files + kreuzberg_ffi_files + kreuzberg_tesseract_files +
    kreuzberg_paddle_ocr_files + kreuzberg_pdfium_render_files + rb_sys_files + workspace_toml
end

# When vendor files exist, get ext/ files from filesystem (to include modified Cargo.toml
# with vendor paths) instead of from git (which has original 5-level crate paths)
ext_files_from_fs = Dir.chdir(__dir__) do
  Dir.glob('ext/**/*', File::FNM_DOTMATCH)
     .reject { |f| File.directory?(f) }
     .reject { |f| f.include?('/target/') }
     .grep_v(/\.(swp|bak|tmp)$/)
     .grep_v(/~$/)
end

files = if vendor_files.any?
          # Use ext/ files from filesystem (modified by vendor script) + non-ext ruby files
          non_ext_ruby_files = ruby_files.reject { |f| f.start_with?('ext/') }
          non_ext_ruby_files + ext_files_from_fs + vendor_files
        else
          ruby_files + crate_files
        end

native_artifacts = Dir.chdir(__dir__) do
  Dir.glob(%w[
             lib/**/*.bundle
             lib/**/*.so
             lib/**/*.dll
             lib/**/*.dylib
           ])
end
files.concat(native_artifacts)

files = files.select { |f| File.exist?(f) }
files = files.uniq

Gem::Specification.new do |spec|
  spec.name = 'kreuzberg'
  spec.version = Kreuzberg::VERSION
  spec.authors = ['Na\'aman Hirschfeld']
  spec.email = ['nhirschfeld@gmail.com']

  spec.summary = 'Document intelligence library — extract text from PDFs, Office docs, images, and 75+ formats'
  spec.description = <<~DESC
    Kreuzberg is a high-performance document intelligence library with a Rust core and native
    Ruby bindings via Magnus. Extract text, metadata, and structured data from 75+ file formats
    including PDF, DOCX, PPTX, XLSX, HTML, RTF, images (with OCR), email, archives, and more.
    Features async/sync APIs, text chunking, language detection, and keyword extraction.
  DESC
  spec.homepage = 'https://github.com/kreuzberg-dev/kreuzberg'
  spec.license = 'Elastic-2.0'
  spec.required_ruby_version = ['>= 3.2.0', '< 5.0']

  spec.metadata = {
    'homepage_uri' => spec.homepage,
    'source_code_uri' => 'https://github.com/kreuzberg-dev/kreuzberg',
    'changelog_uri' => 'https://github.com/kreuzberg-dev/kreuzberg/blob/main/CHANGELOG.md',
    'documentation_uri' => 'https://docs.kreuzberg.dev',
    'bug_tracker_uri' => 'https://github.com/kreuzberg-dev/kreuzberg/issues',
    'rubygems_mfa_required' => 'true',
    'keywords' => 'document-intelligence,document-extraction,text-extraction,ocr,pdf,rust,native-extension,nlp,rag'
  }

  spec.files = files
  spec.bindir = 'exe'
  spec.executables = spec.files.grep(%r{^exe/}) { |f| File.basename(f) }
  spec.require_paths = ['lib']
  spec.extensions = ['ext/kreuzberg_rb/extconf.rb']

  spec.add_dependency 'rb_sys', '~> 0.9.119'

  spec.add_development_dependency 'bundler', '~> 4.0'
  spec.add_development_dependency 'rake', '~> 13.0'
  spec.add_development_dependency 'rake-compiler', '~> 1.2'
  spec.add_development_dependency 'rspec', '~> 3.12'
  spec.add_dependency 'sorbet-runtime', '~> 0.5'
  unless Gem.win_platform?
    spec.add_development_dependency 'rbs', '~> 4.0'
    spec.add_development_dependency 'rubocop', '~> 1.66'
    spec.add_development_dependency 'rubocop-performance', '~> 1.21'
    spec.add_development_dependency 'rubocop-rspec', '~> 3.0'
    spec.add_development_dependency 'steep', '~> 2.0'
  end
  spec.add_development_dependency 'yard', '~> 0.9'
end
