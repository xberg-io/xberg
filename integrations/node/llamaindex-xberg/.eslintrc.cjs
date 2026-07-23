/**
 * ESLint config for @xberg-io/llamaindex-xberg.
 *
 * Uses the TypeScript parser and the @typescript-eslint recommended rules for a
 * plain TypeScript library (no framework plugins).
 */
module.exports = {
  root: true,
  env: {
    node: true,
    es2022: true,
  },
  parser: "@typescript-eslint/parser",
  parserOptions: {
    ecmaVersion: 2022,
    sourceType: "module",
  },
  plugins: ["@typescript-eslint"],
  extends: ["eslint:recommended", "plugin:@typescript-eslint/recommended"],
  ignorePatterns: ["dist/**", "node_modules/**", ".eslintrc.cjs", "tsup.config.ts", "vitest.config.ts"],
  rules: {
    "@typescript-eslint/no-explicit-any": "error",
  },
};
