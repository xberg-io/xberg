/**
 * ESLint config for the Xberg n8n community node.
 *
 * Uses eslint-plugin-n8n-nodes-base to enforce n8n's node-authoring
 * conventions (parameter ordering, display names, icon references, and
 * community-package metadata in package.json).
 */
module.exports = {
  root: true,
  env: {
    node: true,
    es2022: true,
  },
  parser: "@typescript-eslint/parser",
  parserOptions: {
    sourceType: "module",
    extraFileExtensions: [".json"],
  },
  ignorePatterns: ["dist/**", "node_modules/**", ".eslintrc.js", "gulpfile.js", "index.js"],
  overrides: [
    {
      files: ["package.json"],
      plugins: ["eslint-plugin-n8n-nodes-base"],
      extends: ["plugin:n8n-nodes-base/community"],
    },
    {
      files: ["./nodes/**/*.ts"],
      plugins: ["eslint-plugin-n8n-nodes-base"],
      extends: ["plugin:n8n-nodes-base/nodes"],
      rules: {
        // eslint-plugin-n8n-nodes-base@1.16.x predates the `NodeConnectionType`
        // -> `NodeConnectionTypes` rename and flags the modern typed constant,
        // demanding the legacy `['main']` string form. The current official
        // n8n-nodes-starter uses `NodeConnectionTypes.Main`, so we keep the
        // typed constant and silence the outdated rules. ~keep
        "n8n-nodes-base/node-class-description-inputs-wrong-regular-node": "off",
        "n8n-nodes-base/node-class-description-outputs-wrong": "off",
      },
    },
  ],
};
