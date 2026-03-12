import { defineWorkersConfig } from "@cloudflare/vitest-pool-workers/config";

export default defineWorkersConfig({
	test: {
		globals: true,
		poolOptions: {
			workers: {
				main: "./tests/index.ts",
				wrangler: {
					configPath: "./wrangler.toml",
				},
			},
		},
		testTimeout: 60000,
	},
});
