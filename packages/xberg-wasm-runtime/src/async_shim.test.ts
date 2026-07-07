import { describe, it, expect } from "vitest";
import { SingleFlightGuard } from "./async_shim";

describe("single-flight guard", () => {
  it("prevents concurrent calls on the same instance", async () => {
    const guard = new SingleFlightGuard("test-engine");

    const p1 = guard.run(async () => {
      await new Promise((r) => setTimeout(r, 10));
      return "result1";
    });

    // Trying to run concurrently should fail
    const p2 = guard.run(async () => "result2").catch((e) => e.message);

    const [r1, r2] = await Promise.all([p1, p2]);
    expect(r1).toBe("result1");
    expect(r2).toContain("single-flight violation");
  });

  it("allows sequential calls", async () => {
    const guard = new SingleFlightGuard("test-engine");

    const r1 = await guard.run(async () => "first");
    const r2 = await guard.run(async () => "second");

    expect(r1).toBe("first");
    expect(r2).toBe("second");
  });
});
