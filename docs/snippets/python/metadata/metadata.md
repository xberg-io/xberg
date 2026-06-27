```python title="Python"
from xberg import extract_sync, ExtractionConfig

result = extract_sync("document.pdf", config=ExtractionConfig())

# Metadata is flat — format-specific fields are at the top level
metadata = result.metadata
if metadata.get("page_count"):
    print(f"Pages: {metadata['page_count']}")
if metadata.get("title"):
    print(f"Title: {metadata['title']}")
if metadata.get("authors"):
    print(f"Authors: {', '.join(metadata['authors'])}")

result = extract_sync("page.html", config=ExtractionConfig())
metadata = result.metadata
if metadata.get("title"):
    print(f"Title: {metadata['title']}")
if metadata.get("description"):
    print(f"Description: {metadata['description']}")

# Access keywords as array
keywords = metadata.get('keywords', [])
if keywords:
    print(f"Keywords: {', '.join(keywords)}")

# Access canonical URL (renamed from canonical)
canonical_url = metadata.get('canonical_url')
if canonical_url:
    print(f"Canonical URL: {canonical_url}")

# Access Open Graph fields from map
open_graph = metadata.get('open_graph', {})
if open_graph:
    if 'image' in open_graph:
        print(f"Open Graph Image: {open_graph['image']}")
    if 'title' in open_graph:
        print(f"Open Graph Title: {open_graph['title']}")
    if 'type' in open_graph:
        print(f"Open Graph Type: {open_graph['type']}")

# Access Twitter Card fields from map
twitter_card = metadata.get('twitter_card', {})
if twitter_card:
    if 'card' in twitter_card:
        print(f"Twitter Card Type: {twitter_card['card']}")
    if 'creator' in twitter_card:
        print(f"Twitter Creator: {twitter_card['creator']}")

# Access new fields
language = metadata.get('language')
if language:
    print(f"Language: {language}")

text_direction = metadata.get('text_direction')
if text_direction:
    print(f"Text Direction: {text_direction}")

# Access headers
headers = metadata.get('headers', [])
if headers:
    print(f"Headers: {', '.join([h['text'] for h in headers])}")

# Access links
links = metadata.get('links', [])
if links:
    for link in links:
        print(f"Link: {link.get('href')} ({link.get('text')})")

# Access images
images = metadata.get('images', [])
if images:
    for image in images:
        print(f"Image: {image.get('src')}")

# Access structured data
structured_data = metadata.get('structured_data', [])
if structured_data:
    print(f"Structured data items: {len(structured_data)}")
```
