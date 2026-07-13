import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { BoundingBoxCitations } from "../../src/components/BoundingBoxCitations.js";

describe("BoundingBoxCitations", () => {
  it("highlights PII tokens and lists counts without leaking clear values", () => {
    render(
      <BoundingBoxCitations
        redactedText="Contact [EMAIL_1] about [PERSON_1]"
        map={{ "[EMAIL_1]": "alice@example.com", "[PERSON_1]": "Alice" }}
        counts={{ EMAIL: 1, NAME: 1 }}
      />,
    );
    expect(screen.getByText("[EMAIL_1]")).toBeDefined();
    expect(screen.getByText("EMAIL")).toBeDefined();
    expect(screen.queryByText("alice@example.com")).toBeNull();
    expect(screen.queryByText("Alice")).toBeNull();
  });
});
