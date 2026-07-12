"use client";
import { useEffect, useState } from "react";
import Link from "next/link";
import { useReactTable, getCoreRowModel, flexRender, createColumnHelper } from "@tanstack/react-table";
import { listHistory } from "@/lib/ingest-history.js";
import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from "@/components/ui/table.js";
import { Badge } from "@/components/ui/badge.js";
import type { IngestHistoryEntry } from "@/lib/types.js";

const columnHelper = createColumnHelper<IngestHistoryEntry>();
const columns = [
  columnHelper.accessor("filename", {
    header: "Document",
    cell: (info) => (
      <Link className="text-slate-900 underline" href={`/document/${info.row.original.collection}/${info.row.original.externalId}`}>
        {info.getValue()}
      </Link>
    ),
  }),
  columnHelper.accessor("status", { header: "Status", cell: (info) => <Badge>{info.getValue()}</Badge> }),
  columnHelper.accessor("piiCategoryCounts", {
    header: "PII",
    cell: (info) => Object.entries(info.getValue()).map(([k, v]) => `${k}:${v}`).join(", ") || "none",
  }),
];

export function DocumentTable({ collection }: { collection: string }) {
  const [rows, setRows] = useState<IngestHistoryEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void listHistory(collection)
      .then((data) => {
        if (!cancelled) {
          setRows(data);
          setError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [collection]);

  const table = useReactTable({ data: rows, columns, getCoreRowModel: getCoreRowModel() });

  if (error) return <p className="text-sm text-red-600">Failed to load documents: {error}</p>;
  if (loading) return <p className="text-sm text-slate-500">Loading…</p>;
  if (rows.length === 0) return <p className="text-sm text-slate-500">No documents yet.</p>;

  return (
    <Table>
      <TableHeader>
        {table.getHeaderGroups().map((hg) => (
          <TableRow key={hg.id}>
            {hg.headers.map((h) => (
              <TableHead key={h.id}>{flexRender(h.column.columnDef.header, h.getContext())}</TableHead>
            ))}
          </TableRow>
        ))}
      </TableHeader>
      <TableBody>
        {table.getRowModel().rows.map((row) => (
          <TableRow key={row.id}>
            {row.getVisibleCells().map((cell) => (
              <TableCell key={cell.id}>{flexRender(cell.column.columnDef.cell, cell.getContext())}</TableCell>
            ))}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
