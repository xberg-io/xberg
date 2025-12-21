import { defineConfig } from "vitest/config";

export default defineConfig({
	test: {
		globals: true,
		environment: "node",
		pool: "threads",
		poolOptions: {
			threads: {
				singleThread: true,
			},
		},
		testTimeout: 30000,
		hookTimeout: 10000,
	},
});
