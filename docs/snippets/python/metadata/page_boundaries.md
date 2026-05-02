From Kreuzberg import extract_file_sync, ExtractionConfig

Result = extract_file_sync("document.pdf")

If result.metadata.pages and result.metadata.pages.boundaries:
boundaries = result.metadata.pages.boundaries
content_bytes = result.content.encode('utf-8')

    for boundary in boundaries[:3]:
        page_bytes = content_bytes[boundary.byte_start:boundary.byte_end]
        page_text = page_bytes.decode('utf-8')

        print(f"Page {boundary.page_number}:")
        print(f"  Byte range: {boundary.byte_start}-{boundary.byte_end}")
        print(f"  Preview: {page_text[:100]}...")
