import { readFileSync, writeFileSync } from "node:fs";

const target = process.argv[2];
let s = readFileSync(target, "utf8");
const before = s;
s = s.replace(/import \* as import\d+ from "env"\r?\n/g, "");
const removed = before.split("\n").length - s.split("\n").length;
writeFileSync(target, s);
console.log("removed", removed, "unused env import lines");
