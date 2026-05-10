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
    if (content) {
        printf("content length: %zu bytes\n", strlen(content));
        printf("%s\n", content);
    }
    kreuzberg_free_string(content);

    /* Tables are returned as a JSON array string. A real consumer would
     * feed this into a JSON parser and walk each table's grid. */
    char *tables_json = kreuzberg_extraction_result_tables(result);
    if (tables_json) {
        printf("tables JSON (%zu bytes):\n%s\n",
               strlen(tables_json), tables_json);
    } else {
        printf("tables JSON: (none)\n");
    }
    kreuzberg_free_string(tables_json);

    kreuzberg_extraction_result_free(result);
    return 0;
}
```
