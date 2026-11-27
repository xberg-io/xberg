#!/usr/bin/env node
// Launcher: exec the native xberg binary, forwarding argv and inheriting stdio.
// Order: the binary shipped by the installed platform package (no network), then a healthy
// binary already in bin/, then an on-demand download from the GitHub release.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { resolveBundledBinary } from "../install.js";

const BIN_NAME = "xberg";
const MALLOC_MAGAZINES = "MallocMaxMagazines";

function binaryName() {
	return os.type() === "Windows_NT" ? `${BIN_NAME}.exe` : BIN_NAME;
}

const __dirname = path.dirname(fileURLToPath(import.meta.url));
// install.js extracts the binary into this same bin/ directory.
const binPath = path.join(__dirname, binaryName());

// A cached binary is only usable if it is non-empty and (on non-Windows) has an
// exec bit. A truncated or non-executable file means a corrupt cache: re-download.
function isHealthy(file) {
	try {
		const stat = fs.statSync(file);
		if (stat.size <= 0) return false;
		if (os.type() !== "Windows_NT" && (stat.mode & 0o111) === 0) return false;
		return true;
	} catch {
		return false;
	}
}

// npm may pack files without an exec bit. The binary needs it, and so do the musl loader and the
// real binary behind the musl launcher script, which are exec'd by relative path.
function ensureExecutable(binary) {
	if (os.type() === "Windows_NT") return;
	const dir = path.dirname(binary);
	const candidates = [binary, path.join(dir, `${BIN_NAME}.bin`)];
	try {
		for (const entry of fs.readdirSync(path.join(dir, "lib"))) {
			if (entry.startsWith("ld-musl-")) candidates.push(path.join(dir, "lib", entry));
		}
	} catch {}
	for (const file of candidates) {
		try {
			const { mode } = fs.statSync(file);
			if ((mode & 0o111) === 0) fs.chmodSync(file, mode | 0o755);
		} catch {}
	}
}

async function ensureBinary() {
	if (fs.existsSync(binPath) && isHealthy(binPath)) return;
	process.stderr.write(`${BIN_NAME}: binary missing or corrupt, attempting download...\n`);
	// Call main() explicitly rather than relying on import side-effects: ESM
	// caches modules, so the installer's top-level run is gated to direct
	// invocation only and would not fire on import.
	const { main: runInstall } = await import("../install.js");
	await runInstall();
}

function printUnavailable() {
	process.stderr.write(
		`${BIN_NAME} is not available for your platform yet. Install it with:\n` +
			`  brew install xberg-io/tap/xberg\n` +
			`  or use the Xberg plugin:  /plugin marketplace add xberg-io/xberg\n`,
	);
}

// On macOS one libmalloc magazine lowers the CLI's peak RSS and wall time (ADR 0022). libmalloc
// reads the variable before main, so only the launcher can set it; a value the user set is kept.
function nativeEnv() {
	if (process.platform !== "darwin" || Object.hasOwn(process.env, MALLOC_MAGAZINES)) return process.env;
	return { ...process.env, [MALLOC_MAGAZINES]: "1" };
}

async function main() {
	const bundled = resolveBundledBinary();
	if (bundled) ensureExecutable(bundled);
	const target = bundled && isHealthy(bundled) ? bundled : null;
	if (!target) {
		await ensureBinary();
		if (!fs.existsSync(binPath) || !isHealthy(binPath)) {
			printUnavailable();
			process.exit(1);
		}
	}
	const result = spawnSync(target ?? binPath, process.argv.slice(2), { stdio: "inherit", env: nativeEnv() });
	if (result.error) {
		process.stderr.write(`${BIN_NAME}: failed to spawn binary: ${result.error.message}\n`);
		process.exit(1);
	}
	process.exit(result.status ?? 0);
}

main().catch((err) => {
	// No standalone CLI for this platform: print the graceful install hint, not a stack.
	if (err && err.name === "CliUnavailableError") {
		printUnavailable();
		process.exit(1);
	}
	process.stderr.write(`${BIN_NAME}: ${err.message}\n`);
	process.exit(1);
});
