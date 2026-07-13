import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { DeleteDialog } from "../../src/components/DeleteDialog.js";

vi.mock("@/lib/admin-client.js", () => ({
  postAdmin: vi.fn(),
}));

import { postAdmin } from "@/lib/admin-client.js";

describe("DeleteDialog", () => {
  it("disables the trigger when no documents are selected", () => {
    render(
      <DeleteDialog baseUrl="http://x:8080" token="tok" collection="c1" externalIds={[]} />
    );
    expect(screen.getByRole("button", { name: "Delete" })).toBeDisabled();
  });

  it("posts delete_documents with the external ids and notifies onDeleted", async () => {
    const onDeleted = vi.fn();
    vi.mocked(postAdmin).mockResolvedValue({ deleted: 1 });

    render(
      <DeleteDialog
        baseUrl="http://x:8080"
        token="tok"
        collection="c1"
        externalIds={["a.pdf", "b.pdf"]}
        onDeleted={onDeleted}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "Delete 2" }));
    fireEvent.click(screen.getByRole("button", { name: "Confirm delete" }));

    await waitFor(() =>
      expect(postAdmin).toHaveBeenCalledWith("http://x:8080", "tok", {
        op: "delete_documents",
        collection: "c1",
        external_ids: ["a.pdf", "b.pdf"],
      })
    );
    await waitFor(() => expect(onDeleted).toHaveBeenCalledWith(["a.pdf", "b.pdf"]));
  });

  it("surfaces the error and does not call onDeleted on failure", async () => {
    const onDeleted = vi.fn();
    vi.mocked(postAdmin).mockRejectedValue(new Error("collection not found: c1"));

    render(
      <DeleteDialog
        baseUrl="http://x:8080"
        token="tok"
        collection="c1"
        externalIds={["a.pdf"]}
        onDeleted={onDeleted}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    fireEvent.click(screen.getByRole("button", { name: "Confirm delete" }));

    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent(/collection not found/));
    expect(onDeleted).not.toHaveBeenCalled();
  });
});
