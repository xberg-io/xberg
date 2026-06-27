```c title="C"
#include "xberg.h"
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

    XBERGExtractionConfig *config = xberg_extraction_config_from_json(config_json);
    if (!config) {
        fprintf(stderr, "config parse failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        return 1;
    }

    XBERGExtractionResult *result =
        xberg_extract_sync("document.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extraction_config_free(config);
        return 1;
    }

    char *chunks_json = xberg_extraction_result_chunks(result);
    if (chunks_json) {
        printf("Chunks with embeddings (JSON): %s\n", chunks_json);
        xberg_free_string(chunks_json);
    } else {
        printf("No chunks produced\n");
    }

    XBERGMetadata *metadata = xberg_extraction_result_metadata(result);
    if (metadata) {
        char *title = xberg_metadata_title(metadata);
        if (title) {
            printf("Document title: %s\n", title);
            xberg_free_string(title);
        }
        xberg_metadata_free(metadata);
    }

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
