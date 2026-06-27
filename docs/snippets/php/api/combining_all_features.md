```php title="PHP"
<?php
declare(strict_types=1);

use Xberg\Xberg;
use Xberg\ExtractionConfig;
use Xberg\OcrConfig;
use Xberg\ChunkingConfig;
use Xberg\ChunkSizing;
use Xberg\ImageExtractionConfig;
use Xberg\OutputFormat;

// Build config with OCR, chunking, and image extraction
$config = new ExtractionConfig(
    null,                                    // caching
    false,                                   // force_ocr
    null,                                    // max_concurrent_extractions
    null,                                    // cache_dir
    OutputFormat::Markdown,                  // output_format
    true,                                    // include_document_structure
    true,                                    // enable_quality_processing
    true,                                    // use_cache
    null,                                    // use_diffs
    null,                                    // keep_empty_chunks
);

// Set OCR: Tesseract with English language
$ocrConfig = new OcrConfig(
    'tesseract',                             // backend
    'eng',                                   // language
    null,                                    // page_count_hint
    null,                                    // psm_mode
    null,                                    // use_gpu
    null,                                    // languages
    null,                                    // fast_mode
    null,                                    // fast_weight
    null,                                    // min_confidence
);
$config->setOcr($ocrConfig);

// Set chunking: semantic markdown chunks ~800 chars, 100-char overlap
$chunkingConfig = new ChunkingConfig(
    800,                                     // max_characters
    100,                                     // overlap
    true,                                    // trim
    'Markdown',                              // chunker_type
    null,                                    // preset
    true,                                    // prepend_heading_context
    null,                                    // topic_threshold
);
$config->setChunking($chunkingConfig);

// Set image extraction
$imageConfig = new ImageExtractionConfig(
    true,                                    // extract_images
    null,                                    // image_min_width
    null,                                    // image_min_height
    null,                                    // image_output_format
    null,                                    // image_compression_level
);
$config->setImages($imageConfig);

$result = Xberg::extractSync('report.pdf', null, $config);

echo "Content (" . strlen($result->getContent()) . " chars):\n";
echo substr($result->getContent(), 0, 200) . "\n\n";

if ($result->getChunks() !== null) {
    echo "Chunks: " . count($result->getChunks()) . "\n";
}
echo "Tables: " . count($result->getTables()) . "\n";

if ($result->getDetectedLanguages() !== null) {
    echo "Languages: " . implode(', ', $result->getDetectedLanguages()) . "\n";
}

if ($result->getExtractionMethod() !== null) {
    echo "Extraction method: " . $result->getExtractionMethod() . "\n";
}
```
