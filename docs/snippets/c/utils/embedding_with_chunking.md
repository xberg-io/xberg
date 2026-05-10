```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *config_json =
        "{"
        "\"chunking\": {"
        "\"chunker_type\": \"character\","
        "\"max_characters\": 1024,"
        "\"overlap\": 100,"
        "\"embedding\": {"
        "\"model\": {\"preset\": {\"name\": \"balanced\"}},"
        "\"normalize\": true,"
        "\"batch_size\": 32,"
        "\"show_download_progress\": false"
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
    printf("chunks with embeddings (JSON):\n%s\n", chunks_json ? chunks_json : "[]");
    kreuzberg_free_string(chunks_json);

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
