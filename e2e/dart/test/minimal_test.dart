import 'package:test/test.dart';
import 'dart:typed_data';
import 'package:kreuzberg/kreuzberg.dart';
import 'package:kreuzberg/src/kreuzberg_bridge_generated/frb_generated.dart' show RustLib;

void main() {
  setUpAll(() async {
    await RustLib.init();
  });

  test('text extraction works', () async {
    final content = Uint8List.fromList('Hello world'.codeUnits);
    final result = await KreuzbergBridge.extractBytesSync(content, 'text/plain');
    print('Text: ${result.content.substring(0, 5)}');
  });
}
