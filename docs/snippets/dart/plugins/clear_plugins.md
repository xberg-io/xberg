```dart title="Dart"
import 'package:kreuzberg/kreuzberg.dart';

Future<void> main() async {
  // The Dart binding exposes bulk-clear entry points for OCR backends,
  // post-processors, and validators. Document-extractor clearing is not
  // surfaced through flutter_rust_bridge; the built-in extractors are
  // registered automatically by the kreuzberg core when the library
  // initializes.
  await KreuzbergBridge.clearOcrBackends();
  await KreuzbergBridge.clearPostProcessors();
  await KreuzbergBridge.clearValidators();

  print('OCR backends, post-processors, and validators cleared');
}
```
