"use client";
import { useEffect, useState } from "react";
import Link from "next/link";
import {
  useReactTable,
  getCoreRowModel,
  flexRender,
  createColumnHelper,
  type RowSelectionState,
} from "@tanstack/react-table";
import { listHistory } from "@/lib/ingest-history.js";
import { getAuthToken } from "@/lib/auth-client.js";
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from "@/components/ui/table.js";
import { Badge } from "@/components/ui/badge.js";
import { DeleteDialog } from "@/components/DeleteDialog.js";
import { ReingestButton } from "@/components/ReingestButton.js";
import type { IngestHistoryEntry } from "@/lib/types.js";

const MCP_BASE_URL = process.env.NEXT_PUBLIC_MCP_BASE_URL;

const columnHelper = createColumnHelper<IngestHistoryEntry>();
const columns = [
  columnHelper.display({
    id: "select",
    header: ({ table }) => (
      <input
        type="checkbox"
        aria-label="select-all"
        checked={table.getIsAllRowsSelected()}
        onChange={table.getToggleAllRowsSelectedHandler()}
      />
    ),
    cell: ({ row }) => (
      <input
        type="checkbox"
        aria-label={`select-${row.original.filename}`}
        checked={row.getIsSelected()}
        onChange={row.getToggleSelectedHandler()}
      />
    ),
  }),
  columnHelper.accessor("filename", {
    header: "Document",
    cell: (info) => (
      <Link
        className="text-slate-900 underline"
        href={`/document/${info.row.original.collection}/${info.row.original.externalId}`}
      >
        {info.getValue()}
      </Link>
    ),
  }),
  columnHelper.accessor("status", {
    header: "Status",
    cell: (info) => <Badge>{info.getValue()}</Badge>,
  }),
  columnHelper.accessor("piiCategoryCounts", {
    header: "PII",
    cell: (info) =>
      Object.entries(info.getValue())
        .map(([k, v]) => `${k}:${v}`)
        .join(", ") || "none",
  }),
];

export function DocumentTable({ collection }: { collection: string }) {
  const [rows, setRows] = useState<IngestHistoryEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());

  useEffect(() => {
    void listHistory(collection)
      .then(setRows)
      .catch((err) => {
        setError(err instanceof Error ? err.message : String(err));
      });
  }, [collection]);

  const rowSelection: RowSelectionState = Object.fromEntries(
    [...selected].map((id) => [id, true]),
  );

  const table = useReactTable({
    data: rows,
    columns,
    state: { rowSelection },
    getRowId: (row) => row.externalId,
    enableRowSelection: true,
    onRowSelectionChange: (updater) => {
      const next = typeof updater === "function" ? updater(rowSelection) : updater;
      setSelected(new Set(Object.keys(next).filter((k) => next[k])));
    },
    getCoreRowModel: getCoreRowModel(),
  });

  if (error)
    return (
      <p className="text-sm text-destructive">Failed to load documents: {error}</p>
    );
  if (rows.length === 0)
    return <p className="text-sm text-muted-foreground">No documents yet.</p>;

  const onDeleted = (ids: string[]) => {
    setRows((prev) => prev.filter((r) => !ids.includes(r.externalId)));
    setSelected(new Set());
  };

  return (
    <div className="space-y-3">
      <Table>
        <TableHeader>
          {table.getHeaderGroups().map((hg) => (
            <TableRow key={hg.id}>
              {hg.headers.map((h) => (
                <TableHead key={h.id}>
                  {flexRender(h.column.columnDef.header, h.getContext())}
                </TableHead>
              ))}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {table.getRowModel().rows.map((row) => (
            <TableRow key={row.id}>
              {row.getVisibleCells().map((cell) => (
                <TableCell key={cell.id}>
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>

      {selected.size > 0 ? (
        <div className="flex flex-wrap items-center gap-3 border bg-muted rounded p-3">
          <span className="text-sm text-muted-foreground">
            {selected.size} selected
          </span>
          <DeleteDialog
            baseUrl={
              MCP_BASE_URL ??
              (typeof window !== "undefined"
                ? window.location.origin
                : "http://127.0.0.1:8080")
            }
            token={getAuthToken() ?? ""}
            collection={collection}
            externalIds={[...selected]}
            onDeleted={onDeleted}
          />
          {selected.size === 1 ? (
            <ReingestButton
              collection={collection}
              expectedExternalId={[...selected][0] ?? ""}
            />
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
