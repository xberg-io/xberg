// Patches .d.ts files emitted by tsc to add .js extensions on bare relative
// imports — required for NodeNext module resolution in consumers.
import { readdirSync, readFileSync, writeFileSync, statSync } from "node:fs";
import { join, extname, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const distDir = join(__dirname, "..", "dist");

function walk(dir) {
	for (const name of readdirSync(dir)) {
		const full = join(dir, name);
		if (statSync(full).isDirectory()) {
			walk(full);
		} else if (name.endsWith(".d.ts")) {
			const src = readFileSync(full, "utf8");
			const fixed = src.replace(
				/(from\s+['"])(\.\.?\/[^'"]+?)(['"])/g,
				(_, a, p, c) => (extname(p) ? `${a}${p}${c}` : `${a}${p}.js${c}`),
			);
			if (fixed !== src) {
				writeFileSync(full, fixed);
				console.log(`  patched ${full.replace(distDir + "/", "")}`);
			}
		}
	}
}

walk(distDir);
console.log("fix-type-exports: done");
