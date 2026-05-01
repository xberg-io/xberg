import { copyFileSync, mkdirSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = join(__dirname, "..");
const pkgDir = join(root, "pkg");
const distPkgDir = join(root, "dist", "pkg");
const distDir = join(root, "dist");

mkdirSync(distPkgDir, { recursive: true });

const SKIP = new Set(["README.md", "package.json", ".gitignore"]);
for (const file of readdirSync(pkgDir)) {
	if (SKIP.has(file)) continue;
	copyFileSync(join(pkgDir, file), join(distPkgDir, file));
	console.log(`  pkg/${file} → dist/pkg/${file}`);
}

copyFileSync(join(pkgDir, "kreuzberg_wasm.js"), join(distDir, "kreuzberg_wasm.js"));
console.log("  pkg/kreuzberg_wasm.js → dist/kreuzberg_wasm.js");

// pdfium.js is a plain JS helper in the TypeScript source tree
const pdfiumSrc = join(root, "typescript", "pdfium.js");
copyFileSync(pdfiumSrc, join(distDir, "pdfium.js"));
console.log("  typescript/pdfium.js → dist/pdfium.js");

console.log("copy-pkg: done");
