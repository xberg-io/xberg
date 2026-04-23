package dev.kreuzberg;

import java.util.List;
import java.util.Map;

/**
 * Bridge interface for the OcrBackend plugin system.
 *
 * Implementations are wrapped by OcrBackendBridge and exposed to the native
 * runtime through Panama FFM upcall stubs.
 */
public interface IOcrBackend {

    /** Plugin name (used for registry keying). */
    String name();

    /** Plugin version. */
    String version();

    /** Initialize the plugin. */
    default void initialize() throws Exception {}

    /** Shut down the plugin. */
    default void shutdown() throws Exception {}

    /** process_image. */
    ExtractionResult process_image(byte[] image_bytes, OcrConfig config) throws Exception;

    /** process_image_file. */
    ExtractionResult process_image_file(java.nio.file.Path path, OcrConfig config) throws Exception;

    /** supports_language. */
    boolean supports_language(String lang) throws Exception;

    /** backend_type. */
    OcrBackendType backend_type() throws Exception;

    /** supported_languages. */
    List<String> supported_languages() throws Exception;

    /** supports_table_detection. */
    boolean supports_table_detection() throws Exception;

    /** supports_document_processing. */
    boolean supports_document_processing() throws Exception;

    /** process_document. */
    ExtractionResult process_document(java.nio.file.Path _path, OcrConfig _config) throws Exception;

}
