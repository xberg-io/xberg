```kotlin title="Kotlin"
import dev.kreuzberg.*
import java.util.Optional

fun main() {
    val process = ProcessBuilder("kreuzberg", "mcp")
        .inheritIO()
        .start()
    process.waitFor()
}
```
