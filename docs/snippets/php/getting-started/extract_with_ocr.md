```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\OcrConfig;

$ocrConfig = new OcrConfig();
$ocrConfig->setBackend('tesseract');
$ocrConfig->setLanguage('eng');

$config = new ExtractionConfig();
$config->setForceOcr(true);
$config->setOcr($ocrConfig);

$result = Xberg::extractSync('scanned.pdf', null, $config);

echo "Content:\n";
echo $result->getContent();

if ($result->getDetectedLanguages() !== null) {
    echo "Detected Languages: " . implode(', ', $result->getDetectedLanguages()) . "\n";
}
```
