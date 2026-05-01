import { defineConfig } from "tsup";

export default defineConfig({
	entry: [
		"typescript/**/*.ts",
		"!typescript/**/*.spec.ts",
	],
	format: ["esm"],
	outDir: "dist",
	bundle: false,
	dts: false,
	clean: true,
	sourcemap: false,
	outExtension: () => ({ js: ".js" }),
});
