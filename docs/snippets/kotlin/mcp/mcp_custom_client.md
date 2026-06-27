```kotlin title="Kotlin"
import io.xberg.*
import java.util.Optional
import java.io.BufferedReader
import java.io.BufferedWriter
import java.io.InputStreamReader
import java.io.OutputStreamWriter

fun main() {
    val process = ProcessBuilder("xberg", "mcp")
        .redirectErrorStream(true)
        .start()

    val stdin = BufferedWriter(OutputStreamWriter(process.outputStream))
    val stdout = BufferedReader(InputStreamReader(process.inputStream))

    val request = """
        {"method":"tools/call","params":{"name":"extract","arguments":{"path":"document.pdf","async":true}}}
    """.trimIndent()

    stdin.write(request)
    stdin.newLine()
    stdin.flush()

    val response = stdout.readLine()
    println(response)

    stdin.close()
    stdout.close()
    process.destroy()
}
```
