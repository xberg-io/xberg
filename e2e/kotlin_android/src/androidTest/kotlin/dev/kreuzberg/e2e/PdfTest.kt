package dev.kreuzberg.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class PdfTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("kreuzberg_jni")
        }
    }

    @Test
    fun test_render_pdf_page_first() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: render_pdf_page_first */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_render_pdf_page_out_of_range() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: render_pdf_page_out_of_range */)
        // TODO: assert result is not an error
    }

}
