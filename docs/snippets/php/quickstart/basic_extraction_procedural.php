```php
<?php

declare(strict_types=1);

/**
 * Basic Document Extraction (Procedural API)
 *
 * This example shows the procedural API for document extraction,
 * which is more concise for simple use cases.
 */

require_once __DIR__ . '/vendor/autoload.php';

use function Kreuzberg\extract_file;

$result = extract_file('document.pdf');

echo "Extracted Text:\n";
echo str_repeat('=', 50) . "\n";
echo $result->content . "\n\n";

echo "Document Information:\n";
echo str_repeat('=', 50) . "\n";
printf("Title:   %s\n", $result->metadata->title ?? 'Unknown');
printf("Authors: %s\n", isset($result->metadata->authors) ? implode(', ', $result->metadata->authors) : 'Unknown');
printf("Pages:   %d\n", $result->metadata->pageCount ?? 0);
printf("Format:  %s\n", $result->mimeType);

$char_count = mb_strlen($result->content);
$word_count = str_word_count($result->content);
printf("\nStatistics:\n");
printf("Characters: %d\n", $char_count);
printf("Words:      %d\n", $word_count);
```
