import { env } from "@huggingface/transformers";
import type { CacheConfig } from "./types.js";

export function defaultNodeCachePath(): string {
	const home = process.env.USERPROFILE ?? process.env.HOME ?? ".";
	if (process.platform === "win32") {
		return `${process.env.LOCALAPPDATA ?? `${home}/AppData/Local`}/xberg`;
	}
	return `${home}/.cache/xberg`;
}

export function configureTransformersEnvironment(config?: CacheConfig): void {
	if (typeof process === "undefined" || !process.versions?.node) return;
	env.cacheDir = config?.nodeCachePath ?? defaultNodeCachePath();
}
