#!/usr/bin/env node
// Launcher: exec the downloaded native kreuzberg binary, forwarding argv and
// inheriting stdio. If the binary is missing (postinstall failed), download it
// on demand before exec.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const BIN_NAME = "kreuzberg";

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

async function ensureBinary() {
	if (fs.existsSync(binPath) && isHealthy(binPath)) return;
	process.stderr.write(`${BIN_NAME}: binary missing or corrupt, attempting download...\n`);
	// Call main() explicitly rather than relying on import side-effects: ESM
	// caches modules, so the installer's top-level run is gated to direct
	// invocation only and would not fire on import.
	const { main } = await import("../install.js");
	await main();
}

function printUnavailable() {
	process.stderr.write(
		`${BIN_NAME} is not available for your platform yet. Install it with:\n` +
			`  brew install kreuzberg-dev/tap/kreuzberg\n` +
			`  or use the Kreuzberg plugin:  /plugin marketplace add kreuzberg-dev/plugins\n`,
	);
}

async function main() {
	await ensureBinary();
	if (!fs.existsSync(binPath) || !isHealthy(binPath)) {
		printUnavailable();
		process.exit(1);
	}
	const result = spawnSync(binPath, process.argv.slice(2), { stdio: "inherit" });
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
