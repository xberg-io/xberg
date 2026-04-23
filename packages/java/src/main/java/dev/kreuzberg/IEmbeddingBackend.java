package dev.kreuzberg;

import java.util.List;
import java.util.Map;

/**
 * Bridge interface for the EmbeddingBackend plugin system.
 *
 * Implementations are wrapped by EmbeddingBackendBridge and exposed to the native
 * runtime through Panama FFM upcall stubs.
 */
public interface IEmbeddingBackend {

    /** Plugin name (used for registry keying). */
    String name();

    /** Plugin version. */
    String version();

    /** Initialize the plugin. */
    default void initialize() throws Exception {}

    /** Shut down the plugin. */
    default void shutdown() throws Exception {}

    /** dimensions. */
    long dimensions() throws Exception;

    /** embed. */
    List<List<Float>> embed(List<String> texts) throws Exception;

}
