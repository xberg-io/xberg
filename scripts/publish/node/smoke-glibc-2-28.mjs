// Glibc-2.28 loadability smoke test for the @kreuzberg/node prebuilds.
//
// Runs inside redhat/ubi8 (glibc 2.28) against an extracted node-bindings-*.tar.gz.
// Loads the .node directly via process.dlopen so this script has no
// dependency on the alef-generated TS wrapper or any other tooling.
//
// We're not testing feature parity — that's covered by ci-e2e.yaml. We're
// testing that the prebuilt .node loads on glibc 2.28 and that its core napi
// surface is reachable. If glibc/glibcxx ever silently drift past 2.28, this
// smoke fails before publish.
//
// Required env: NODE_PATH — absolute path to the .node file inside the unpacked tar.

import { readFileSync, existsSync } from 'node:fs';
import { createRequire } from 'node:module';

const nodePath = process.env.NODE_PATH;
if (!nodePath) {
  console.error('NODE_PATH not set (path to the .node file)');
  process.exit(2);
}
if (!existsSync(nodePath)) {
  console.error(`NODE_PATH does not exist: ${nodePath}`);
  process.exit(2);
}

console.log(`=== Loading ${nodePath} ===`);

const require_ = createRequire(import.meta.url);
let native;
try {
  native = require_(nodePath);
} catch (e) {
  console.error('FAIL: dlopen of .node failed (glibc symbol miss?):');
  console.error(e?.message || e);
  process.exit(1);
}

const failures = [];
function check(name, fn) {
  try {
    fn();
    console.log(`  OK  ${name}`);
  } catch (e) {
    console.error(`  FAIL ${name}: ${e?.message || e}`);
    failures.push(name);
  }
}

check('module is an object', () => {
  if (typeof native !== 'object' || native === null) {
    throw new Error(`got ${typeof native}`);
  }
});

const required = [
  'extractBytes',
  'extractBytesSync',
  'extractTextFromPdf',
  'chunkText',
  'detectMimeTypeFromBytes',
  'blake3HashBytes',
  'validateChunkingParams',
  'validateMimeType',
  'listSupportedFormats',
];
for (const name of required) {
  check(`export ${name} is function`, () => {
    if (typeof native[name] !== 'function') {
      throw new Error(`got ${typeof native[name]}`);
    }
  });
}

check('listSupportedFormats returns non-empty array', () => {
  const formats = native.listSupportedFormats();
  if (!Array.isArray(formats) || formats.length === 0) {
    throw new Error(`got ${typeof formats} length=${Array.isArray(formats) ? formats.length : 'n/a'}`);
  }
});

check('blake3HashBytes returns non-empty hex string', () => {
  const hash = native.blake3HashBytes(Array.from(Buffer.from('kreuzberg-glibc-228-smoke')));
  if (typeof hash !== 'string' || hash.length === 0) {
    throw new Error(`got ${typeof hash} length=${hash?.length}`);
  }
});

check('detectMimeTypeFromBytes(%PDF-) -> application/pdf', () => {
  const mime = native.detectMimeTypeFromBytes([0x25, 0x50, 0x44, 0x46, 0x2d, 0x31, 0x2e, 0x34]);
  if (mime !== 'application/pdf') throw new Error(`got ${mime}`);
});

check('validateChunkingParams accepts (1000, 100)', () => {
  native.validateChunkingParams(1000, 100);
});

check('validateChunkingParams rejects (10, 50) (overlap >= max)', () => {
  let threw = false;
  try { native.validateChunkingParams(10, 50); } catch { threw = true; }
  if (!threw) throw new Error('expected throw, got success');
});

console.log(`\n=== Summary ===`);
if (failures.length === 0) {
  console.log(`OK: ${nodePath} loads and responds on glibc 2.28.`);
  process.exit(0);
} else {
  console.error(`FAIL: ${failures.length} smoke check(s) failed: ${failures.join(', ')}`);
  process.exit(1);
}
