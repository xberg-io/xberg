```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  final extractors = await KreuzbergBridge.listDocumentExtractors();
  print('Registered extractors: $extractors');

  final processors = await KreuzbergBridge.listPostProcessors();
  print('Registered processors: $processors');

  final backends = await KreuzbergBridge.listOcrBackends();
  print('Registered OCR backends: $backends');

  final validators = await KreuzbergBridge.listValidators();
  print('Registered validators: $validators');
}
```
