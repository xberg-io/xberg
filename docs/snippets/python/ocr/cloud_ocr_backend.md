```python title="Python"
from xberg import register_ocr_backend, ExtractedDocument, OcrBackendType, OcrConfig, Metadata
import httpx

class CloudOcrBackend:
    def __init__(self, api_key: str):
        self.api_key: str = api_key
        self.langs: list[str] = ["eng", "deu", "fra"]

    def name(self) -> str:
        return "cloud-ocr"

    def version(self) -> str:
        return "1.0.0"

    def supported_languages(self) -> list[str]:
        return self.langs

    def supports_language(self, lang: str) -> bool:
        return lang in self.langs

    def backend_type(self) -> OcrBackendType:
        return OcrBackendType.CUSTOM

    def supports_table_detection(self) -> bool:
        return False

    def supports_document_processing(self) -> bool:
        return False

    def emits_structured_markdown(self) -> bool:
        return False

    def process_image(self, image_bytes: bytes, config: OcrConfig) -> ExtractedDocument:
        with httpx.Client() as client:
            response = client.post(
                "https://api.example.com/ocr",
                files={"image": image_bytes},
                json={"language": config.language[0] if config.language else "eng"},
            )
            text: str = response.json()["text"]
            return ExtractedDocument(
                content=text,
                mime_type="text/plain",
                metadata=Metadata(),
            )

    def process_image_file(self, path: str, config: OcrConfig) -> ExtractedDocument:
        with open(path, "rb") as f:
            return self.process_image(f.read(), config)

    def process_document(self, path: str, config: OcrConfig) -> ExtractedDocument:
        return self.process_image_file(path, config)

    def initialize(self) -> None:
        pass

    def shutdown(self) -> None:
        pass

backend: CloudOcrBackend = CloudOcrBackend(api_key="your-api-key")
register_ocr_backend(backend)
```
