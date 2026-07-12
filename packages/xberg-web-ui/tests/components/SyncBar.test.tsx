import { describe, it, expect, vi } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { SyncBar } from "../../src/components/SyncBar.js";
import { EngineProvider, useEngine } from "../../src/providers/EngineProvider.js";

function Trigger() {
  const { ingestFile } = useEngine();
  return (
    <button onClick={() => void ingestFile(new File(["x"], "x.txt"), "collection", "pw").catch(() => {})}>
      go
    </button>
  );
}

describe("SyncBar", () => {
  it("shows 'All synced' when nothing is pending and there is no error", () => {
    const fakeClient = { ingestFile: vi.fn() };
    render(
      <EngineProvider baseUrl="http://x:8080" workerClient={fakeClient as never}>
        <SyncBar />
      </EngineProvider>
    );
    expect(screen.getByText("All synced")).toBeDefined();
  });

  it("renders an error badge with role='alert' after a failed ingest", async () => {
    const fakeClient = {
      ingestFile: vi.fn().mockRejectedValue(new Error("Mock ingest error")),
    };

    render(
      <EngineProvider baseUrl="http://x:8080" workerClient={fakeClient as never}>
        <SyncBar />
        <Trigger />
      </EngineProvider>
    );

    await act(async () => {
      screen.getByText("go").click();
    });

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("Mock ingest error");
    });
  });

  it("shows a 'Syncing N…' badge while an ingest is pending", async () => {
    const fakeClient = {
      ingestFile: vi.fn(() => new Promise(() => {})), // never resolves
    };

    render(
      <EngineProvider baseUrl="http://x:8080" workerClient={fakeClient as never}>
        <SyncBar />
        <Trigger />
      </EngineProvider>
    );

    await act(async () => {
      screen.getByText("go").click();
    });

    await waitFor(() => {
      expect(screen.getByText("Syncing 1…")).toBeDefined();
    });
  });
});
