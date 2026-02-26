#include "../../kreuzberg.h"
#include <assert.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /* kreuzberg_validate_binarization_method: 1=valid, 0=invalid */
    assert(kreuzberg_validate_binarization_method("otsu") == 1);
    assert(kreuzberg_validate_binarization_method("invalid_method") == 0);

    /* kreuzberg_validate_ocr_backend */
    assert(kreuzberg_validate_ocr_backend("tesseract") == 1);
    assert(kreuzberg_validate_ocr_backend("invalid_backend") == 0);

    /* kreuzberg_validate_language_code */
    assert(kreuzberg_validate_language_code("eng") == 1);
    assert(kreuzberg_validate_language_code("en") == 1);
    assert(kreuzberg_validate_language_code("xyz123") == 0);

    /* kreuzberg_validate_token_reduction_level */
    assert(kreuzberg_validate_token_reduction_level("off") == 1);
    assert(kreuzberg_validate_token_reduction_level("invalid_level") == 0);

    /* kreuzberg_validate_tesseract_psm: valid range 0-13 */
    assert(kreuzberg_validate_tesseract_psm(0) == 1);
    assert(kreuzberg_validate_tesseract_psm(6) == 1);
    assert(kreuzberg_validate_tesseract_psm(13) == 1);
    assert(kreuzberg_validate_tesseract_psm(-1) == 0);
    assert(kreuzberg_validate_tesseract_psm(14) == 0);
    assert(kreuzberg_validate_tesseract_psm(100) == 0);

    /* kreuzberg_validate_tesseract_oem: valid range 0-3 */
    assert(kreuzberg_validate_tesseract_oem(0) == 1);
    assert(kreuzberg_validate_tesseract_oem(3) == 1);
    assert(kreuzberg_validate_tesseract_oem(-1) == 0);
    assert(kreuzberg_validate_tesseract_oem(4) == 0);
    assert(kreuzberg_validate_tesseract_oem(100) == 0);

    /* kreuzberg_validate_output_format */
    assert(kreuzberg_validate_output_format("text") == 1);
    assert(kreuzberg_validate_output_format("markdown") == 1);
    assert(kreuzberg_validate_output_format("invalid_format") == 0);

    /* kreuzberg_validate_confidence: valid range 0.0 to 1.0 */
    assert(kreuzberg_validate_confidence(0.0) == 1);
    assert(kreuzberg_validate_confidence(0.5) == 1);
    assert(kreuzberg_validate_confidence(1.0) == 1);
    assert(kreuzberg_validate_confidence(-1.0) == 0);
    assert(kreuzberg_validate_confidence(2.0) == 0);

    /* kreuzberg_validate_dpi: must be positive */
    assert(kreuzberg_validate_dpi(300) == 1);
    assert(kreuzberg_validate_dpi(72) == 1);
    assert(kreuzberg_validate_dpi(0) == 0);
    assert(kreuzberg_validate_dpi(-1) == 0);

    /* kreuzberg_validate_chunking_params: max_chars > 0, max_overlap < max_chars */
    assert(kreuzberg_validate_chunking_params(1000, 200) == 1);
    assert(kreuzberg_validate_chunking_params(100, 0) == 1);
    assert(kreuzberg_validate_chunking_params(0, 0) == 0);
    assert(kreuzberg_validate_chunking_params(100, 100) == 0);
    assert(kreuzberg_validate_chunking_params(100, 200) == 0);

    /* kreuzberg_get_valid_binarization_methods: returns non-null JSON array */
    char *methods = kreuzberg_get_valid_binarization_methods();
    assert(methods != NULL);
    assert(methods[0] == '[');
    kreuzberg_free_string(methods);

    /* kreuzberg_get_valid_language_codes: returns non-null JSON array */
    char *codes = kreuzberg_get_valid_language_codes();
    assert(codes != NULL);
    assert(codes[0] == '[');
    kreuzberg_free_string(codes);

    /* kreuzberg_get_valid_ocr_backends: returns non-null JSON array */
    char *backends = kreuzberg_get_valid_ocr_backends();
    assert(backends != NULL);
    assert(backends[0] == '[');
    kreuzberg_free_string(backends);

    /* kreuzberg_get_valid_token_reduction_levels: returns non-null JSON array */
    char *levels = kreuzberg_get_valid_token_reduction_levels();
    assert(levels != NULL);
    assert(levels[0] == '[');
    kreuzberg_free_string(levels);

    printf("test_validation: all tests passed\n");
    return 0;
}
