package dev.kreuzberg;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;

/**
 * Helper class for validating configuration parameters through FFI.
 *
 * This class provides Java-idiomatic wrappers around the Kreuzberg FFI validation
 * functions. It handles the low-level FFM API details and throws KreuzbergException
 * for validation failures.
 *
 * <p><strong>Internal API:</strong> This class is not intended for direct use.
 * Configuration builders use this internally for parameter validation.</p>
 *
 * @since 4.0.0
 */
public final class ValidationHelper {
    private ValidationHelper() {
    }

    /**
     * Validates a binarization method string.
     *
     * @param method the binarization method (e.g., "otsu", "adaptive", "sauvola")
     * @throws KreuzbergException if the method is invalid
     */
    public static void validateBinarizationMethod(String method)
        throws KreuzbergException {
        if (method == null) {
            throw new KreuzbergException("Binarization method cannot be null");
        }
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment methodSegment = KreuzbergFFI.allocateCString(arena, method);
            checkValidationResult(
                KreuzbergFFI.KREUZBERG_VALIDATE_BINARIZATION_METHOD,
                new Object[]{methodSegment},
                "Invalid binarization method: " + method
            );
        } catch (Exception e) {
            throw handleValidationException(e, "Failed to validate binarization method");
        }
    }

    /**
     * Validates an OCR backend string.
     *
     * @param backend the OCR backend (e.g., "tesseract", "easyocr", "paddleocr")
     * @throws KreuzbergException if the backend is invalid
     */
    public static void validateOcrBackend(String backend)
        throws KreuzbergException {
        if (backend == null) {
            throw new KreuzbergException("OCR backend cannot be null");
        }
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment backendSegment = KreuzbergFFI.allocateCString(arena, backend);
            checkValidationResult(
                KreuzbergFFI.KREUZBERG_VALIDATE_OCR_BACKEND,
                new Object[]{backendSegment},
                "Invalid OCR backend: " + backend
            );
        } catch (Exception e) {
            throw handleValidationException(e, "Failed to validate OCR backend");
        }
    }

    /**
     * Validates a language code (ISO 639-1 or 639-3 format).
     *
     * Accepts both 2-letter codes (e.g., "en", "de") and 3-letter codes
     * (e.g., "eng", "deu").
     *
     * @param code the language code
     * @throws KreuzbergException if the code is invalid
     */
    public static void validateLanguageCode(String code)
        throws KreuzbergException {
        if (code == null) {
            throw new KreuzbergException("Language code cannot be null");
        }
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment codeSegment = KreuzbergFFI.allocateCString(arena, code);
            checkValidationResult(
                KreuzbergFFI.KREUZBERG_VALIDATE_LANGUAGE_CODE,
                new Object[]{codeSegment},
                "Invalid language code: " + code
            );
        } catch (Exception e) {
            throw handleValidationException(e, "Failed to validate language code");
        }
    }

    /**
     * Validates a token reduction level string.
     *
     * @param level the token reduction level (e.g., "off", "light", "moderate")
     * @throws KreuzbergException if the level is invalid
     */
    public static void validateTokenReductionLevel(String level)
        throws KreuzbergException {
        if (level == null) {
            throw new KreuzbergException("Token reduction level cannot be null");
        }
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment levelSegment = KreuzbergFFI.allocateCString(arena, level);
            checkValidationResult(
                KreuzbergFFI.KREUZBERG_VALIDATE_TOKEN_REDUCTION_LEVEL,
                new Object[]{levelSegment},
                "Invalid token reduction level: " + level
            );
        } catch (Exception e) {
            throw handleValidationException(e, "Failed to validate token reduction level");
        }
    }

    /**
     * Validates a Tesseract Page Segmentation Mode (PSM) value.
     *
     * @param psm the PSM value (valid range: 0-13)
     * @throws KreuzbergException if the PSM is invalid
     */
    public static void validateTesseractPsm(int psm)
        throws KreuzbergException {
        try {
            checkValidationResult(
                KreuzbergFFI.KREUZBERG_VALIDATE_TESSERACT_PSM,
                new Object[]{psm},
                "Invalid Tesseract PSM: " + psm
            );
        } catch (Exception e) {
            throw handleValidationException(e, "Failed to validate Tesseract PSM");
        }
    }

    /**
     * Validates a Tesseract OCR Engine Mode (OEM) value.
     *
     * @param oem the OEM value (valid range: 0-3)
     * @throws KreuzbergException if the OEM is invalid
     */
    public static void validateTesseractOem(int oem)
        throws KreuzbergException {
        try {
            checkValidationResult(
                KreuzbergFFI.KREUZBERG_VALIDATE_TESSERACT_OEM,
                new Object[]{oem},
                "Invalid Tesseract OEM: " + oem
            );
        } catch (Exception e) {
            throw handleValidationException(e, "Failed to validate Tesseract OEM");
        }
    }

    /**
     * Validates an output format string.
     *
     * @param format the output format (e.g., "text", "markdown")
     * @throws KreuzbergException if the format is invalid
     */
    public static void validateOutputFormat(String format)
        throws KreuzbergException {
        if (format == null) {
            throw new KreuzbergException("Output format cannot be null");
        }
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment formatSegment = KreuzbergFFI.allocateCString(arena, format);
            checkValidationResult(
                KreuzbergFFI.KREUZBERG_VALIDATE_OUTPUT_FORMAT,
                new Object[]{formatSegment},
                "Invalid output format: " + format
            );
        } catch (Exception e) {
            throw handleValidationException(e, "Failed to validate output format");
        }
    }

    /**
     * Validates a confidence threshold value.
     *
     * Confidence thresholds must be between 0.0 and 1.0 inclusive.
     *
     * @param confidence the confidence threshold value
     * @throws KreuzbergException if the confidence is invalid
     */
    public static void validateConfidence(double confidence)
        throws KreuzbergException {
        try {
            checkValidationResult(
                KreuzbergFFI.KREUZBERG_VALIDATE_CONFIDENCE,
                new Object[]{confidence},
                "Invalid confidence: " + confidence
            );
        } catch (Exception e) {
            throw handleValidationException(e, "Failed to validate confidence");
        }
    }

    /**
     * Validates a DPI (dots per inch) value.
     *
     * DPI must be a positive integer, typically 72-600.
     *
     * @param dpi the DPI value
     * @throws KreuzbergException if the DPI is invalid
     */
    public static void validateDpi(int dpi) throws KreuzbergException {
        try {
            checkValidationResult(
                KreuzbergFFI.KREUZBERG_VALIDATE_DPI,
                new Object[]{dpi},
                "Invalid DPI: " + dpi
            );
        } catch (Exception e) {
            throw handleValidationException(e, "Failed to validate DPI");
        }
    }

    /**
     * Validates chunking parameters.
     *
     * Checks that max_chars > 0 and max_overlap < max_chars.
     *
     * @param maxChars the maximum characters per chunk
     * @param maxOverlap the maximum overlap between chunks
     * @throws KreuzbergException if the parameters are invalid
     */
    public static void validateChunkingParams(long maxChars, long maxOverlap)
        throws KreuzbergException {
        try {
            checkValidationResult(
                KreuzbergFFI.KREUZBERG_VALIDATE_CHUNKING_PARAMS,
                new Object[]{maxChars, maxOverlap},
                "Invalid chunking parameters: maxChars=" + maxChars
                    + ", maxOverlap=" + maxOverlap
            );
        } catch (Exception e) {
            throw handleValidationException(e, "Failed to validate chunking parameters");
        }
    }

    /**
     * Checks the validation result and throws KreuzbergException if invalid.
     *
     * @param methodHandle the MethodHandle to invoke
     * @param args the arguments to pass to the method handle
     * @param fallbackMsg the error message if Rust error is unavailable
     * @throws Exception if FFI invocation fails
     * @throws KreuzbergException if validation fails
     */
    @SuppressWarnings("PMD.AvoidCatchingThrowable")
    private static void checkValidationResult(
        java.lang.invoke.MethodHandle methodHandle,
        Object[] args,
        String fallbackMsg) throws Exception {
        try {
            int result = (int) methodHandle.invokeWithArguments(args);
            if (result == 0) {
                String errorMsg = getLastError();
                String msg = errorMsg != null ? errorMsg : fallbackMsg;
                throw new KreuzbergException(msg);
            }
        } catch (KreuzbergException e) {
            throw e;
        } catch (Throwable e) {
            throw new Exception(e);
        }
    }

    /**
     * Handles exceptions from validation, preserving stack traces.
     *
     * @param e the exception caught
     * @param context the context of what was being validated
     * @return a KreuzbergException with the original exception as cause
     */
    private static KreuzbergException handleValidationException(
        Exception e,
        String context) {
        if (e instanceof KreuzbergException) {
            return (KreuzbergException) e;
        }
        return new KreuzbergException(context, e);
    }

    /**
     * Gets the last error message from the FFI layer.
     *
     * @return the error message, or null if no error
     */
    @SuppressWarnings("PMD.AvoidCatchingThrowable")
    private static String getLastError() {
        try {
            MemorySegment errorSegment = (MemorySegment) KreuzbergFFI
                .KREUZBERG_LAST_ERROR.invoke();
            return KreuzbergFFI.readCString(errorSegment);
        } catch (Throwable e) {
            return null;
        }
    }
}
