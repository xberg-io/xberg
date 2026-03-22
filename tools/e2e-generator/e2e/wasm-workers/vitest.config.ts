import { cloudflareTest } from "@cloudflare/vitest-pool-workers";
import { defineConfig } from "vitest/config";

export default defineConfig({
	plugins: [
		cloudflareTest({
			main: "./tests/index.ts",
			wrangler: {
				configPath: "./wrangler.toml",
			},
		}),
	],
	test: {
		globals: true,
		testTimeout: 60000,
	},
});
