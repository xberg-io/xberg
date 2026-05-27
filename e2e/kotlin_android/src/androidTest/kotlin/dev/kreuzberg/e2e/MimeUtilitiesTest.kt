package dev.kreuzberg.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class MimeUtilitiesTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("kreuzberg_jni")
        }
    }

    @Test
    fun test_mime_detect_bytes() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: mime_detect_bytes */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_mime_detect_image() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: mime_detect_image */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_mime_get_extensions() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: mime_get_extensions */)
        // TODO: assert result is not an error
    }

}
