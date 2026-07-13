import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { LayoutBlocks } from "../../src/components/LayoutBlocks.js";

describe("LayoutBlocks", () => {
  it("renders one region per OCR line", () => {
    render(
      <LayoutBlocks
        lines={[
          { text: "Hello", confidence: 0.95, bbox: { x: 10, y: 20, w: 100, h: 30 } },
          { text: "World", confidence: 0.6 },
        ]}
        width={200}
        height={120}
      />
    );
    expect(screen.getAllByTestId("layout-block")).toHaveLength(2);
  });
});
