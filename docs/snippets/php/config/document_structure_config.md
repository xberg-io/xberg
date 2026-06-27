```php title="Document Structure Config (PHP)"
<?php
use Xberg\ExtractionConfig;
use Xberg\Xberg;

$config = new ExtractionConfig(includeDocumentStructure: true);

$resultOutput = Xberg::extract(\Xberg\ExtractInput::uri('document.pdf'), $config);

$result = $resultOutput->results[0];

if ($result->document !== null) {
    foreach ($result->document->nodes as $node) {
        echo "[{$node->content->nodeType}]\n";
    }
}
?>
```
