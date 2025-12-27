```python title="Python"
from kreuzberg import extract_file_sync, ExtractionConfig

result = extract_file_sync("document.pdf", config=ExtractionConfig())

pdf_meta: dict = result.metadata.get("pdf", {})
if pdf_meta:
    print(f"Pages: {pdf_meta.get('page_count')}")
    print(f"Author: {pdf_meta.get('author')}")
    print(f"Title: {pdf_meta.get('title')}")

result = extract_file_sync("page.html", config=ExtractionConfig())
html_meta: dict = result.metadata.get("html", {})
if html_meta:
    print(f"Title: {html_meta.get('title')}")
    print(f"Description: {html_meta.get('description')}")

    # Access keywords as array
    keywords = html_meta.get('keywords', [])
    if keywords:
        print(f"Keywords: {', '.join(keywords)}")

    # Access canonical URL (renamed from canonical)
    canonical_url = html_meta.get('canonical_url')
    if canonical_url:
        print(f"Canonical URL: {canonical_url}")

    # Access Open Graph fields from map
    open_graph = html_meta.get('open_graph', {})
    if open_graph:
        if 'image' in open_graph:
            print(f"Open Graph Image: {open_graph['image']}")
        if 'title' in open_graph:
            print(f"Open Graph Title: {open_graph['title']}")
        if 'type' in open_graph:
            print(f"Open Graph Type: {open_graph['type']}")

    # Access Twitter Card fields from map
    twitter_card = html_meta.get('twitter_card', {})
    if twitter_card:
        if 'card' in twitter_card:
            print(f"Twitter Card Type: {twitter_card['card']}")
        if 'creator' in twitter_card:
            print(f"Twitter Creator: {twitter_card['creator']}")

    # Access new fields
    language = html_meta.get('language')
    if language:
        print(f"Language: {language}")

    text_direction = html_meta.get('text_direction')
    if text_direction:
        print(f"Text Direction: {text_direction}")

    # Access headers
    headers = html_meta.get('headers')
    if headers:
        print(f"Headers: {', '.join(headers)}")

    # Access links
    links = html_meta.get('links', [])
    if links:
        for link in links:
            print(f"Link: {link.get('href')} ({link.get('text')})")

    # Access images
    images = html_meta.get('images', [])
    if images:
        for image in images:
            print(f"Image: {image.get('src')}")

    # Access structured data
    structured_data = html_meta.get('structured_data', [])
    if structured_data:
        print(f"Structured data items: {len(structured_data)}")
```
