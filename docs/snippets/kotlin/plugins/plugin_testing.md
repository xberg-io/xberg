<!-- snippet:skip reason="kotlin.test is not on the snippet-runner classpath; the plugin-testing pattern documented here cannot compile under the runner's lightweight Kotlin profile. Run these tests from a real Gradle build." -->
```kotlin title="Kotlin"
import dev.kreuzberg.*
import dev.kreuzberg.kt.Kreuzberg
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class MinLengthValidatorTest {

    private fun makeResult(content: String): ExtractionResult =
        ExtractionResult.builder()
            .content(content)
            .mimeType("text/plain")
            .metadata(Metadata.builder().build())
            .tables(emptyList())
            .processingWarnings(emptyList())
            .build()

    @Test
    fun `validate accepts content above minimum length`() {
        val validator = MinLengthValidator(minLength = 5)
        val result = makeResult("hello world")
        validator.validate(result, ExtractionConfig.builder().build())
    }

    @Test
    fun `validate rejects content below minimum length`() {
        val validator = MinLengthValidator(minLength = 100)
        val result = makeResult("too short")
        assertFailsWith<IllegalStateException> {
            validator.validate(result, ExtractionConfig.builder().build())
        }
    }

    @Test
    fun `priority and name are stable`() {
        val validator = MinLengthValidator(minLength = 1)
        assertEquals("min-length-validator", validator.name())
        assertEquals(100, validator.priority())
        assertTrue(validator.should_validate(makeResult(""), ExtractionConfig.builder().build()))
    }

    @Test
    fun `registration round-trip exposes the plugin in the listing`() {
        ValidatorBridge.registerValidator(MinLengthValidator(minLength = 1))
        try {
            assertTrue("min-length-validator" in Kreuzberg.listValidators())
        } finally {
            ValidatorBridge.unregisterValidator("min-length-validator")
        }
    }
}
```
