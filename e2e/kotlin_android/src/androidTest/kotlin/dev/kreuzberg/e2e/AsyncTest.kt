package dev.kreuzberg.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class AsyncTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("kreuzberg_jni")
        }
    }

    @Test
    fun test_async_extract_bytes() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: async_extract_bytes */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_async_extract_bytes_empty_mime() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: async_extract_bytes_empty_mime */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_async_extract_bytes_invalid_mime() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: async_extract_bytes_invalid_mime */)
        // TODO: assert result is not an error
    }

}
