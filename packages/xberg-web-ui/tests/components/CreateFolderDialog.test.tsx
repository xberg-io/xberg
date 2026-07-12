import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { CreateFolderDialog } from "../../src/components/CreateFolderDialog.js";
import { setAuthToken } from "../../src/lib/auth-client.js";

describe("CreateFolderDialog", () => {
  beforeEach(() => {
    setAuthToken("tok");
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ status: 200, ok: true, json: async () => ({ created: true }) })
    );
  });

  it("creates a folder and calls onCreated with the sanitized name", async () => {
    const onCreated = vi.fn();
    render(<CreateFolderDialog baseUrl="http://x:8080" onCreated={onCreated} />);

    fireEvent.click(screen.getByText("New folder"));
    fireEvent.change(screen.getByLabelText("Folder name"), { target: { value: "Dossier Client X" } });
    fireEvent.click(screen.getByText("Create"));

    await waitFor(() => expect(onCreated).toHaveBeenCalledWith("Dossier_Client_X"));
  });
});
