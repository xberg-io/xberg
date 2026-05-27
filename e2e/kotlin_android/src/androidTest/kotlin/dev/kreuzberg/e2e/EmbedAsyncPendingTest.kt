package dev.kreuzberg.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class EmbedAsyncPendingTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("kreuzberg_jni")
        }
    }

    @Test
    fun test_embed_texts_async_empty_input() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: embed_texts_async_empty_input */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_embed_texts_async_happy() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: embed_texts_async_happy */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_embed_texts_async_preset_switch() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: embed_texts_async_preset_switch */)
        // TODO: assert result is not an error
    }

}
