```php title="PHP"
<?php
declare(strict_types=1);

use Kreuzberg\Kreuzberg;
use Kreuzberg\ExtractionConfig;
use Kreuzberg\ChunkingConfig;
use Kreuzberg\PageConfig;

$config = new ExtractionConfig(
    chunking: new ChunkingConfig(
        maxCharacters: 500,
        overlap: 50
    ),
    pages: new PageConfig(
        extractPages: true
    )
);

$result = Kreuzberg::extractFileSync('document.pdf', null, $config);

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
