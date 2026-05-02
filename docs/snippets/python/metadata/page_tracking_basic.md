From Kreuzberg import extract_file_sync, ExtractionConfig, PageConfig

Config = ExtractionConfig(
pages=PageConfig(extract_pages=True)
)

Result = extract_file_sync("document.pdf", config=config)

If result.pages:
for page in result.pages:
print(f"Page {page.page_number}:")
print(f" Content: {len(page.content)} chars")
print(f" Tables: {len(page.tables)}")
print(f" Images: {len(page.images)}")
