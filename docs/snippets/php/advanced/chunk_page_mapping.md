```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\ChunkingConfig;
use Xberg\PageConfig;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxCharacters: 500,
        overlap: 50
    ),
    pages: new PageConfig(
        extractPages: true
    )
);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);

$result = $resultOutput->results[0];

if ($result->getChunks()) {
    foreach ($result->getChunks() as $chunk) {
        $metadata = $chunk->getMetadata();
        if ($metadata) {
            $firstPage = $metadata->getFirstPage();
            $lastPage = $metadata->getLastPage();

            if ($firstPage !== null && $lastPage !== null) {
                if ($firstPage === $lastPage) {
                    $pageRange = "Page " . $firstPage;
                } else {
                    $pageRange = "Pages " . $firstPage . "-" . $lastPage;
                }
                echo "Chunk: " . substr($chunk->getContent(), 0, 50) . "... (" . $pageRange . ")\n";
            }
        }
    }
}
?>
```
