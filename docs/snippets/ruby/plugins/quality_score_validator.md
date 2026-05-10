```ruby title="Ruby"
require "kreuzberg"

# Custom validator checking document quality score
class QualityScoreValidator
  def initialize(min_score: 0.5)
    @min_score = min_score
  end

  def call(result)
    quality_score = result.quality_score || 0.0

    if quality_score < @min_score
      raise StandardError,
            format("Quality score too low: %.2f < %.2f", quality_score, @min_score)
    end
  end
end

# Register with default minimum score of 0.5
validator = QualityScoreValidator.new(min_score: 0.5)
Kreuzberg.register_validator("quality_score_check", validator)

# Usage with quality processing enabled
config = Kreuzberg::ExtractionConfig.new(
  enable_quality_processing: true
)

begin
  result = Kreuzberg.extract_file_sync("document.pdf", config: config)
  puts "Document quality verified: #{result.quality_score}"
rescue StandardError => e
  puts "Quality check failed: #{e.message}"
end
```
