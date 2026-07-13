"use client";
import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogTrigger,
  DialogClose,
} from "@/components/ui/dialog.js";
import { Button } from "@/components/ui/button.js";
import { postAdmin } from "@/lib/admin-client.js";

export interface DeleteDialogProps {
  baseUrl: string;
  token: string;
  collection: string;
  externalIds: string[];
  onDeleted?: (externalIds: string[]) => void;
}

export function DeleteDialog({
  baseUrl,
  token,
  collection,
  externalIds,
  onDeleted,
}: DeleteDialogProps) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const count = externalIds.length;

  const remove = async () => {
    setBusy(true);
    setError(null);
    try {
      await postAdmin(baseUrl, token, {
        op: "delete_documents",
        collection,
        external_ids: externalIds,
      });
      onDeleted?.(externalIds);
      setOpen(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) setError(null);
      }}
    >
      <DialogTrigger asChild>
        <Button variant="destructive" disabled={count === 0}>
          Delete{count > 1 ? ` ${count}` : ""}
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete document{count > 1 ? "s" : ""}</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          Permanently delete {count === 1 ? "this document" : `${count} documents`} from{" "}
          <span className="font-mono">{collection}</span>? This cannot be undone.
        </p>
        {error && (
          <p role="alert" className="mt-2 text-sm text-destructive">
            {error}
          </p>
        )}
        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline" disabled={busy}>
              Cancel
            </Button>
          </DialogClose>
          <Button
            variant="destructive"
            disabled={busy || count === 0}
            aria-label="Confirm delete"
            onClick={remove}
          >
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
