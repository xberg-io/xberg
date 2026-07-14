// Patches the wasm-bindgen "web" target glue (pkg/web/xberg_wasm.js) so it no
// longer imports host libc shims from a separate "env" module. Instead it
// provides its own implementation that closes over the module-scoped `wasm`
// object (set during init), so strcmp/memchr can read the linear memory.
//
// Idempotent: re-running after a wasm-pack rebuild re-applies the patch.
import { readFileSync, writeFileSync } from "node:fs";

const target = process.argv[2];
if (!target) {
  console.error("usage: patch-web-env.mjs <pkg/web/xberg_wasm.js>");
  process.exit(1);
}

let s = readFileSync(target, "utf8");

if (s.includes("function makeEnv()")) {
  console.log("already patched:", target);
  process.exit(0);
}

// 1. Drop the `import * as importN from "env"` lines (10 of them).
const before = s;
s = s.replace(/import \* as import\d+ from "env"\r?\n/g, "");
if (s === before) {
  console.error("ERROR: no env import lines found (unexpected target)");
  process.exit(1);
}

// 2. Replace the ten `"env": importN,` return entries with `"env": makeEnv(),`.
if (!/"env": import\d+,/.test(s)) {
  console.error("ERROR: no env return entries found (unexpected target)");
  process.exit(1);
}
s = s.replace(/"env": import\d+,\r?\n/g, "");
s = s.replace(
  /"\.\/xberg_wasm_bg\.js": import0,/,
  '"env": makeEnv(),\n        "./xberg_wasm_bg.js": import0,',
);

// 3. Inject the makeEnv() implementation right after the `wasm` declaration so
//    the returned closures can see the module-scoped `wasm` variable.
const makeEnv = `
function makeEnv() {
  function mem() {
    if (!wasm || !wasm.memory) {
      throw new Error("xberg-wasm env: wasm memory not ready");
    }
    return wasm.memory.buffer;
  }
  function iswspace(c) {
    return ([9, 10, 11, 12, 13, 32, 0x85, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002,
      0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028,
      0x2029, 0x202f, 0x205f, 0x3000].indexOf(c) >= 0) ? 1 : 0;
  }
  function iswalpha(c) {
    return ((c >= 65 && c <= 90) || (c >= 97 && c <= 122) ||
      (c >= 0xc0 && c <= 0x24f) || (c >= 0x370 && c <= 0x1fff)) ? 1 : 0;
  }
  function iswdigit(c) { return (c >= 48 && c <= 57) ? 1 : 0; }
  function iswalnum(c) { return (iswalpha(c) || iswdigit(c)) ? 1 : 0; }
  function iswlower(c) {
    return ((c >= 97 && c <= 122) || (c >= 0xdf && c <= 0x24f &&
      c !== 0x1f6 && c !== 0x1f7 && c !== 0x1f8 && c !== 0x1f9)) ? 1 : 0;
  }
  function iswupper(c) {
    return ((c >= 65 && c <= 90) || (c >= 0xc0 && c <= 0xde)) ? 1 : 0;
  }
  function iswxdigit(c) {
    return ((c >= 48 && c <= 57) || (c >= 65 && c <= 70) || (c >= 97 && c <= 102)) ? 1 : 0;
  }
  function towupper(c) {
    if (c >= 97 && c <= 122) return c - 32;
    if (c >= 0xe0 && c <= 0xf6) return c - 32;
    if (c >= 0xf8 && c <= 0xff) return c - 32;
    return c;
  }
  function towlower(c) {
    if (c >= 65 && c <= 90) return c + 32;
    if (c >= 0xc0 && c <= 0xd6) return c + 32;
    if (c >= 0xd8 && c <= 0xde) return c + 32;
    return c;
  }
  function strcmp(s1, s2) {
    const u8 = new Uint8Array(mem());
    let i = 0;
    for (;;) {
      const a = u8[s1 + i];
      const b = u8[s2 + i];
      if (a === 0 && b === 0) return 0;
      if (a === 0) return -1;
      if (b === 0) return 1;
      if (a !== b) return a < b ? -1 : 1;
      i++;
    }
  }
  function memchr(s, c, n) {
    const u8 = new Uint8Array(mem());
    const byte = c & 0xff;
    for (let i = 0; i < n; i++) {
      if (u8[s + i] === byte) return s + i;
    }
    return 0;
  }
  return {
    iswspace, iswalpha, towupper, iswalnum, towlower,
    strcmp, iswlower, iswupper, memchr, iswxdigit,
  };
}
`;
s = s.replace(
  /let wasmModule, wasmInstance, wasm;/,
  `let wasmModule, wasmInstance, wasm;${makeEnv}`,
);

writeFileSync(target, s);
console.log("patched:", target);
