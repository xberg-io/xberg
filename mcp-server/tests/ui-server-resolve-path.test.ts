import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { resolveUiSubPath } from "../src/http/ui-route-resolver.js";

describe("http/ui-server resolveUiSubPath", () => {
  let dir: string;

  beforeAll(() => {
    dir = mkdtempSync(join(tmpdir(), "xberg-ui-resolve-test-"));
    mkdirSync(join(dir, "document", "placeholder", "placeholder"), { recursive: true });
    writeFileSync(join(dir, "document", "placeholder", "placeholder", "index.html"), "<html>doc shell</html>");
    mkdirSync(join(dir, "document", "real-collection", "real-id"), { recursive: true });
    writeFileSync(join(dir, "document", "real-collection", "real-id", "index.html"), "<html>real doc</html>");
    mkdirSync(join(dir, "folder", "placeholder"), { recursive: true });
    writeFileSync(join(dir, "folder", "placeholder", "index.html"), "<html>folder shell</html>");
  });

  afterAll(() => {
    rmSync(dir, { recursive: true, force: true });
  });

  it("passes through a document path that was actually exported", () => {
    expect(resolveUiSubPath(dir, "/document/real-collection/real-id/")).toBe(
      "/document/real-collection/real-id/",
    );
  });

  it("falls back to the placeholder shell for a document path that wasn't exported", () => {
    expect(resolveUiSubPath(dir, "/document/unknown-collection/unknown-id/")).toBe(
      "/document/placeholder/placeholder/",
    );
  });

  it("falls back to the placeholder shell for a folder path that wasn't exported", () => {
    expect(resolveUiSubPath(dir, "/folder/unknown-collection")).toBe("/folder/placeholder/");
  });

  it("strips query strings before matching", () => {
    expect(resolveUiSubPath(dir, "/document/unknown/unknown/?token=abc")).toBe(
      "/document/placeholder/placeholder/",
    );
  });

  it("leaves unrelated paths untouched", () => {
    expect(resolveUiSubPath(dir, "/app.js")).toBe("/app.js");
    expect(resolveUiSubPath(dir, "/")).toBe("/");
  });
});
