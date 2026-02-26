/*
 * Basic text extraction example using kreuzberg-ffi.
 *
 * Demonstrates extracting text from a byte buffer with a known MIME type.
 *
 * Compile:
 *   make basic_extraction
 *   # or:
 *   cc -o basic_extraction basic_extraction.c $(pkg-config --cflags --libs kreuzberg)
 */

#include <kreuzberg.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    printf("kreuzberg-ffi version: %s\n\n", kreuzberg_version());

    /* Extract text from an HTML byte buffer */
    const char *html = "<html><body><h1>Hello</h1><p>World from Kreuzberg!</p></body></html>";
    struct CExtractionResult *result =
        kreuzberg_extract_bytes_sync((const uint8_t *)html, strlen(html), "text/html");

    if (result == NULL) {
        fprintf(stderr, "Extraction failed: %s\n", kreuzberg_last_error());
        return 1;
    }

    if (!result->success) {
        fprintf(stderr, "Extraction returned failure\n");
        kreuzberg_free_result(result);
        return 1;
    }

    printf("Extracted text:\n%s\n", result->content);
    printf("MIME type: %s\n", result->mime_type ? result->mime_type : "(none)");

    kreuzberg_free_result(result);
    return 0;
}
