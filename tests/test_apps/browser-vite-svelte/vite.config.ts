import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

function wasmImportPlugin() {
	return {
		name: "wasm-dynamic-import",
		resolveId: (id: string) => {
			if (id.includes("kreuzberg_wasm.js") || id.includes("pdfium.js")) {
				return { id, external: true };
			}
			return null;
		},
	};
}

export default defineConfig({
	plugins: [wasmImportPlugin(), svelte()],
});
