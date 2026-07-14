#!/usr/bin/env node
// wasm-pack writes a `.gitignore` containing `*` into every --out-dir it
// produces (pkg/nodejs, pkg/bundler, ...). pnpm's packing of `file:` deps
// honors nested .gitignore files, so consumers (mcp-server, e2e/wasm) end up
// with a copy of @xberg-io/xberg-wasm whose pkg/ contents -- including the
// .d.ts -- are silently missing. The repo's `build:all` already deletes them
// with `find pkg -name .gitignore -delete`; this is the same cleanup as a
// cross-platform script, wired into every individual build script so no
// single-target build leaves the trap behind.
import { readdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const pkgDir = fileURLToPath(new URL("../crates/xberg-wasm/pkg", import.meta.url));

let removed = 0;
let targets;
try {
  targets = readdirSync(pkgDir, { withFileTypes: true });
} catch {
  console.log("clean-pkg-gitignore: no pkg/ directory, nothing to do");
  process.exit(0);
}
for (const entry of targets) {
  if (!entry.isDirectory()) continue;
  const gi = join(pkgDir, entry.name, ".gitignore");
  try {
    rmSync(gi);
    removed++;
  } catch {
    // absent -- fine
  }
}
console.log(`clean-pkg-gitignore: removed ${removed} pkg/*/.gitignore file(s)`);
