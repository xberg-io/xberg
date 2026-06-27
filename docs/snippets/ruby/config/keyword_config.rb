require 'xberg'

# Example 1: Basic YAKE configuration
# Uses YAKE algorithm with default parameters and English stopword filtering
def basic_yake
  config = Xberg::ExtractionConfig.new(
    keywords: Xberg::KeywordConfig.new(
      algorithm: :yake,
      max_keywords: 10,
      min_score: 0.0,
      ngram_range: [1, 3],
      language: 'en',
      yake_params: nil,
      rake_params: nil
    )
  )

  result = Xberg.extract('document.pdf', config: config)
  puts "Keywords: #{result.keywords}"
end

# Example 2: Advanced YAKE with custom parameters
# Fine-tunes YAKE with custom window size for co-occurrence analysis
def advanced_yake
  config = Xberg::ExtractionConfig.new(
    keywords: Xberg::KeywordConfig.new(
      algorithm: :yake,
      max_keywords: 15,
      min_score: 0.1,
      ngram_range: [1, 2],
      language: 'en',
      yake_params: Xberg::YakeParams.new(
        window_size: 1
      ),
      rake_params: nil
    )
  )

  result = Xberg.extract('document.pdf', config: config)
  puts "Keywords: #{result.keywords}"
end

# Example 3: RAKE configuration
# Uses RAKE algorithm for rapid keyword extraction with phrase constraints
def rake_config
  config = Xberg::ExtractionConfig.new(
    keywords: Xberg::KeywordConfig.new(
      algorithm: :rake,
      max_keywords: 10,
      min_score: 5.0,
      ngram_range: [1, 3],
      language: 'en',
      yake_params: nil,
      rake_params: Xberg::RakeParams.new(
        min_word_length: 1,
        max_words_per_phrase: 3
      )
    )
  )

  result = Xberg.extract('document.pdf', config: config)
  puts "Keywords: #{result.keywords}"
end

basic_yake if __FILE__ == $0
