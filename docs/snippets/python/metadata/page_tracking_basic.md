from xberg import extract, ExtractInput, ExtractionConfig, PageConfig

config = ExtractionConfig(
    pages=PageConfig(extract_pages=True)
)

result = extract(ExtractInput.from_uri("document.pdf"), config)

if result.results[0].pages:
    for page in result.results[0].pages:
        print(f"Page {page.page_number}:")
        print(f" Content: {len(page.content)} chars")
        print(f" Tables: {len(page.tables)}")
        print(f" Images: {len(page.images)}")
