#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/*
 * Plugin error injection tests.
 *
 * Registers callbacks that return error strings and verifies that
 * the error propagates correctly through the FFI layer.
 */

/* ---- Document extractor that always returns an error string ---- */
static char *failing_doc_extractor(const uint8_t *content, uintptr_t content_len,
                                   const char *mime_type, const char *config_json) {
    (void)content;
    (void)content_len;
    (void)mime_type;
    (void)config_json;
    /* Return a malloc'd error string -- simulates extraction failure */
    char *err = (char *)malloc(64);
    if (!err)
        return NULL;
    snprintf(err, 64, "extraction failed: test error");
    return err;
}

/* ---- OCR backend that always returns an error string ---- */
static char *failing_ocr_backend(const uint8_t *image_bytes, uintptr_t image_length,
                                 const char *config_json) {
    (void)image_bytes;
    (void)image_length;
    (void)config_json;
    char *err = (char *)malloc(64);
    if (!err)
        return NULL;
    snprintf(err, 64, "ocr failed: test error");
    return err;
}

/* ---- Post-processor that always returns an error string ---- */
static char *failing_post_processor(const char *result_json) {
    (void)result_json;
    char *err = (char *)malloc(64);
    if (!err)
        return NULL;
    snprintf(err, 64, "post-processing failed: test error");
    return err;
}

/* ---- Validator that always returns a validation error string ---- */
static char *failing_validator(const char *result_json) {
    (void)result_json;
    char *err = (char *)malloc(64);
    if (!err)
        return NULL;
    snprintf(err, 64, "validation failed: test error");
    return err;
}

int main(void) {
    /* ================================================================
     * FAILING DOCUMENT EXTRACTOR
     * ================================================================ */
    printf("  testing failing document extractor registration...\n");
    {
        bool ok = kreuzberg_clear_document_extractors();
        assert(ok);

        /* Register a failing extractor */
        ok = kreuzberg_register_document_extractor("fail-extractor", failing_doc_extractor,
                                                   "application/x-fail", 100);
        assert(ok);

        /* Verify it's listed */
        char *list = kreuzberg_list_document_extractors();
        assert(list != NULL);
        assert(strstr(list, "fail-extractor") != NULL);
        kreuzberg_free_string(list);

        /* Unregister and verify cleanup */
        ok = kreuzberg_unregister_document_extractor("fail-extractor");
        assert(ok);
        ok = kreuzberg_clear_document_extractors();
        assert(ok);
    }

    /* ================================================================
     * FAILING OCR BACKEND
     * ================================================================ */
    printf("  testing failing OCR backend registration...\n");
    {
        bool ok = kreuzberg_clear_ocr_backends();
        assert(ok);

        /* Register a failing OCR backend */
        ok = kreuzberg_register_ocr_backend("fail-ocr", failing_ocr_backend);
        assert(ok);

        /* Verify listed */
        char *list = kreuzberg_list_ocr_backends();
        assert(list != NULL);
        assert(strstr(list, "fail-ocr") != NULL);
        kreuzberg_free_string(list);

        /* Unregister and verify */
        ok = kreuzberg_unregister_ocr_backend("fail-ocr");
        assert(ok);
        ok = kreuzberg_clear_ocr_backends();
        assert(ok);
    }

    /* ================================================================
     * FAILING POST-PROCESSOR
     * ================================================================ */
    printf("  testing failing post-processor registration...\n");
    {
        bool ok = kreuzberg_clear_post_processors();
        assert(ok);

        ok = kreuzberg_register_post_processor("fail-processor", failing_post_processor, 50);
        assert(ok);

        char *list = kreuzberg_list_post_processors();
        assert(list != NULL);
        assert(strstr(list, "fail-processor") != NULL);
        kreuzberg_free_string(list);

        ok = kreuzberg_unregister_post_processor("fail-processor");
        assert(ok);
        ok = kreuzberg_clear_post_processors();
        assert(ok);
    }

    /* ================================================================
     * FAILING VALIDATOR
     * ================================================================ */
    printf("  testing failing validator registration...\n");
    {
        bool ok = kreuzberg_clear_validators();
        assert(ok);

        ok = kreuzberg_register_validator("fail-validator", failing_validator, 50);
        assert(ok);

        char *list = kreuzberg_list_validators();
        assert(list != NULL);
        assert(strstr(list, "fail-validator") != NULL);
        kreuzberg_free_string(list);

        ok = kreuzberg_unregister_validator("fail-validator");
        assert(ok);
        ok = kreuzberg_clear_validators();
        assert(ok);
    }

    /* ================================================================
     * NULL CALLBACK REGISTRATION
     * ================================================================ */
    printf("  testing NULL callback registration...\n");
    {
        /* Registering with NULL name should fail gracefully */
        bool ok = kreuzberg_register_document_extractor(NULL, failing_doc_extractor,
                                                        "application/x-fail", 100);
        assert(!ok);

        /* Registering with NULL callback should fail gracefully */
        ok = kreuzberg_register_document_extractor("null-cb", NULL, "application/x-fail", 100);
        assert(!ok);

        /* Registering OCR backend with NULL name should fail */
        ok = kreuzberg_register_ocr_backend(NULL, failing_ocr_backend);
        assert(!ok);

        /* Registering OCR backend with NULL callback should fail */
        ok = kreuzberg_register_ocr_backend("null-cb-ocr", NULL);
        assert(!ok);

        /* Registering validator with NULL name should fail */
        ok = kreuzberg_register_validator(NULL, failing_validator, 50);
        assert(!ok);

        /* Registering validator with NULL callback should fail */
        ok = kreuzberg_register_validator("null-cb-val", NULL, 50);
        assert(!ok);

        /* Registering post-processor with NULL name should fail */
        ok = kreuzberg_register_post_processor(NULL, failing_post_processor, 50);
        assert(!ok);

        /* Registering post-processor with NULL callback should fail */
        ok = kreuzberg_register_post_processor("null-cb-pp", NULL, 50);
        assert(!ok);
    }

    /* ================================================================
     * MULTIPLE REGISTRATIONS WITH SAME NAME
     * ================================================================ */
    printf("  testing duplicate name registration...\n");
    {
        bool ok = kreuzberg_clear_document_extractors();
        assert(ok);

        ok = kreuzberg_register_document_extractor("dup-name", failing_doc_extractor,
                                                   "application/x-dup", 100);
        assert(ok);

        /* Re-registering the same name should succeed (overwrites) */
        ok = kreuzberg_register_document_extractor("dup-name", failing_doc_extractor,
                                                   "application/x-dup2", 200);
        assert(ok);

        /* Only one entry should exist with this name */
        char *list = kreuzberg_list_document_extractors();
        assert(list != NULL);
        assert(strstr(list, "dup-name") != NULL);
        kreuzberg_free_string(list);

        ok = kreuzberg_clear_document_extractors();
        assert(ok);
    }

    printf("test_plugins_errors: all tests passed\n");
    return 0;
}
