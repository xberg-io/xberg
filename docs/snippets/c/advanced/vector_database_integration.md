```c title="C"
#include "xberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    const char *document_path = "document.pdf";
    const char *document_id = "doc-001";

    const char *config_json =
        "{"
        "\"chunking\": {"
        "\"chunker_type\": \"character\","
        "\"max_characters\": 512,"
        "\"overlap\": 50,"
        "\"embedding\": {"
        "\"model\": {\"preset\": {\"name\": \"balanced\"}},"
        "\"normalize\": true,"
        "\"batch_size\": 32"
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
        xberg_extract_sync(document_path, NULL, config);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                xberg_last_error_code(),
                xberg_last_error_context());
        xberg_extraction_config_free(config);
        return 1;
    }

    /* The chunks JSON array carries content + embedding + metadata for each
       chunk. Pass this directly to your vector database client (pgvector,
       Qdrant, Pinecone, etc.) along with the document_id as a metadata field. */
    char *chunks_json = xberg_extraction_result_chunks(result);
    printf("document_id: %s\n", document_id);
    printf("chunks (JSON, ready to upsert into a vector DB):\n%s\n",
           chunks_json ? chunks_json : "[]");
    xberg_free_string(chunks_json);

    xberg_extraction_result_free(result);
    xberg_extraction_config_free(config);
    return 0;
}
```
