/* Flat C shim over libwpd + librevenge for Xberg.
 *
 * libwpd exposes no `extract()` call. It drives librevenge's SAX-like
 * RVNGTextInterface: the caller passes a concrete implementation into
 * WPDocument::parse and libwpd invokes its callbacks. This file provides such
 * an implementation (TextCollector) that accumulates a plain-text rendering of
 * the document, and exposes it to Rust through a flat C API returning owned
 * UTF-8 that the Rust side frees.
 *
 * Every entry point catches all C++ exceptions: libwpd throws on malformed
 * input, and an exception must never unwind across the FFI boundary.
 */
#include <librevenge-stream/librevenge-stream.h>
#include <librevenge/librevenge.h>
#include <libwpd/libwpd.h>

#include <cstdlib>
#include <cstring>
#include <string>

namespace {
using librevenge::RVNGPropertyList;
using librevenge::RVNGString;

/* Accumulates document text. Paragraph, list-item and table-cell/row
 * boundaries emit whitespace so downstream paragraph splitting works and table
 * content stays legible (cells tab-separated, rows newline-separated). */
class TextCollector : public librevenge::RVNGTextInterface {
  public:
    std::string text;

    // Content callbacks that carry text.
    void insertText(const RVNGString &s) override {
        if (s.cstr())
            text += s.cstr();
    }
    void insertTab() override {
        text += '\t';
    }
    void insertSpace() override {
        text += ' ';
    }
    void insertLineBreak() override {
        text += '\n';
    }
    void closeParagraph() override {
        text += "\n\n";
    }
    void closeListElement() override {
        text += '\n';
    }

    // Tables: tab between cells, newline between rows, blank line after table.
    void closeTableCell() override {
        text += '\t';
    }
    void closeTableRow() override {
        text += '\n';
    }
    void closeTable() override {
        text += '\n';
    }

    // Remaining pure virtuals are structural and produce no text of their own.
    void setDocumentMetaData(const RVNGPropertyList &) override {}
    void startDocument(const RVNGPropertyList &) override {}
    void endDocument() override {}
    void definePageStyle(const RVNGPropertyList &) override {}
    void defineEmbeddedFont(const RVNGPropertyList &) override {}
    void openPageSpan(const RVNGPropertyList &) override {}
    void closePageSpan() override {}
    void openHeader(const RVNGPropertyList &) override {}
    void closeHeader() override {}
    void openFooter(const RVNGPropertyList &) override {}
    void closeFooter() override {}
    void defineParagraphStyle(const RVNGPropertyList &) override {}
    void openParagraph(const RVNGPropertyList &) override {}
    void defineCharacterStyle(const RVNGPropertyList &) override {}
    void openSpan(const RVNGPropertyList &) override {}
    void closeSpan() override {}
    void openLink(const RVNGPropertyList &) override {}
    void closeLink() override {}
    void defineSectionStyle(const RVNGPropertyList &) override {}
    void openSection(const RVNGPropertyList &) override {}
    void closeSection() override {}
    void insertField(const RVNGPropertyList &) override {}
    void openOrderedListLevel(const RVNGPropertyList &) override {}
    void openUnorderedListLevel(const RVNGPropertyList &) override {}
    void closeOrderedListLevel() override {}
    void closeUnorderedListLevel() override {}
    void openListElement(const RVNGPropertyList &) override {}
    void openFootnote(const RVNGPropertyList &) override {}
    void closeFootnote() override {}
    void openEndnote(const RVNGPropertyList &) override {}
    void closeEndnote() override {}
    void openComment(const RVNGPropertyList &) override {}
    void closeComment() override {}
    void openTextBox(const RVNGPropertyList &) override {}
    void closeTextBox() override {}
    void openTable(const RVNGPropertyList &) override {}
    void openTableRow(const RVNGPropertyList &) override {}
    void openTableCell(const RVNGPropertyList &) override {}
    void insertCoveredTableCell(const RVNGPropertyList &) override {}
    void openFrame(const RVNGPropertyList &) override {}
    void closeFrame() override {}
    void insertBinaryObject(const RVNGPropertyList &) override {}
    void insertEquation(const RVNGPropertyList &) override {}
    void openGroup(const RVNGPropertyList &) override {}
    void closeGroup() override {}
    void defineGraphicStyle(const RVNGPropertyList &) override {}
    void drawRectangle(const RVNGPropertyList &) override {}
    void drawEllipse(const RVNGPropertyList &) override {}
    void drawPolygon(const RVNGPropertyList &) override {}
    void drawPolyline(const RVNGPropertyList &) override {}
    void drawPath(const RVNGPropertyList &) override {}
    void drawConnector(const RVNGPropertyList &) override {}
};
} // namespace

extern "C" {

/* Result codes shared with the Rust side (see error.rs). */
enum {
    XBERG_WPD_OK = 0,
    XBERG_WPD_INVALID_ARGS = 1,
    XBERG_WPD_UNSUPPORTED_FORMAT = 2,
    XBERG_WPD_PARSE_ERROR = 3,
    XBERG_WPD_OUT_OF_MEMORY = 4,
    XBERG_WPD_PANIC = 5,
};

/* Returns non-zero if the buffer looks like a WordPerfect document libwpd can
 * parse. Never throws. */
int xberg_wpd_is_supported(const unsigned char *data, unsigned long len) {
    if (!data || len == 0)
        return 0;
    try {
        librevenge::RVNGStringStream input(data, static_cast<unsigned int>(len));
        return libwpd::WPDocument::isFileFormatSupported(&input) != libwpd::WPD_CONFIDENCE_NONE ? 1
                                                                                                : 0;
    } catch (...) {
        return 0;
    }
}

/* Extract UTF-8 text from an in-memory WordPerfect document. On XBERG_WPD_OK,
 * *out_text is a malloc'd NUL-terminated buffer the caller frees via
 * xberg_wpd_free_string. On any other return, *out_text is left null. */
int xberg_wpd_extract_text(const unsigned char *data, unsigned long len, char **out_text) {
    if (!out_text)
        return XBERG_WPD_INVALID_ARGS;
    *out_text = nullptr;
    if (!data || len == 0)
        return XBERG_WPD_INVALID_ARGS;

    try {
        librevenge::RVNGStringStream input(data, static_cast<unsigned int>(len));
        if (libwpd::WPDocument::isFileFormatSupported(&input) == libwpd::WPD_CONFIDENCE_NONE)
            return XBERG_WPD_UNSUPPORTED_FORMAT;

        TextCollector collector;
        if (libwpd::WPDocument::parse(&input, &collector, nullptr) != libwpd::WPD_OK)
            return XBERG_WPD_PARSE_ERROR;

        const size_t n = collector.text.size();
        char *buf = static_cast<char *>(std::malloc(n + 1));
        if (!buf)
            return XBERG_WPD_OUT_OF_MEMORY;
        std::memcpy(buf, collector.text.data(), n);
        buf[n] = '\0';
        *out_text = buf;
        return XBERG_WPD_OK;
    } catch (...) {
        return XBERG_WPD_PANIC;
    }
}

void xberg_wpd_free_string(char *s) {
    std::free(s);
}

} // extern "C"
