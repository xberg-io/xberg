package dev.kreuzberg.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.BeforeClass
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SmokeTest {

    companion object {
        @BeforeClass
        @JvmStatic
        fun loadNativeLibrary() {
            System.loadLibrary("kreuzberg_jni")
        }
    }

    @Test
    fun test_ocr_image_png() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: ocr_image_png */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_smoke_docx_basic() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: smoke_docx_basic */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_smoke_html_basic() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: smoke_html_basic */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_smoke_image_png() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: smoke_image_png */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_smoke_json_basic() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: smoke_json_basic */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_smoke_pdf_basic() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: smoke_pdf_basic */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_smoke_txt_basic() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: smoke_txt_basic */)
        // TODO: assert result is not an error
    }

    @Test
    fun test_smoke_xlsx_basic() {
        val client = Kreuzberg()
        val result = client.extract_file(/* fixture: smoke_xlsx_basic */)
        // TODO: assert result is not an error
    }

}
