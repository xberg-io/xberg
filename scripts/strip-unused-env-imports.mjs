import { readFileSync, writeFileSync } from "node:fs";

const target = process.argv[2];
if (!target) {
  console.error("usage: strip-unused-env-imports.mjs <glue.js>");
  process.exit(1);
}
let s = readFileSync(target, "utf8");
const before = s;
s = s.replace(/import \* as import\d+ from "env"\r?\n/g, "");
const removed = before.split("\n").length - s.split("\n").length;
if (removed === 0) {
  console.error("ERROR: no unused env import lines found (unexpected target)");
  process.exit(1);
}
writeFileSync(target, s);
console.log("removed", removed, "unused env import lines");
