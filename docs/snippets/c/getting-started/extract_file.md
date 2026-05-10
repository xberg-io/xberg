```c title="C"
#include <kreuzberg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(void) {
    KREUZBERGExtractionResult *result =
        kreuzberg_extract_file("document.pdf", NULL, NULL);
    if (!result) {
        fprintf(stderr, "extraction failed (code %d): %s\n",
                kreuzberg_last_error_code(),
                kreuzberg_last_error_context());
        return 1;
    }

    char *content = kreuzberg_extraction_result_content(result);
    printf("content:\n%s\n", content ? content : "(empty)");
    kreuzberg_free_string(content);

    char *tables_json = kreuzberg_extraction_result_tables(result);
    printf("tables (JSON): %s\n", tables_json ? tables_json : "[]");
    kreuzberg_free_string(tables_json);

    KREUZBERGMetadata *metadata = kreuzberg_extraction_result_metadata(result);
    if (metadata) {
        char *title = kreuzberg_metadata_title(metadata);
        char *language = kreuzberg_metadata_language(metadata);
        printf("title: %s\n", title ? title : "(none)");
        printf("language: %s\n", language ? language : "(none)");
        kreuzberg_free_string(title);
        kreuzberg_free_string(language);
        kreuzberg_metadata_free(metadata);
    }

    kreuzberg_extraction_result_free(result);
    return 0;
}
```
