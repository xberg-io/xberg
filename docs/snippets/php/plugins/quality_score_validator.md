```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

class QualityScoreValidator implements Validator {
    private float $minQualityScore = 0.7;

    public function name(): string {
        return "quality-score-validator";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        // Load quality scoring models or rules
    }

    public function shutdown(): void {
        // Cleanup resources
    }

    public function validate(object $result, object $config): void {
        $qualityScore = $this->calculateQualityScore($result);
        
        if ($qualityScore < $this->minQualityScore) {
            throw new Exception(
                sprintf(
                    "Quality score too low: %.2f < %.2f",
                    $qualityScore,
                    $this->minQualityScore
                )
            );
        }
    }

    public function priority(): int {
        return 90;
    }

    private function calculateQualityScore(object $result): float {
        $score = 1.0;
        
        // Penalize if content is too short
        if (strlen($result->content) < 100) {
            $score *= 0.8;
        }
        
        // Penalize if many detection warnings
        if (isset($result->processing_warnings) && count($result->processing_warnings) > 5) {
            $score *= 0.9;
        }
        
        // Reward if language was detected
        if (isset($result->detected_languages) && !empty($result->detected_languages)) {
            $score *= 1.05;
        }
        
        return min(1.0, $score);
    }
}

// Register the quality score validator
$validator = new QualityScoreValidator();
Kreuzberg::registerValidator($validator);

echo "Quality score validator registered\n";
```
