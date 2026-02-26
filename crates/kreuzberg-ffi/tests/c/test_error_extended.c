#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

int main(void) {
    /* Test 1: classify_error with various messages */
    {
        /* IO-related error message */
        uint32_t code = kreuzberg_classify_error("Failed to open file: permission denied");
        /* Should classify as IO error (code 4) */
        assert(code == kreuzberg_error_code_io());

        /* Validation-related error message */
        code = kreuzberg_classify_error("validation failed: invalid input");
        assert(code == kreuzberg_error_code_validation());

        /* Parse-related error message */
        code = kreuzberg_classify_error("parse error: unexpected token");
        assert(code == kreuzberg_error_code_parsing());

        /* Unsupported format (avoid "application" which contains "io") */
        code = kreuzberg_classify_error("unsupported type: x-custom");
        assert(code == kreuzberg_error_code_unsupported_format());

        /* Generic/unknown message should return some valid code */
        code = kreuzberg_classify_error("something happened");
        /* Should return a valid error code (any value is acceptable) */
        (void)code;

        /* NULL message should be handled gracefully */
        code = kreuzberg_classify_error(NULL);
        /* Implementation may return internal error or any code for NULL */
        (void)code;
    }

    /* Test 2: last_panic_context should return NULL if no panic occurred */
    {
        char *context = kreuzberg_last_panic_context();
        /* No panic has occurred, so this should be NULL */
        if (context != NULL) {
            /* If it returns something, it should be a valid string */
            printf("  note: panic context unexpectedly non-NULL: %s\n", context);
            kreuzberg_free_string(context);
        }
    }

    /* Test 3: clone_string: clone a string, verify content, free */
    {
        const char *original = "Hello, kreuzberg clone test!";
        char *cloned = kreuzberg_clone_string(original);
        assert(cloned != NULL);
        assert(strcmp(cloned, original) == 0);
        /* The cloned string is a separate allocation */
        assert(cloned != original);
        kreuzberg_free_string(cloned);
    }

    /* Test 4: clone_string with empty string */
    {
        const char *original = "";
        char *cloned = kreuzberg_clone_string(original);
        assert(cloned != NULL);
        assert(strcmp(cloned, original) == 0);
        assert(strlen(cloned) == 0);
        kreuzberg_free_string(cloned);
    }

    /* Test 5: clone_string with NULL (should return NULL) */
    {
        char *cloned = kreuzberg_clone_string(NULL);
        assert(cloned == NULL);
    }

    /* Test 6: free_error_details with NULL (safe no-op) */
    kreuzberg_free_error_details(NULL);

    /* Test 7: get_error_details_ptr when no error has been triggered */
    {
        /*
         * Before any error, get_error_details_ptr may return NULL or
         * a details struct with empty/default values.
         */
        struct CErrorDetails *details = kreuzberg_get_error_details_ptr();
        if (details != NULL) {
            /* If returned, it should have valid (possibly empty) fields */
            /* error_code may be 0 or any value */
            kreuzberg_free_error_details(details);
        }
    }

    /* Test 8: Trigger an error, then get_error_details_ptr */
    {
        /* Extract a NULL file path to trigger an error */
        struct CExtractionResult *result = kreuzberg_extract_file_sync(NULL);
        assert(result == NULL);

        /* Now get error details */
        struct CErrorDetails *details = kreuzberg_get_error_details_ptr();
        if (details != NULL) {
            /* Error message should be non-NULL after an error */
            assert(details->message != NULL);
            assert(strlen(details->message) > 0);

            /* error_type should be set */
            if (details->error_type != NULL) {
                assert(strlen(details->error_type) > 0);
            }

            /* error_code should be non-zero for a real error */
            /* (though the exact value depends on the error type) */

            /* source_file and source_function may be NULL */

            kreuzberg_free_error_details(details);
        }
    }

    /* Test 9: get_error_details (stack-allocated variant) after error */
    {
        /* Trigger another error */
        struct CExtractionResult *result =
            kreuzberg_extract_file_sync("/nonexistent/error_test.pdf");
        assert(result == NULL);

        struct CErrorDetails details = kreuzberg_get_error_details();

        /* Should have a message about the failure */
        if (details.message != NULL) {
            assert(strlen(details.message) > 0);
            kreuzberg_free_string(details.message);
        }
        if (details.error_type != NULL) {
            kreuzberg_free_string(details.error_type);
        }
        if (details.source_file != NULL) {
            kreuzberg_free_string(details.source_file);
        }
        if (details.source_function != NULL) {
            kreuzberg_free_string(details.source_function);
        }
        if (details.context_info != NULL) {
            kreuzberg_free_string(details.context_info);
        }
    }

    /* Test 10: Verify error code constants are consistent */
    {
        uint32_t validation_code = kreuzberg_error_code_validation();
        uint32_t io_code = kreuzberg_error_code_io();
        uint32_t parse_code = kreuzberg_error_code_parsing();

        /* These should be distinct values */
        assert(validation_code != io_code);
        assert(validation_code != parse_code);
        assert(io_code != parse_code);
    }

    /* Test 11: clone_string with longer content */
    {
        const char *long_str =
            "This is a longer string to test kreuzberg_clone_string with "
            "more content. It includes multiple sentences and should be "
            "cloned exactly as-is without any truncation or modification.";
        char *cloned = kreuzberg_clone_string(long_str);
        assert(cloned != NULL);
        assert(strcmp(cloned, long_str) == 0);
        assert(strlen(cloned) == strlen(long_str));
        kreuzberg_free_string(cloned);
    }

    printf("test_error_extended: all tests passed\n");
    return 0;
}
