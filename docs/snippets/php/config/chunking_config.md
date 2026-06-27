```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\ChunkingConfig;

// Basic chunking
$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxCharacters: 1000,
        overlap: 200
    )
);

$result = Xberg::extractSync('document.pdf', null, $config);

echo "Number of chunks: " . count($result->getChunks()) . "\n";
foreach ($result->getChunks() as $chunk) {
    echo "Chunk size: " . strlen($chunk->getContent()) . " characters\n";
}
?>
```

```php title="PHP - Markdown with Heading Context"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\ChunkingConfig;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxCharacters: 500,
        overlap: 50,
        chunkerType: 'markdown',
        prependHeadingContext: true
    )
);

$result = Xberg::extractSync('document.md', null, $config);

foreach ($result->getChunks() as $chunk) {
    $metadata = $chunk->getMetadata();
    if ($metadata && $metadata->getHeadingContext()) {
        $headings = $metadata->getHeadingContext()->getHeadings();
        foreach ($headings as $heading) {
            echo "Heading L" . $heading->getLevel() . ": " . $heading->getText() . "\n";
        }
    }
    echo "Content: " . substr($chunk->getContent(), 0, 100) . "...\n";
}
?>
```
