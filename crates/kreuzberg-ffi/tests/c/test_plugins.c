#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ---- Stub callbacks for each plugin type ---- */

/*
 * DocumentExtractorCallback:
 *   char *(*)(const uint8_t *content, uintptr_t content_len,
 *             const char *mime_type, const char *config_json)
 */
static char *my_doc_extractor(const uint8_t *content, uintptr_t content_len, const char *mime_type,
                              const char *config_json) {
    (void)content;
    (void)content_len;
    (void)mime_type;
    (void)config_json;
    return NULL; /* Stub: returns NULL (no result) */
}

/*
 * OcrBackendCallback:
 *   char *(*)(const uint8_t *image_bytes, uintptr_t image_length,
 *             const char *config_json)
 */
static char *my_ocr_backend(const uint8_t *image_bytes, uintptr_t image_length,
                            const char *config_json) {
    (void)image_bytes;
    (void)image_length;
    (void)config_json;
    return NULL; /* Stub */
}

/*
 * PostProcessorCallback:
 *   char *(*)(const char *result_json)
 */
static char *my_post_processor(const char *result_json) {
    (void)result_json;
    return NULL; /* Stub */
}

/*
 * ValidatorCallback:
 *   char *(*)(const char *result_json)
 *   Returns NULL if validation passes, error string otherwise.
 */
static char *my_validator(const char *result_json) {
    (void)result_json;
    return NULL; /* Stub: validation passes */
}

/* ---- Helper: check if a JSON array string contains a given name ---- */
static int json_list_contains(const char *json, const char *name) {
    if (json == NULL || name == NULL)
        return 0;
    return strstr(json, name) != NULL;
}

int main(void) {
    /* ================================================================
     * DOCUMENT EXTRACTORS
     * ================================================================ */
    printf("  testing document extractors...\n");
    {
        /* Clear any pre-existing registrations */
        bool ok = kreuzberg_clear_document_extractors();
        assert(ok);

        /* List should be empty (or a JSON array with no custom entries) */
        char *list = kreuzberg_list_document_extractors();
        assert(list != NULL);
        assert(!json_list_contains(list, "test-doc-extractor"));
        kreuzberg_free_string(list);

        /* Register a document extractor */
        ok = kreuzberg_register_document_extractor("test-doc-extractor", my_doc_extractor,
                                                   "application/x-test", 100);
        assert(ok);

        /* List again: should contain our registration */
        list = kreuzberg_list_document_extractors();
        assert(list != NULL);
        assert(json_list_contains(list, "test-doc-extractor"));
        kreuzberg_free_string(list);

        /* Unregister */
        ok = kreuzberg_unregister_document_extractor("test-doc-extractor");
        assert(ok);

        /* Verify removed */
        list = kreuzberg_list_document_extractors();
        assert(list != NULL);
        assert(!json_list_contains(list, "test-doc-extractor"));
        kreuzberg_free_string(list);

        /* Unregistering a non-existent name returns true (idempotent no-op) */
        ok = kreuzberg_unregister_document_extractor("nonexistent-extractor");
        assert(ok);

        /* Clear all (no-op since already empty, but should succeed) */
        ok = kreuzberg_clear_document_extractors();
        assert(ok);
    }

    /* ================================================================
     * OCR BACKENDS
     * ================================================================ */
    printf("  testing OCR backends...\n");
    {
        /* Clear any pre-existing */
        bool ok = kreuzberg_clear_ocr_backends();
        assert(ok);

        /* List should not contain our backend */
        char *list = kreuzberg_list_ocr_backends();
        assert(list != NULL);
        assert(!json_list_contains(list, "test-ocr"));
        kreuzberg_free_string(list);

        /* Register an OCR backend */
        ok = kreuzberg_register_ocr_backend("test-ocr", my_ocr_backend);
        assert(ok);

        /* List again: should contain our backend */
        list = kreuzberg_list_ocr_backends();
        assert(list != NULL);
        assert(json_list_contains(list, "test-ocr"));
        kreuzberg_free_string(list);

        /* Unregister */
        ok = kreuzberg_unregister_ocr_backend("test-ocr");
        assert(ok);

        /* Verify removed */
        list = kreuzberg_list_ocr_backends();
        assert(list != NULL);
        assert(!json_list_contains(list, "test-ocr"));
        kreuzberg_free_string(list);

        /* Register with languages */
        ok = kreuzberg_register_ocr_backend_with_languages("test-ocr-lang", my_ocr_backend,
                                                           "[\"en\", \"de\", \"fr\"]");
        assert(ok);

        /* Verify it was registered */
        list = kreuzberg_list_ocr_backends();
        assert(list != NULL);
        assert(json_list_contains(list, "test-ocr-lang"));
        kreuzberg_free_string(list);

        /* Test get_ocr_languages for our backend.
         * Note: kreuzberg_get_ocr_languages uses LanguageRegistry::global()
         * which may or may not be populated by register_ocr_backend_with_languages.
         */
        char *languages = kreuzberg_get_ocr_languages("test-ocr-lang");
        if (languages != NULL) {
            /* If languages are returned, verify they are a JSON value */
            assert(strlen(languages) > 0);
            kreuzberg_free_string(languages);
        }

        /*
         * kreuzberg_is_language_supported queries the global LanguageRegistry,
         * which may be independent of the OCR backend registry. Just verify
         * the function doesn't crash and returns a valid boolean (0 or 1).
         */
        int32_t supported = kreuzberg_is_language_supported("test-ocr-lang", "en");
        assert(supported == 0 || supported == 1);

        supported = kreuzberg_is_language_supported("test-ocr-lang", "zh");
        assert(supported == 0 || supported == 1);

        /* Test is_language_supported with NULL arguments */
        supported = kreuzberg_is_language_supported(NULL, "en");
        assert(supported == 0);

        supported = kreuzberg_is_language_supported("test-ocr-lang", NULL);
        assert(supported == 0);

        /* Test list with languages */
        char *backends_with_langs = kreuzberg_list_ocr_backends_with_languages();
        if (backends_with_langs != NULL) {
            /* Should return valid JSON */
            assert(strlen(backends_with_langs) > 0);
            kreuzberg_free_string(backends_with_langs);
        }

        /* Clear all OCR backends */
        ok = kreuzberg_clear_ocr_backends();
        assert(ok);

        /* Verify cleared */
        list = kreuzberg_list_ocr_backends();
        assert(list != NULL);
        assert(!json_list_contains(list, "test-ocr-lang"));
        kreuzberg_free_string(list);
    }

    /* ================================================================
     * POST-PROCESSORS
     * ================================================================ */
    printf("  testing post-processors...\n");
    {
        /* Clear any pre-existing */
        bool ok = kreuzberg_clear_post_processors();
        assert(ok);

        /* List should not contain our processor */
        char *list = kreuzberg_list_post_processors();
        assert(list != NULL);
        assert(!json_list_contains(list, "test-processor"));
        kreuzberg_free_string(list);

        /* Register a post-processor with priority */
        ok = kreuzberg_register_post_processor("test-processor", my_post_processor, 100);
        assert(ok);

        /* List again: should contain our processor */
        list = kreuzberg_list_post_processors();
        assert(list != NULL);
        assert(json_list_contains(list, "test-processor"));
        kreuzberg_free_string(list);

        /* Unregister */
        ok = kreuzberg_unregister_post_processor("test-processor");
        assert(ok);

        /* Verify removed */
        list = kreuzberg_list_post_processors();
        assert(list != NULL);
        assert(!json_list_contains(list, "test-processor"));
        kreuzberg_free_string(list);

        /* Register with stage (valid stages: "early", "middle", "late") */
        ok = kreuzberg_register_post_processor_with_stage("test-stage-processor", my_post_processor,
                                                          50, "early");
        assert(ok);

        /* Verify registered */
        list = kreuzberg_list_post_processors();
        assert(list != NULL);
        assert(json_list_contains(list, "test-stage-processor"));
        kreuzberg_free_string(list);

        /* Clear all post-processors */
        ok = kreuzberg_clear_post_processors();
        assert(ok);

        /* Verify cleared */
        list = kreuzberg_list_post_processors();
        assert(list != NULL);
        assert(!json_list_contains(list, "test-stage-processor"));
        kreuzberg_free_string(list);
    }

    /* ================================================================
     * VALIDATORS
     * ================================================================ */
    printf("  testing validators...\n");
    {
        /* Clear any pre-existing */
        bool ok = kreuzberg_clear_validators();
        assert(ok);

        /* List should not contain our validator */
        char *list = kreuzberg_list_validators();
        assert(list != NULL);
        assert(!json_list_contains(list, "test-validator"));
        kreuzberg_free_string(list);

        /* Register a validator with priority */
        ok = kreuzberg_register_validator("test-validator", my_validator, 100);
        assert(ok);

        /* List again: should contain our validator */
        list = kreuzberg_list_validators();
        assert(list != NULL);
        assert(json_list_contains(list, "test-validator"));
        kreuzberg_free_string(list);

        /* Unregister */
        ok = kreuzberg_unregister_validator("test-validator");
        assert(ok);

        /* Verify removed */
        list = kreuzberg_list_validators();
        assert(list != NULL);
        assert(!json_list_contains(list, "test-validator"));
        kreuzberg_free_string(list);

        /* Unregistering a non-existent name returns true (idempotent no-op) */
        ok = kreuzberg_unregister_validator("nonexistent-validator");
        assert(ok);

        /* Clear all validators */
        ok = kreuzberg_clear_validators();
        assert(ok);
    }

    printf("test_plugins: all tests passed\n");
    return 0;
}
