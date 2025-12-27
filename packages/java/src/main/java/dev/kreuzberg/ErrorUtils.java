package dev.kreuzberg;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;
import java.util.HashMap;
import java.util.Map;

/**
 * Utility functions for error handling and classification via FFI.
 *
 * <p>Provides access to error details, error code classification, and error code metadata
 * from the Kreuzberg native library. Phase 2 FFI integration for error handling.</p>
 *
 * @since 4.0.0
 */
@SuppressWarnings("PMD.AvoidCatchingThrowable")
public final class ErrorUtils {

    private ErrorUtils() {
    }

    /**
     * Classify an error message and return the corresponding error code.
     *
     * <p>Analyzes the error message string and returns the numeric error code (0-7)
     * that best represents the error type.</p>
     *
     * <p>Error codes:</p>
     * <ul>
     *   <li>0 - Validation error (invalid parameters, constraints)</li>
     *   <li>1 - Parsing error (corrupt data, malformed content)</li>
     *   <li>2 - OCR error (OCR processing failures)</li>
     *   <li>3 - MissingDependency error (missing libraries)</li>
     *   <li>4 - IO error (file I/O, permissions)</li>
     *   <li>5 - Plugin error (plugin registration/execution)</li>
     *   <li>6 - UnsupportedFormat error (unsupported MIME types)</li>
     *   <li>7 - Internal error (internal library errors)</li>
     * </ul>
     *
     * @param message the error message to classify
     * @return error code (0-7), or 7 if classification fails
     * @throws IllegalArgumentException if message is null
     * @throws KreuzbergException if FFI call fails
     * @since 4.0.0
     */
    public static int classifyError(String message) throws KreuzbergException {
        if (message == null) {
            throw new IllegalArgumentException("message must not be null");
        }

        try (Arena arena = Arena.ofConfined()) {
            MemorySegment messageSeg = KreuzbergFFI.allocateCString(arena, message);

            try {
                return (int) KreuzbergFFI.KREUZBERG_CLASSIFY_ERROR.invoke(messageSeg);
            } catch (Throwable e) {
                throw new KreuzbergException("Failed to classify error", e);
            }
        }
    }

    /**
     * Get the human-readable name of an error code.
     *
     * <p>Returns the symbolic name (e.g., "validation", "ocr") for the given error code.</p>
     *
     * @param code the error code (0-7)
     * @return human-readable name, or "unknown" if code is invalid
     * @throws KreuzbergException if FFI call fails
     * @since 4.0.0
     */
    public static String getErrorCodeName(int code) throws KreuzbergException {
        try {
            MemorySegment namePtr = (MemorySegment) KreuzbergFFI.KREUZBERG_ERROR_CODE_NAME.invoke((long) code);

            if (namePtr == null || namePtr.address() == 0) {
                return "unknown";
            }

            return KreuzbergFFI.readCString(namePtr);
        } catch (Throwable e) {
            throw new KreuzbergException("Failed to get error code name for code: " + code, e);
        }
    }

    /**
     * Get the human-readable description of an error code.
     *
     * <p>Returns a brief description suitable for user-facing error messages.</p>
     *
     * @param code the error code (0-7)
     * @return human-readable description
     * @throws KreuzbergException if FFI call fails
     * @since 4.0.0
     */
    public static String getErrorCodeDescription(int code) throws KreuzbergException {
        try {
            MemorySegment descPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_ERROR_CODE_DESCRIPTION.invoke((long) code);

            if (descPtr == null || descPtr.address() == 0) {
                return "Unknown error";
            }

            return KreuzbergFFI.readCString(descPtr);
        } catch (Throwable e) {
            throw new KreuzbergException("Failed to get error code description for code: " + code, e);
        }
    }

    /**
     * Get structured error details from the FFI layer.
     *
     * <p>Returns a Map containing detailed error information including:</p>
     * <ul>
     *   <li>"message" (String) - Error message</li>
     *   <li>"error_code" (Integer) - Error code (0-7)</li>
     *   <li>"error_type" (String) - Error type name</li>
     *   <li>"source_file" (String) - Source file path if available</li>
     *   <li>"source_function" (String) - Function name if available</li>
     *   <li>"source_line" (Integer) - Line number if available</li>
     *   <li>"context_info" (String) - Additional context if available</li>
     *   <li>"is_panic" (Boolean) - Whether error came from a panic</li>
     * </ul>
     *
     * @return Map with error details, or empty map if no error details available
     * @throws KreuzbergException if FFI call fails
     * @since 4.0.0
     */
    public static Map<String, Object> getErrorDetails() throws KreuzbergException {
        Map<String, Object> result = new HashMap<>();

        try {
            MemorySegment detailsPtr = (MemorySegment) KreuzbergFFI.KREUZBERG_GET_ERROR_DETAILS.invoke();

            if (detailsPtr == null || detailsPtr.address() == 0) {
                return result;
            }

            String errorMsg = KreuzbergFFI.readCString(detailsPtr);
            if (errorMsg != null) {
                result.put("message", errorMsg);
            }

            return result;
        } catch (Throwable e) {
            throw new KreuzbergException("Failed to get error details", e);
        }
    }

    /**
     * Map error code integer to ErrorCode enum.
     *
     * @param code the error code (0-7)
     * @return the ErrorCode enum value
     * @since 4.0.0
     */
    public static ErrorCode mapErrorCode(int code) {
        return ErrorCode.fromCode(code);
    }
}
