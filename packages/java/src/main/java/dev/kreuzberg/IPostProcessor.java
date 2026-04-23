package dev.kreuzberg;

import java.util.List;
import java.util.Map;

/**
 * Bridge interface for the PostProcessor plugin system.
 *
 * Implementations are wrapped by PostProcessorBridge and exposed to the native
 * runtime through Panama FFM upcall stubs.
 */
public interface IPostProcessor {

    /** Plugin name (used for registry keying). */
    String name();

    /** Plugin version. */
    String version();

    /** Initialize the plugin. */
    default void initialize() throws Exception {}

    /** Shut down the plugin. */
    default void shutdown() throws Exception {}

    /** process. */
    void process(ExtractionResult result, ExtractionConfig config) throws Exception;

    /** processing_stage. */
    ProcessingStage processing_stage() throws Exception;

    /** should_process. */
    boolean should_process(ExtractionResult _result, ExtractionConfig _config) throws Exception;

    /** estimated_duration_ms. */
    long estimated_duration_ms(ExtractionResult _result) throws Exception;

}
