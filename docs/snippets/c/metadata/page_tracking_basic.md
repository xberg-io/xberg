```c title="C"
#include "kreuzberg.h"
#include <stdio.h>
#include <string.h>

int main(void) {
    const char *config_json =
        "{"
        "\"pages\": {"
        "\"extract_pages\": true"
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

    char *pages_json = kreuzberg_extraction_result_pages(result);
    if (pages_json) {
        printf("Pages (JSON array): %s\n", pages_json);
        kreuzberg_free_string(pages_json);
    } else {
        printf("No pages available\n");
    }

    KREUZBERGMetadata *metadata = kreuzberg_extraction_result_metadata(result);
    if (metadata) {
        KREUZBERGPageStructure *pages = kreuzberg_metadata_pages(metadata);
        if (pages) {
            printf("Total page count: %zu\n", kreuzberg_page_structure_total_count(pages));
            kreuzberg_page_structure_free(pages);
        }
        kreuzberg_metadata_free(metadata);
    }

    kreuzberg_extraction_result_free(result);
    kreuzberg_extraction_config_free(config);
    return 0;
}
```
