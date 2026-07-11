import { pipeline } from "@huggingface/transformers";

const modelId = process.argv[2] ?? "Xenova/bge-m3";
const configs = [
	{ device: "cpu", dtype: "q8" },
	{ device: "wasm", dtype: "q8" },
	{ device: "webgpu", dtype: "fp32" },
];

let hadFailure = false;

for (const config of configs) {
	console.log(`\n--- ${modelId} device=${config.device} dtype=${config.dtype} ---`);
	const start = performance.now();
	try {
		const extractor = await pipeline("feature-extraction", modelId, config);
		const output = await extractor(["hello world"], { pooling: "mean", normalize: false });
		const elapsedMs = Math.round(performance.now() - start);
		console.log(`OK: dims=[${output.dims.join(", ")}] loaded in ${elapsedMs}ms`);
	} catch (error) {
		console.error(`FAILED: ${error instanceof Error ? error.message : String(error)}`);
		hadFailure = true;
	}
}

if (hadFailure) {
	process.exitCode = 1;
}
