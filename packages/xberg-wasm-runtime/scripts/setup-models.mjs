import { resolve } from "node:path";
import { CacheManager } from "../dist/cache.js";

const homeDirectory = process.env.USERPROFILE ?? process.env.HOME ?? ".";
const cacheDirectory = resolve(
  process.env.XBERG_CACHE_DIR ??
    (process.platform === "win32"
      ? `${process.env.LOCALAPPDATA ?? `${homeDirectory}/AppData/Local`}/xberg`
      : `${homeDirectory}/.cache/xberg`),
);

const manager = new CacheManager(cacheDirectory);
const result = await manager.warm({
  onProgress: (model) => console.log(`[setup] preparing ${model}`),
});
const status = await manager.status();

console.log(`[setup] cache=${cacheDirectory}`);
console.log(`[setup] cached=${status.cached.join(", ") || "none"} bytes=${status.size}`);
if (result.failed.length > 0) {
  console.error(`[setup] failed=${result.failed.join(", ")}`);
  process.exitCode = 1;
} else {
  console.log(`[setup] ready=${result.success.join(", ")}`);
}
