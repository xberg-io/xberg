package dev.kreuzberg;

import java.util.List;

/**
 * Bridge interface for the DocumentExtractor plugin system.
 *
 * Implementations are wrapped by DocumentExtractorBridge and exposed to the native
 * runtime through Panama FFM upcall stubs.
 */
public interface IDocumentExtractor {

    /** Plugin name (used for registry keying). */
    String name();

    /** Plugin version. */
    String version();

    /** Initialize the plugin. */
    default void initialize() throws Exception {}

    /** Shut down the plugin. */
    default void shutdown() throws Exception {}

/** extract_bytes. */    String extract_bytes(byte[] content, String mime_type, ExtractionConfig config) throws Exception;

/** extract_file. */    String extract_file(java.nio.file.Path path, String mime_type, ExtractionConfig config) throws Exception;

/** supported_mime_types. */    List<String> supported_mime_types() throws Exception;

/** priority. */    int priority() throws Exception;

/** can_handle. */    boolean can_handle(java.nio.file.Path _path, String _mime_type) throws Exception;

/** as_sync_extractor. */    String as_sync_extractor() throws Exception;
}
