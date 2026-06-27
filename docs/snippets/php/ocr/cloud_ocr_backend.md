```php title="PHP"
<?php
declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\Config\ExtractionConfig;
use Xberg\Config\OcrConfig;
use Xberg\Config\LlmConfig;

// Cloud-based OCR using Vision Language Model (VLM)
// Requires API key and model configuration
$config = new ExtractionConfig(
    ocr: new OcrConfig(
        backend: 'vlm',
        language: 'eng',
        vlmConfig: new LlmConfig(
            provider: 'anthropic',
            apiKey: getenv('ANTHROPIC_API_KEY'),
            model: 'claude-3-5-sonnet-20241022'
        ),
        vlmPrompt: 'Extract all text from this document page. Preserve formatting and structure.'
    )
);

$xberg = new Xberg($config);
$result = $xberg->extract('document.pdf');

echo "Cloud OCR Results:\n";
echo "Content length: " . strlen($result->content) . " characters\n";
echo "Preview: " . substr($result->content, 0, 200) . "...\n";
?>
```
