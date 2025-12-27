<?php
/**
 * Basic usage examples for Kreuzberg PHP bindings
 */

if (!extension_loaded('kreuzberg')) {
    die("Kreuzberg extension not loaded. Please install and enable it in php.ini\n");
}

echo "Kreuzberg PHP Bindings Examples\n";
echo "================================\n\n";

echo "Example 1: Extract a PDF file\n";
echo "------------------------------\n";
try {
    $result = kreuzberg_extract_file("document.pdf");
    echo "MIME Type: {$result->mime_type}\n";
    echo "Content length: " . strlen($result->content) . " characters\n";
    echo "Number of tables: " . count($result->tables) . "\n";
    echo "Content preview: " . substr($result->content, 0, 200) . "...\n\n";
} catch (Exception $e) {
    echo "Error: " . $e->getMessage() . "\n\n";
}

echo "Example 2: Extract with custom OCR\n";
echo "-----------------------------------\n";
try {
    $config = new ExtractionConfig();
    $config->force_ocr = true;
    $config->ocr = new OcrConfig();
    $config->ocr->language = "eng";

    $result = kreuzberg_extract_file("scanned.pdf", null, $config);
    echo "Extracted with OCR\n";
    echo "Content length: " . strlen($result->content) . " characters\n\n";
} catch (Exception $e) {
    echo "Error: " . $e->getMessage() . "\n\n";
}

echo "Example 3: Extract from bytes\n";
echo "------------------------------\n";
try {
    if (file_exists("document.pdf")) {
        $data = file_get_contents("document.pdf");
        $result = kreuzberg_extract_bytes($data, "application/pdf");
        echo "Extracted from bytes\n";
        echo "Content length: " . strlen($result->content) . " characters\n\n";
    } else {
        echo "Skipping (document.pdf not found)\n\n";
    }
} catch (Exception $e) {
    echo "Error: " . $e->getMessage() . "\n\n";
}

echo "Example 4: Batch processing\n";
echo "---------------------------\n";
try {
    $paths = ["doc1.pdf", "doc2.txt", "doc3.docx"];

    $existing_paths = array_filter($paths, 'file_exists');

    if (!empty($existing_paths)) {
        $results = kreuzberg_batch_extract_files($existing_paths);

        foreach ($results as $i => $result) {
            echo "Document " . ($i + 1) . ": {$result->mime_type}\n";
            echo "  Content: " . strlen($result->content) . " characters\n";
        }
        echo "\n";
    } else {
        echo "No files found for batch processing\n\n";
    }
} catch (Exception $e) {
    echo "Error: " . $e->getMessage() . "\n\n";
}

echo "Example 5: Extract tables\n";
echo "-------------------------\n";
try {
    if (file_exists("spreadsheet.xlsx")) {
        $result = kreuzberg_extract_file("spreadsheet.xlsx");

        foreach ($result->tables as $table) {
            echo "Table on page {$table->page_number}:\n";
            echo $table->markdown . "\n";

            $rows = count($table->cells);
            $cols = $rows > 0 ? count($table->cells[0]) : 0;
            echo "Dimensions: {$rows} rows x {$cols} columns\n\n";
        }
    } else {
        echo "Skipping (spreadsheet.xlsx not found)\n\n";
    }
} catch (Exception $e) {
    echo "Error: " . $e->getMessage() . "\n\n";
}

echo "Example 6: MIME type detection\n";
echo "-------------------------------\n";
try {
    if (file_exists("document.pdf")) {
        $mime_type = kreuzberg_detect_mime_type_from_path("document.pdf");
        echo "Detected MIME type: $mime_type\n";

        $extensions = kreuzberg_get_extensions_for_mime($mime_type);
        echo "Extensions: " . implode(", ", $extensions) . "\n\n";
    } else {
        echo "Skipping (document.pdf not found)\n\n";
    }
} catch (Exception $e) {
    echo "Error: " . $e->getMessage() . "\n\n";
}

echo "Example 7: Metadata extraction\n";
echo "-------------------------------\n";
try {
    if (file_exists("document.pdf")) {
        $result = kreuzberg_extract_file("document.pdf");

        echo "Metadata:\n";
        foreach ($result->metadata as $key => $value) {
            echo "  {$key}: {$value}\n";
        }
        echo "\n";
    } else {
        echo "Skipping (document.pdf not found)\n\n";
    }
} catch (Exception $e) {
    echo "Error: " . $e->getMessage() . "\n\n";
}

echo "Example 8: Language detection\n";
echo "------------------------------\n";
try {
    $config = new ExtractionConfig();
    $config->language_detection = new LanguageDetectionConfig();
    $config->language_detection->enabled = true;
    $config->language_detection->min_confidence = 0.8;

    if (file_exists("document.pdf")) {
        $result = kreuzberg_extract_file("document.pdf", null, $config);

        if ($result->detected_languages) {
            echo "Detected languages: " . implode(", ", $result->detected_languages) . "\n";
            $primary = $result->get_detected_language();
            echo "Primary language: $primary\n\n";
        } else {
            echo "No languages detected\n\n";
        }
    } else {
        echo "Skipping (document.pdf not found)\n\n";
    }
} catch (Exception $e) {
    echo "Error: " . $e->getMessage() . "\n\n";
}

echo "Example 9: Text chunking\n";
echo "------------------------\n";
try {
    $config = new ExtractionConfig();
    $config->chunking = new ChunkingConfig();
    $config->chunking->max_chars = 500;
    $config->chunking->max_overlap = 100;

    if (file_exists("document.pdf")) {
        $result = kreuzberg_extract_file("document.pdf", null, $config);

        if ($result->chunks) {
            echo "Number of chunks: " . $result->get_chunk_count() . "\n";

            foreach ($result->chunks as $i => $chunk) {
                echo "Chunk " . ($i + 1) . ":\n";
                echo "  Length: " . strlen($chunk->content) . " characters\n";
                echo "  Token count: {$chunk->token_count}\n";
                echo "  Preview: " . substr($chunk->content, 0, 100) . "...\n";
            }
            echo "\n";
        } else {
            echo "No chunks generated\n\n";
        }
    } else {
        echo "Skipping (document.pdf not found)\n\n";
    }
} catch (Exception $e) {
    echo "Error: " . $e->getMessage() . "\n\n";
}

echo "Example 10: Version info\n";
echo "------------------------\n";
$version = kreuzberg_version();
echo "Kreuzberg version: $version\n\n";

echo "All examples completed!\n";
