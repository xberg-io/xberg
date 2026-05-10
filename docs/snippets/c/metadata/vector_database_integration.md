```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    const char *config_json =
        "{"
        "\"chunking\": {"
        "\"chunker_type\": \"character\","
        "\"max_characters\": 512,"
        "\"overlap\": 50,"
        "\"embedding\": {"
        "\"model\": {\"preset\": {\"name\": \"balanced\"}},"
        "\"normalize\": true"
        "}"
        "}"
        "}";

    KREUZBERGExtractionConfig *config = kreuzberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    KREUZBERGExtractionResult *result =
        kreuzberg_extract_file_sync("document.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    char *chunks_json = kreuzberg_extraction_result_chunks(result);
    if (chunks_json) {
        printf("Chunks with embeddings (JSON): %s\n", chunks_json);
        kreuzberg_free_string(chunks_json);
    } else {
        printf("No chunks produced\n");
    }

    KREUZBERGMetadata *metadata = kreuzberg_extraction_result_metadata(result);
    if (metadata) {
        char *title = kreuzberg_metadata_title(metadata);
        if (title) {
            printf("Document title: %s\n", title);
            kreuzberg_free_string(title);
        }
        kreuzberg_metadata_free(metadata);
    }

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
