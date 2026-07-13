import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ReingestButton } from "../../src/components/ReingestButton.js";
import { EngineProvider } from "../../src/providers/EngineProvider.js";
import type { IngestHistoryEntry } from "../../src/lib/types.js";

function renderWithEngine(
  ui: React.ReactElement,
  fakeClient: { ingestFile: ReturnType<typeof vi.fn> },
) {
  return render(
    <EngineProvider baseUrl="http://x:8080" workerClient={fakeClient as never}>
      {ui}
    </EngineProvider>,
  );
}

async function pickFile(name: string) {
  fireEvent.change(screen.getByLabelText("Rehydration passphrase"), {
    target: { value: "pass1234" },
  });
  const file = new File([new Uint8Array([1])], name, {
    type: "application/pdf",
  });
  const input = document.querySelector('input[type="file"]') as HTMLInputElement;
  fireEvent.change(input, { target: { files: [file] } });
}

describe("ReingestButton", () => {
  it("re-ingests when the picked file's sanitized name matches expectedExternalId", async () => {
    const entry: IngestHistoryEntry = {
      collection: "c1",
      externalId: "contrat.pdf",
      filename: "contrat.pdf",
      mime: "application/pdf",
      redactedText: "hi",
      piiCategoryCounts: {},
      documentId: "doc-1",
      status: "synced",
      ingestedAt: 1,
    };
    const fakeClient = { ingestFile: vi.fn().mockResolvedValue(entry) };

    renderWithEngine(
      <ReingestButton collection="c1" expectedExternalId="contrat.pdf" />,
      fakeClient,
    );

    await pickFile("contrat.pdf");

    await waitFor(() =>
      expect(fakeClient.ingestFile).toHaveBeenCalledWith(
        expect.any(File),
        "c1",
        "pass1234",
      ),
    );
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("refuses to ingest a differently-named file instead of silently creating a new document", async () => {
    const fakeClient = { ingestFile: vi.fn() };

    renderWithEngine(
      <ReingestButton collection="c1" expectedExternalId="contrat.pdf" />,
      fakeClient,
    );

    await pickFile("invoice.pdf");

    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent(
        /would ingest as a new document/,
      ),
    );
    expect(fakeClient.ingestFile).not.toHaveBeenCalled();
  });
});
