<!-- snippet:syntax-only -->

```php title="PHP"
<?php

declare(strict_types=1);

require_once __DIR__ . '/vendor/autoload.php';

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\LlmConfig;
use Xberg\StructuredExtractionConfig;

$schema = json_encode([
    'type' => 'object',
    'properties' => [
        'title' => ['type' => 'string'],
        'authors' => ['type' => 'array', 'items' => ['type' => 'string']],
        'date' => ['type' => 'string'],
    ],
    'required' => ['title', 'authors', 'date'],
    'additionalProperties' => false,
], JSON_THROW_ON_ERROR);

$llm = new LlmConfig(
    model: 'openai/gpt-4o-mini',
    apiKey: null,
    baseUrl: null,
    timeoutSecs: null,
    maxRetries: null,
    temperature: null,
    maxTokens: null,
);

$structured = StructuredExtractionConfig::from_json(json_encode([
    'schema' => json_decode($schema, true),
    'schema_name' => 'paper_metadata',
    'strict' => true,
    'llm' => [
        'model' => $llm->model,
    ],
], JSON_THROW_ON_ERROR));

$config = new ExtractionConfig();
$config->structured_extraction = $structured;

$xberg = new Xberg($config);
$result = $xberg->extract('paper.pdf');

if ($result->structured_output !== null) {
    echo $result->structured_output, "\n";
}
```
