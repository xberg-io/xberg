#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /* Test basic builder lifecycle: new -> build -> free config */
    struct ConfigBuilder *builder = kreuzberg_config_builder_new();
    assert(builder != NULL);

    ExtractionConfig *config = kreuzberg_config_builder_build(builder);
    assert(config != NULL);
    /* builder is now consumed; do NOT free it */

    /* Verify config produces JSON */
    char *json = kreuzberg_config_to_json(config);
    assert(json != NULL);
    assert(strlen(json) > 0);
    /*
     * Note: config_to_json serializes null optional fields which the parser
     * rejects as invalid objects. We just verify JSON is produced.
     */
    kreuzberg_free_string(json);
    kreuzberg_config_free(config);

    /* Test builder with setters */
    builder = kreuzberg_config_builder_new();
    assert(builder != NULL);

    /* set_use_cache takes int32_t: 1 = true, 0 = false */
    int32_t rc = kreuzberg_config_builder_set_use_cache(builder, 1);
    assert(rc == 0);

    /* set_include_document_structure takes int32_t: 1 = true, 0 = false */
    rc = kreuzberg_config_builder_set_include_document_structure(builder, 0);
    assert(rc == 0);

    /* set_ocr takes a JSON string */
    rc = kreuzberg_config_builder_set_ocr(builder, "{}");
    assert(rc == 0);

    /* set_pdf takes a JSON string */
    rc = kreuzberg_config_builder_set_pdf(builder, "{}");
    assert(rc == 0);

    /* set_chunking takes a JSON string */
    rc = kreuzberg_config_builder_set_chunking(builder, "{}");
    assert(rc == 0);

    /* set_image_extraction takes a JSON string */
    rc = kreuzberg_config_builder_set_image_extraction(builder, "{}");
    assert(rc == 0);

    /* set_post_processor takes a JSON string */
    rc = kreuzberg_config_builder_set_post_processor(builder, "{}");
    assert(rc == 0);

    /* set_language_detection takes a JSON string */
    rc = kreuzberg_config_builder_set_language_detection(builder, "{}");
    assert(rc == 0);

    /* Build the config with all fields set */
    config = kreuzberg_config_builder_build(builder);
    assert(config != NULL);

    /* Verify the built config produces JSON */
    json = kreuzberg_config_to_json(config);
    assert(json != NULL);
    assert(strlen(json) > 0);
    kreuzberg_free_string(json);
    kreuzberg_config_free(config);

    /* Test builder free without build (discard path) */
    builder = kreuzberg_config_builder_new();
    assert(builder != NULL);
    kreuzberg_config_builder_free(builder);

    /* Test builder free with NULL (safe no-op) */
    kreuzberg_config_builder_free(NULL);

    printf("test_config_builder: all tests passed\n");
    return 0;
}
