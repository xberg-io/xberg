```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

class StatefulPlugin : IPostProcessor {
    private val callCount = AtomicLong(0)
    private val cache: ConcurrentHashMap<String, String> = ConcurrentHashMap()

    override fun name(): String = "stateful-plugin"
    override fun version(): String = "1.0.0"

    override fun initialize() {
        callCount.set(0)
        cache.clear()
    }

    override fun shutdown() {
        println("Plugin called ${callCount.get()} times")
        cache.clear()
    }

    override fun process(result: ExtractionResult, config: ExtractionConfig) {
        val count = callCount.incrementAndGet()
        cache["last_mime"] = result.mimeType()
        cache["last_call"] = count.toString()
    }

    override fun processing_stage(): ProcessingStage = ProcessingStage.Middle

    override fun should_process(
        _result: ExtractionResult,
        _config: ExtractionConfig,
    ): Boolean = true

    override fun estimated_duration_ms(_result: ExtractionResult): Long = 1L

    override fun priority(): Int = 50

    fun callCount(): Long = callCount.get()
    fun lastMime(): String? = cache["last_mime"]
}

fun registerStatefulPlugin() {
    PostProcessorBridge.registerPostProcessor(StatefulPlugin())
}
```
