import { describe, it, expect } from "vitest";
import { rotatePointInPage } from "../../src/components/ui/pdf-viewer.js";

// A 100x50 (landscape) page. Verify the top-left corner (0,0) lands on the
// correct corner of the rotated frame for each quarter turn, and that a
// rotated dimension swap matches the corner it maps to.
describe("rotatePointInPage", () => {
  const width = 100;
  const height = 50;

  it("rotation 0: point unchanged", () => {
    expect(rotatePointInPage({ width, height, rotation: 0, x: 0, y: 0 })).toEqual({ x: 0, y: 0 });
    expect(rotatePointInPage({ width, height, rotation: 0, x: 30, y: 10 })).toEqual({ x: 30, y: 10 });
  });

  it("rotation 1 (90 clockwise): top-left moves to top-right of the rotated (height x width) frame", () => {
    // Rotated frame is height x width = 50 x 100.
    expect(rotatePointInPage({ width, height, rotation: 1, x: 0, y: 0 })).toEqual({ x: 50, y: 0 });
    // Bottom-left of the original (0, height) moves to top-left of the rotated frame.
    expect(rotatePointInPage({ width, height, rotation: 1, x: 0, y: height })).toEqual({ x: 0, y: 0 });
  });

  it("rotation 2 (180): top-left moves to bottom-right", () => {
    expect(rotatePointInPage({ width, height, rotation: 2, x: 0, y: 0 })).toEqual({ x: width, y: height });
  });

  it("rotation 3 (270 clockwise / 90 counter-clockwise): top-left moves to bottom-left of the rotated frame", () => {
    expect(rotatePointInPage({ width, height, rotation: 3, x: 0, y: 0 })).toEqual({ x: 0, y: width });
  });
});
