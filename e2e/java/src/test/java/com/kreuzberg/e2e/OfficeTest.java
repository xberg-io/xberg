package com.kreuzberg.e2e;

// CHECKSTYLE.OFF: UnusedImports - generated code
// CHECKSTYLE.OFF: LineLength - generated code
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.kreuzberg.BytesWithMime;
import dev.kreuzberg.ExtractionResult;
import dev.kreuzberg.Kreuzberg;
import dev.kreuzberg.config.ExtractionConfig;
import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertTrue;
// CHECKSTYLE.ON: UnusedImports
// CHECKSTYLE.ON: LineLength

/** Auto-generated tests for office fixtures. */
public class OfficeTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void officeBibtexBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_bibtex_basic",
            "bibtex/comprehensive.bib",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/x-bibtex", "text/x-bibtex"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeDjotBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_djot_basic",
            "markdown/tables.djot",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("text/x-djot", "text/djot"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeDocLegacy() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_doc_legacy",
            "doc/unit_test_lists.doc",
            config,
            Arrays.asList("office"),
            "Requires the office feature.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/msword"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocbookBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docbook_basic",
            "docbook/docbook-reader.docbook",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/docbook+xml", "text/docbook"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeDocxBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_basic",
            "docx/sample_document.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeDocxEquations() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_equations",
            "docx/equations.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxFake() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_fake",
            "docx/fake.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxFormatting() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_formatting",
            "docx/unit_test_formatting.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxHeaders() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_headers",
            "docx/unit_test_headers.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxLists() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_lists",
            "docx/unit_test_lists.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeDocxTables() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_docx_tables",
            "docx/docx_tables.docx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.wordprocessingml.document"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
                E2EHelpers.Assertions.assertContentContainsAll(result, Arrays.asList("Simple uniform table", "Nested Table", "merged cells", "Header Col"));
                E2EHelpers.Assertions.assertTableCount(result, 1, null);
            }
        );
    }

    @Test
    public void officeEpubBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_epub_basic",
            "epub/features.epub",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/epub+zip"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
            }
        );
    }

    @Test
    public void officeFb2Basic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_fb2_basic",
            "fictionbook/basic.fb2",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/x-fictionbook+xml"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeFictionbookBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_fictionbook_basic",
            "fictionbook/basic.fb2",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/x-fictionbook+xml", "application/x-fictionbook"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeJatsBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_jats_basic",
            "jats/sample_article.jats",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/x-jats+xml", "text/jats"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeJupyterBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_jupyter_basic",
            "jupyter/rank.ipynb",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/x-ipynb+json"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeLatexBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_latex_basic",
            "latex/basic_sections.tex",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/x-latex", "text/x-latex"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeMarkdownBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_markdown_basic",
            "markdown/comprehensive.md",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("text/markdown"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeOdsBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_ods_basic",
            "data_formats/test_01.ods",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.oasis.opendocument.spreadsheet"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeOdtBold() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_odt_bold",
            "odt/bold.odt",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.oasis.opendocument.text"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeOdtList() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_odt_list",
            "odt/unorderedList.odt",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.oasis.opendocument.text"));
                E2EHelpers.Assertions.assertMinContentLength(result, 30);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("list item", "New level", "Pushed us"));
            }
        );
    }

    @Test
    public void officeOdtSimple() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_odt_simple",
            "odt/simple.odt",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.oasis.opendocument.text"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("favorite things", "Parrots", "Analysis"));
            }
        );
    }

    @Test
    public void officeOdtTable() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_odt_table",
            "odt/table.odt",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.oasis.opendocument.text"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
                E2EHelpers.Assertions.assertTableCount(result, 1, null);
            }
        );
    }

    @Test
    public void officeOpmlBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_opml_basic",
            "opml/outline.opml",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/xml+opml", "text/x-opml", "application/x-opml+xml"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeOrgBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_org_basic",
            "org/comprehensive.org",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("text/x-org", "text/org"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officePpsxSlideshow() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_ppsx_slideshow",
            "pptx/sample.ppsx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.presentationml.slideshow"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officePptLegacy() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_ppt_legacy",
            "ppt/simple.ppt",
            config,
            Arrays.asList("office"),
            "Requires the office feature.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.ms-powerpoint"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officePptxBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_pptx_basic",
            "pptx/simple.pptx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.presentationml.presentation"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
            }
        );
    }

    @Test
    public void officePptxImages() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_pptx_images",
            "pptx/powerpoint_with_image.pptx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.presentationml.presentation"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officePptxPitchDeck() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_pptx_pitch_deck",
            "pptx/pitch_deck_presentation.pptx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.presentationml.presentation"));
                E2EHelpers.Assertions.assertMinContentLength(result, 100);
            }
        );
    }

    @Test
    public void officeRstBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_rst_basic",
            "rst/restructured_text.rst",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("text/x-rst", "text/prs.fallenstein.rst"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
            }
        );
    }

    @Test
    public void officeRtfBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_rtf_basic",
            "rtf/extraction_test.rtf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/rtf", "text/rtf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeTypstBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_typst_basic",
            "typst/headings.typ",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/x-typst", "text/x-typst"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeXlsLegacy() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_xls_legacy",
            "xls/test_excel.xls",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.ms-excel"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void officeXlsxBasic() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_xlsx_basic",
            "xlsx/stanley_cups.xlsx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
                E2EHelpers.Assertions.assertMinContentLength(result, 100);
                E2EHelpers.Assertions.assertContentContainsAll(result, Arrays.asList("Team", "Location", "Stanley Cups"));
                E2EHelpers.Assertions.assertTableCount(result, 1, null);
                E2EHelpers.Assertions.assertMetadataExpectation(result, "sheet_count", Map.of("gte", 2));
                E2EHelpers.Assertions.assertMetadataExpectation(result, "sheet_names", Map.of("contains", Arrays.asList("Stanley Cups")));
            }
        );
    }

    @Test
    public void officeXlsxMultiSheet() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_xlsx_multi_sheet",
            "xlsx/excel_multi_sheet.xlsx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
                E2EHelpers.Assertions.assertMetadataExpectation(result, "sheet_count", Map.of("gte", 2));
            }
        );
    }

    @Test
    public void officeXlsxOfficeExample() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "office_xlsx_office_example",
            "xlsx/test_01.xlsx",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

}
