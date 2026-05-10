```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *config_json =
        "{"
        "\"chunking\": {"
        "\"chunker_type\": \"character\","
        "\"max_characters\": 500,"
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
        kreuzberg_extract_file_sync("research_paper.pdf", NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    char *chunks_json = kreuzberg_extraction_result_chunks(result);
    printf("chunks (JSON, each item includes content, embedding, and metadata.chunk_index/total_chunks/byte_start/byte_end):\n%s\n",
           chunks_json ? chunks_json : "[]");
    kreuzberg_free_string(chunks_json);

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
