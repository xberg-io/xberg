```php title="PHP"
<?php declare(strict_types=1);

use Kreuzberg\Kreuzberg;

class MinLengthValidator implements Validator {
    private int $minLength;

    public function __construct(int $minLength = 50) {
        $this->minLength = $minLength;
    }

    public function name(): string {
        return "min-length-validator";
    }

    public function version(): string {
        return "1.0.0";
    }

    public function initialize(): void {
        // Validation configuration loaded
    }

    public function shutdown(): void {
        // Cleanup
    }

    public function validate(object $result, object $config): void {
        $contentLength = strlen($result->content);
        
        if ($contentLength < $this->minLength) {
            throw new Exception(
                sprintf(
                    "Content too short: %d < %d characters",
                    $contentLength,
                    $this->minLength
                )
            );
        }
    }

    public function priority(): int {
        return 100;
    }
}

// Register validator with 50-character minimum
$validator = new MinLengthValidator(50);
Kreuzberg::registerValidator($validator);

echo "Min-length validator registered (minimum: 50 chars)\n";
```
