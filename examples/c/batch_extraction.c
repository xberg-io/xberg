/*
 * Batch extraction example using kreuzberg-ffi.
 *
 * Demonstrates extracting text from multiple byte buffers in a single call.
 *
 * Compile:
 *   make batch_extraction
 *   # or:
 *   cc -o batch_extraction batch_extraction.c $(pkg-config --cflags --libs kreuzberg)
 */

#include <kreuzberg.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    printf("kreuzberg-ffi batch extraction example\n\n");

    /* Prepare multiple text inputs */
    const char *texts[] = {
        "First document content.",
        "Second document with more text.",
        "Third document: hello world!",
    };
    const char *mimes[] = {
        "text/plain",
        "text/plain",
        "text/plain",
    };
    const int count = 3;

    /* Extract each one individually (batch API requires Option_ErrorCallback
       which is not exposed in the C header -- use sequential extraction) */
    for (int i = 0; i < count; i++) {
        printf("--- Document %d ---\n", i + 1);

        struct CExtractionResult *result = kreuzberg_extract_bytes_sync(
            (const uint8_t *)texts[i], strlen(texts[i]), mimes[i]);

        if (result == NULL) {
            fprintf(stderr, "  Error: %s\n", kreuzberg_last_error());
            continue;
        }

        if (result->success && result->content) {
            printf("  Content: %s\n", result->content);
        } else {
            printf("  Extraction returned failure\n");
        }

        kreuzberg_free_result(result);
    }

    printf("\nDone.\n");
    return 0;
}
