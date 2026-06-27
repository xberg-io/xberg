```php title="Document Structure Config (PHP)"
<?php
use Xberg\ExtractionConfig;
use Xberg\Xberg;

$config = new ExtractionConfig(includeDocumentStructure: true);

$result = Xberg::extractSync('document.pdf', $config);

if ($result->document !== null) {
    foreach ($result->document->nodes as $node) {
        echo "[{$node->content->nodeType}]\n";
    }
}
?>
```
