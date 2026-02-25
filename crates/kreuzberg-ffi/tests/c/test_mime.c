#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

/* Create an empty temp file for MIME detection tests (check_exists=true in FFI) */
static void create_temp_file(const char *name) {
    FILE *f = fopen(name, "w");
    assert(f != NULL);
    fclose(f);
}

int main(void) {
    /* Create temp files so detect_mime_type_from_path's existence check passes */
    create_temp_file("_test_mime.pdf");
    create_temp_file("_test_mime.txt");
    create_temp_file("_test_mime.html");

    /* Test MIME type detection from path */
    char *mime = kreuzberg_detect_mime_type_from_path("_test_mime.pdf");
    assert(mime != NULL);
    assert(strcmp(mime, "application/pdf") == 0);
    kreuzberg_free_string(mime);

    mime = kreuzberg_detect_mime_type_from_path("_test_mime.txt");
    assert(mime != NULL);
    assert(strcmp(mime, "text/plain") == 0);
    kreuzberg_free_string(mime);

    mime = kreuzberg_detect_mime_type_from_path("_test_mime.html");
    assert(mime != NULL);
    assert(strcmp(mime, "text/html") == 0);
    kreuzberg_free_string(mime);

    /* Clean up temp files */
    remove("_test_mime.pdf");
    remove("_test_mime.txt");
    remove("_test_mime.html");

    /* Test nonexistent file returns NULL */
    mime = kreuzberg_detect_mime_type_from_path("/nonexistent/file.pdf");
    assert(mime == NULL);

    /* Test MIME validation - returns a string (non-NULL means valid) */
    char *valid = kreuzberg_validate_mime_type("application/pdf");
    assert(valid != NULL);
    kreuzberg_free_string(valid);

    valid = kreuzberg_validate_mime_type("text/plain");
    assert(valid != NULL);
    kreuzberg_free_string(valid);

    /* Test NULL path */
    mime = kreuzberg_detect_mime_type_from_path(NULL);
    assert(mime == NULL);

    printf("test_mime: all tests passed\n");
    return 0;
}
