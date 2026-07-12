"use client";
import { useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogFooter, DialogTrigger, DialogClose } from "@/components/ui/dialog.js";
import { Button } from "@/components/ui/button.js";
import { Input } from "@/components/ui/input.js";
import { postCollection } from "@/lib/sync-client.js";
import { sanitizeExternalId } from "@/lib/sanitize-id.js";
import { EMBEDDING_DIM } from "@/lib/constants.js";

export function CreateFolderDialog({ baseUrl, onCreated }: { baseUrl: string; onCreated: (name: string) => void }) {
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const create = async () => {
    const safeName = sanitizeExternalId(name.trim());
    setBusy(true);
    setError(null);
    try {
      await postCollection(baseUrl, { name: safeName, embedding_dim: EMBEDDING_DIM });
      onCreated(safeName);
      setName("");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog>
      <DialogTrigger>
        <Button>New folder</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New folder</DialogTitle>
        </DialogHeader>
        <label htmlFor="folder-name" className="text-sm font-medium">
          Folder name
        </label>
        <Input id="folder-name" value={name} onChange={(e) => setName(e.target.value)} />
        {error && (
          <p role="alert" className="mt-2 text-sm text-red-600">
            {error}
          </p>
        )}
        <DialogFooter>
          <DialogClose>
            <Button variant="outline" disabled={busy}>
              Cancel
            </Button>
          </DialogClose>
          <Button disabled={busy || name.trim().length === 0} onClick={create}>
            Create
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
