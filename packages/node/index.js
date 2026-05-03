// Re-export the NAPI-RS native module.
// The native binary is built into crates/kreuzberg-node; this package
// acts as the npm-distribution wrapper around it.
module.exports = require("../../crates/kreuzberg-node/index.js");
