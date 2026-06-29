import 'dart:async';
import 'dart:convert';
import 'dart:io';

class MockServerHandle {
  MockServerHandle._(this._process, this.url, this.fixtureUrls);

  final Process? _process;
  final String url;
  final Map<String, String> fixtureUrls;

  Future<void> stop() async {
    final process = _process;
    if (process == null) return;

    try {
      await process.stdin.close();
    } catch (_) {}

    process.kill(ProcessSignal.sigterm);
    try {
      await process.exitCode.timeout(const Duration(seconds: 5));
    } on TimeoutException {
      process.kill(ProcessSignal.sigkill);
      await process.exitCode;
    }
  }
}

class _MockServerStartup {
  _MockServerStartup(this.url, this.fixtureUrls);

  final String url;
  final Map<String, String> fixtureUrls;
}

void useTestDocumentsCwd() {
  final testDocuments = _findTestDocumentsDir();
  Directory.current = testDocuments.path;
}

Future<MockServerHandle> startMockServer() async {
  final presetUrl = Platform.environment['MOCK_SERVER_URL'];
  if (presetUrl != null && presetUrl.isNotEmpty) {
    return MockServerHandle._(null, presetUrl, _fixtureUrlsFromEnvironment());
  }

  final repoRoot = _findRepoRoot();
  final mockServer = File(
    '${repoRoot.path}/e2e/rust/target/release/mock-server',
  );
  if (!mockServer.existsSync()) {
    throw StateError(
      'mock-server binary is missing; run scripts/e2e/build-mock-server.sh first',
    );
  }

  final process = await Process.start(mockServer.path, [
    '${repoRoot.path}/fixtures',
  ]);
  process.stderr.transform(utf8.decoder).listen(stderr.write);

  final completer = Completer<_MockServerStartup>();
  StreamSubscription<String>? subscription;
  var collectedUrl = '';
  var collectedFixtureUrls = <String, String>{};

  subscription = process.stdout
      .transform(utf8.decoder)
      .transform(const LineSplitter())
      .listen((line) {
        final trimmed = line.trim();
        if (trimmed.startsWith('MOCK_SERVER_URL=')) {
          collectedUrl = trimmed.substring('MOCK_SERVER_URL='.length);
          return;
        }
        if (trimmed.startsWith('MOCK_SERVERS=')) {
          final rawJson = trimmed.substring('MOCK_SERVERS='.length);
          final decoded = jsonDecode(rawJson) as Map<String, dynamic>;
          collectedFixtureUrls = decoded.map(
            (key, value) => MapEntry(key, value as String),
          );
          if (collectedUrl.isNotEmpty && !completer.isCompleted) {
            completer.complete(
              _MockServerStartup(collectedUrl, collectedFixtureUrls),
            );
            subscription?.cancel();
          }
        }
      }, onError: completer.completeError);

  try {
    final startup = await completer.future.timeout(const Duration(seconds: 30));
    return MockServerHandle._(process, startup.url, startup.fixtureUrls);
  } on TimeoutException {
    process.kill(ProcessSignal.sigkill);
    throw StateError('mock-server startup timeout');
  }
}

Directory _findRepoRoot() {
  var current = Directory.current.absolute;
  for (var i = 0; i < 16; i++) {
    if (File('${current.path}/Cargo.toml').existsSync() &&
        Directory('${current.path}/test_documents').existsSync()) {
      return current;
    }

    final parent = current.parent;
    if (parent.path == current.path) break;
    current = parent;
  }

  throw StateError(
    'could not locate repository root from ${Directory.current.path}',
  );
}

Directory _findTestDocumentsDir() {
  var current = Directory.current.absolute;
  for (var i = 0; i < 16; i++) {
    final candidate = Directory('${current.path}/test_documents');
    if (candidate.existsSync()) return candidate;

    final parent = current.parent;
    if (parent.path == current.path) break;
    current = parent;
  }

  throw StateError(
    'could not locate test_documents from ${Directory.current.path}',
  );
}

Map<String, String> _fixtureUrlsFromEnvironment() {
  final result = <String, String>{};
  for (final entry in Platform.environment.entries) {
    if (!entry.key.startsWith('MOCK_SERVER_') || entry.key == 'MOCK_SERVER_URL')
      continue;
    result[entry.key.substring('MOCK_SERVER_'.length).toLowerCase()] =
        entry.value;
  }
  return result;
}
