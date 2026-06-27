```php title="Element-Based Output (PHP)"
<?php
use Xberg\ExtractionConfig;
use Xberg\Xberg;

// Configure element-based output
$config = ExtractionConfig::default();
$config->setOutputFormat('element_based');

// Extract document
$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);
$result = $resultOutput->results[0];

// Access elements
foreach ($result->getElements() as $element) {
    echo "Type: " . $element->getElementType() . "\n";
    echo "Text: " . substr($element->getText(), 0, 100) . "\n";

    if ($element->getMetadata()->getPageNumber()) {
        echo "Page: " . $element->getMetadata()->getPageNumber() . "\n";
    }

    if ($element->getMetadata()->getCoordinates()) {
        $coords = $element->getMetadata()->getCoordinates();
        echo sprintf("Coords: (%s, %s) - (%s, %s)\n",
            $coords->getLeft(), $coords->getTop(),
            $coords->getRight(), $coords->getBottom());
    }

    echo "---\n";
}

// Filter by element type
$titles = array_filter($result->getElements(), function($e) {
    return $e->getElementType() === 'title';
});

foreach ($titles as $title) {
    $level = $title->getMetadata()->getAdditional()['level'] ?? 'unknown';
    echo "[{$level}] {$title->getText()}\n";
}
?>
```
