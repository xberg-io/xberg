import 'package:test/test.dart';
import 'dart:typed_data';
import 'package:xberg/xberg.dart';
import 'package:xberg/src/xberg_bridge_generated/frb_generated.dart'
    show RustLib;

void main() {
  setUpAll(() async {
    await RustLib.init();
  });

  test('text extraction works', () async {
    final content = Uint8List.fromList('Hello world'.codeUnits);
    final output = await XbergBridge.extract(
      ExtractInput(
        kind: ExtractInputKind.bytes,
        bytes: content,
        mimeType: 'text/plain',
      ),
    );
    print('Text: ${output.results.first.content.substring(0, 5)}');
  });
}
