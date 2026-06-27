```dart title="Dart"
import 'package:xberg/xberg.dart';

Future<void> main() async {
  const config = ExtractionConfig(
    useCache: true,
    enableQualityProcessing: true,
    forceOcr: true,
    disableOcr: false,
    ocr: OcrConfig(
      enabled: true,
      backend: 'tesseract',
      language: ['eng'],
      autoRotate: false,
      vlmFallback: VlmFallbackPolicy.disabled(),
    ),
    resultFormat: ResultFormat.unified,
    outputFormat: OutputFormat.plain(),
    includeDocumentStructure: false,
    maxArchiveDepth: 3,
    useLayoutForMarkdown: false,
    url: UrlExtractionConfig(
      mode: UrlExtractionMode.auto,
      allowLocalFileInputs: true,
      allowFileUris: true,
    ),
  );

  const input = ExtractInput(
    kind: ExtractInputKind.uri,
    uri: 'scanned.pdf',
  );
  final output = await XbergBridge.extract(input, config: config);
  final document = output.results.first;

  print(document.content);
}
```
