import { env } from "@huggingface/transformers";
import { afterEach, describe, expect, it } from "vitest";
import { configureTransformersEnvironment, defaultNodeCachePath } from "./runtime-env";

describe("configureTransformersEnvironment", () => {
	const originalCacheDirectory = env.cacheDir;

	afterEach(() => {
		env.cacheDir = originalCacheDirectory;
	});

	it("uses the configured Node cache directory", () => {
		configureTransformersEnvironment({ nodeCachePath: "C:/tmp/xberg-models" });
		expect(env.cacheDir).toBe("C:/tmp/xberg-models");
	});

	it("uses the platform default when no cache directory is configured", () => {
		configureTransformersEnvironment();
		expect(env.cacheDir).toBe(defaultNodeCachePath());
	});
});
