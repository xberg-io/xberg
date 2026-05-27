import { extractFile } from "./dist/index.js";

const result = await extractFile("images/sample.png", undefined, { disableOcr: true });
console.log("metadata.format:", result.metadata.format);
console.log("Full metadata:", JSON.stringify(result.metadata, null, 2));
