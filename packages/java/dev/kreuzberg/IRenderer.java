package dev.kreuzberg;

/**
 * Bridge interface for the Renderer plugin system.
 *
 * Implementations are wrapped by RendererBridge and exposed to the native
 * runtime through Panama FFM upcall stubs.
 */
public interface IRenderer {

    /** Plugin name (used for registry keying). */
    String name();

    /** Plugin version. */
    String version();

    /** Initialize the plugin. */
    default void initialize() throws Exception {}

    /** Shut down the plugin. */
    default void shutdown() throws Exception {}

/** render. */    String render(String doc) throws Exception;
}
