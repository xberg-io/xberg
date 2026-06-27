import { extract } from "./dist/index.js";

const output = await extract({ kind: "uri", uri: "images/sample.png" }, { disableOcr: true });
const result = output.results[0];
console.log("metadata.format:", result.metadata.format);
console.log("Full metadata:", JSON.stringify(result.metadata, null, 2));
