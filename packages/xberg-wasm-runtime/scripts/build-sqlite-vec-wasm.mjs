import { execFileSync } from "node:child_process";
import { cpSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const commit = readFileSync(join(packageRoot, "vendor", "sqlite-vec-COMMIT"), "utf8").trim();
const buildRoot = mkdtempSync(join(tmpdir(), "xberg-sqlite-vec-"));
const sourceRoot = join(buildRoot, "source");
const outputRoot = join(packageRoot, "wasm", "sqlite-vec");
const sqliteVersion = "3530300";

const run = (command, args, options = {}) =>
  execFileSync(command, args, { stdio: "inherit", ...options });

try {
  run("git", ["clone", "--filter=blob:none", "https://github.com/asg017/sqlite-vec.git", sourceRoot]);
  run("git", ["checkout", "--detach", commit], { cwd: sourceRoot });
  const sourceDateEpoch = execFileSync("git", ["show", "-s", "--format=%ct", commit], {
    cwd: sourceRoot,
    encoding: "utf8",
  }).trim();
  const makefilePath = join(sourceRoot, "Makefile");
  const makefile = readFileSync(makefilePath, "utf8")
    .replace("SQLITE_WASM_VERSION=3450300", `SQLITE_WASM_VERSION=${sqliteVersion}`)
    .replace("SQLITE_WASM_YEAR=2024", "SQLITE_WASM_YEAR=2026");
  writeFileSync(makefilePath, makefile);

  const mount = `${sourceRoot}:/src`;
  const buildCommand = [
    "apt-get update -qq",
    "apt-get install -y -qq tcl file gettext-base curl unzip",
    `curl -fsSL -o /tmp/sqlite.zip https://www.sqlite.org/sqlite-amalgamation-${sqliteVersion}.zip`,
    "unzip -q /tmp/sqlite.zip -d /tmp/sqlite",
    "mkdir -p vendor",
    `touch -d @${sourceDateEpoch} VERSION`,
    `cp /tmp/sqlite/sqlite-amalgamation-${sqliteVersion}/sqlite3.h vendor/sqlite3.h`,
    `cp /tmp/sqlite/sqlite-amalgamation-${sqliteVersion}/sqlite3ext.h vendor/sqlite3ext.h`,
    "make sqlite-vec.h wasm",
  ].join(" && ");

  run("docker", [
    "run", "--rm",
    "-e", "DEBIAN_FRONTEND=noninteractive",
    "-e", "TZ=Etc/UTC",
    "-e", `SOURCE_DATE_EPOCH=${sourceDateEpoch}`,
    "-v", mount,
    "-w", "/src",
    "emscripten/emsdk:3.1.74",
    "bash", "-lc", buildCommand,
  ]);

  mkdirSync(outputRoot, { recursive: true });
  cpSync(join(sourceRoot, "dist", ".wasm", "sqlite3.mjs"), join(outputRoot, "sqlite3.mjs"));
  cpSync(join(sourceRoot, "dist", ".wasm", "sqlite3.wasm"), join(outputRoot, "sqlite3.wasm"));
  cpSync(
    join(sourceRoot, "dist", ".build", `sqlite-src-${sqliteVersion}`, "ext", "wasm", "jswasm", "sqlite3-opfs-async-proxy.js"),
    join(outputRoot, "sqlite3-opfs-async-proxy.js"),
  );
  console.log(`Built pinned sqlite-vec ${commit} into ${outputRoot}`);
} finally {
  rmSync(buildRoot, { recursive: true, force: true });
}
