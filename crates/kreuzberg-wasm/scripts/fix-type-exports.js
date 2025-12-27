#!/usr/bin/env node
/**
 * Post-build script to fix missing type exports in generated .d.ts files
 * Ensures ExtractionConfig and ExtractionResult are exported from the main entry point
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const distDir = path.join(__dirname, "..", "dist");

/**
 * Fix type exports in a .d.ts or .d.mts file
 * @param {string} filePath - Path to the file to fix
 */
function fixTypeExports(filePath) {
	try {
		if (!fs.existsSync(filePath)) {
			console.warn(`File not found: ${filePath}`);
			return;
		}

		let content = fs.readFileSync(filePath, "utf-8");

		let moduleRef = null;
		const moduleAlias = null;

		const typeImportMatch = content.match(/import\s+{([^}]+)}\s+from\s+['"](\.\/(types-[^\s'"]+))['"];?/);
		if (typeImportMatch) {
			const importPath = typeImportMatch[2];
			const baseModule = importPath.replace(/\.(mjs|d\.mts|d\.ts|js)$/, "");
			if (filePath.endsWith(".d.mts")) {
				moduleRef = `${baseModule}.d.mts`;
			} else {
				moduleRef = `${baseModule}.js`;
			}

			const imports = typeImportMatch[1];
		}

		if (!moduleRef) {
			console.log(`- Could not determine types module for ${path.basename(filePath)}`);
			return;
		}

		let importModuleRef = moduleRef;
		if (!filePath.endsWith(".d.mts") && !filePath.endsWith(".d.cts")) {
			importModuleRef = moduleRef.replace(/\.js$/, ".d.ts");
		}
		const correctedImport = `import { E as ExtractionConfig, a as ExtractionResult } from '${importModuleRef}';`;
		const correctedExport = `export { C as Chunk, b as ChunkingConfig, c as ChunkMetadata, d as ExtractedImage, I as ImageExtractionConfig, L as LanguageDetectionConfig, M as Metadata, O as OcrBackendProtocol, e as OcrConfig, P as PageContent, f as PageExtractionConfig, g as PdfConfig, h as PostProcessorConfig, T as Table, i as TesseractConfig, j as TokenReductionConfig, E as ExtractionConfig, a as ExtractionResult } from '${importModuleRef}';`;

		const lines = content.split("\n");
		let replaced = false;
		let foundCorrectExport = false;
		let importFixed = false;
		let duplicateRemoved = false;
		let runtimeFixed = false;

		for (let i = 0; i < lines.length; i++) {
			let line = lines[i];

			if (line.startsWith("import {") && /from\s+['"]\.\/types-[^'"]+['"]/.test(line)) {
				if (!line.includes(importModuleRef)) {
					lines[i] = correctedImport;
					importFixed = true;
				}
			}

			if (line.startsWith("export {") && /from\s+['"]\.\/types-[^'"]+['"]/.test(line)) {
				if (line.includes("ExtractionConfig") && line.includes("ExtractionResult") && line.includes(importModuleRef)) {
					foundCorrectExport = true;
				} else if (line.includes("from")) {
					lines[i] = correctedExport;
					replaced = true;
				}
			}

			if (
				line.startsWith("export {") &&
				!line.includes("from") &&
				line.includes("ExtractionConfig") &&
				line.includes("ExtractionResult")
			) {
				const exportContent = line.match(/export\s+\{([^}]+)\}/)?.[1];
				if (exportContent) {
					const exports = exportContent
						.split(",")
						.map((e) => e.trim())
						.filter((e) => {
							return e !== "ExtractionConfig" && e !== "ExtractionResult";
						});
					if (exports.length > 0) {
						lines[i] = `export { ${exports.join(", ")} };`;
						duplicateRemoved = true;
					} else {
						lines[i] = "";
						duplicateRemoved = true;
					}
				}
			}

			if (line.includes("from './runtime") && (line.includes("RuntimeType") || line.includes("WasmCapabilities"))) {
				line = line.replace(/(\sRuntimeType(?=\s*,|\s*\}|\s*from))/g, " type RuntimeType");
				line = line.replace(/(\sWasmCapabilities(?=\s*,|\s*\}|\s*from))/g, " type WasmCapabilities");
				if (filePath.endsWith(".d.mts")) {
					line = line.replace(/from\s+['"]\.\/runtime\.js['"]/, "from './runtime.d.mts'");
				} else if (filePath.endsWith(".d.cts")) {
					line = line.replace(/from\s+['"]\.\/runtime\.cjs['"]/, "from './runtime.d.cts'");
				} else if (filePath.endsWith(".d.ts")) {
					line = line.replace(/from\s+['"]\.\/runtime\.js['"]/, "from './runtime.d.ts'");
				}
				lines[i] = line;
				runtimeFixed = true;
			}

			if (filePath.endsWith(".d.mts") && line.includes("from './runtime.mjs'") && line.includes("RuntimeType")) {
				line = line.replace("from './runtime.mjs'", "from './runtime.d.mts'");
				lines[i] = line;
				runtimeFixed = true;
			}

			if (filePath.endsWith(".d.mts") && /from\s+['"]\.\/(adapters|ocr)\/[^'"]+\.mjs['"]/.test(line)) {
				line = line.replace(/\.mjs'/g, ".d.mts'").replace(/\.mjs"/g, '.d.mts"');
				lines[i] = line;
				runtimeFixed = true;
			}
		}

		if (replaced || importFixed || duplicateRemoved || runtimeFixed) {
			content = lines.join("\n");
			fs.writeFileSync(filePath, content);
			const changes = [];
			if (importFixed) changes.push("imports");
			if (replaced) changes.push("exports");
			if (duplicateRemoved) changes.push("duplicates");
			if (runtimeFixed) changes.push("module references");
			console.log(`✓ Fixed type ${changes.join(" and ")} in ${path.basename(filePath)}`);
		} else if (foundCorrectExport) {
			console.log(`✓ ${path.basename(filePath)} already has correct exports`);
		} else {
			console.log(`- No changes needed for ${path.basename(filePath)}`);
		}
	} catch (error) {
		console.error(`✗ Error fixing ${filePath}:`, error.message);
		process.exit(1);
	}
}

/**
 * Recursively find all .d.ts, .d.mts, and .d.cts files
 * @param {string} dir - Directory to search
 * @returns {string[]} Array of file paths
 */
function findTypeDefinitions(dir) {
	const files = [];
	const entries = fs.readdirSync(dir, { withFileTypes: true });

	for (const entry of entries) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			files.push(...findTypeDefinitions(fullPath));
		} else if (entry.name.endsWith(".d.ts") || entry.name.endsWith(".d.mts") || entry.name.endsWith(".d.cts")) {
			files.push(fullPath);
		}
	}

	return files;
}

console.log("Fixing type exports in generated .d.ts files...\n");

const typeFiles = findTypeDefinitions(distDir);
for (const file of typeFiles) {
	fixTypeExports(file);
}

console.log("\nFixing remaining type references in .d.mts files...");
const dmtsFiles = typeFiles.filter((f) => f.endsWith(".d.mts"));
for (const file of dmtsFiles) {
	try {
		let content = fs.readFileSync(file, "utf-8");
		const originalContent = content;

		content = content.replace(/types-([^'"/]+)\.mjs/g, "types-$1.d.mts");

		if (content !== originalContent) {
			fs.writeFileSync(file, content);
			console.log(`✓ Fixed remaining references in ${path.basename(file)}`);
		}
	} catch (error) {
		console.error(`✗ Error fixing ${file}:`, error.message);
	}
}

console.log("\nType export fixes complete!");
