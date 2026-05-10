```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <stdlib.h>

int main(void) {
    /* Items is a JSON array of BatchFileItem objects.
     * Each entry has a "path" field and an optional "config" override. */
    const char *items_json =
        "["
        "  {\"path\": \"doc1.pdf\"},"
        "  {\"path\": \"doc2.docx\"},"
        "  {\"path\": \"scan.png\", \"config\": {\"force_ocr\": true}}"
        "]";

    KREUZBERGExtractionConfig *config = kreuzberg_extraction_config_default();

    /* Returns a JSON array of ExtractionResult objects, or NULL on failure. */
    char *results_json =
        kreuzberg_batch_extract_files_sync(items_json, config);
    if (!results_json) {
        fprintf(stderr, "batch extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        kreuzberg_extraction_config_free(config);
        return 1;
    }

    printf("%s\n", results_json);
    kreuzberg_free_string(results_json);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
