"use client";
import { useEngine } from "@/providers/EngineProvider.js";
import { Badge } from "@/components/ui/badge.js";

export function SyncBar() {
  const { pendingCount, lastError } = useEngine();
  return (
    <div className="flex items-center justify-end gap-2 border-b border-slate-200 px-4 py-2 text-sm" aria-live="polite">
      {lastError && (
        <Badge className="bg-red-100 text-red-700" role="alert">
          {lastError}
        </Badge>
      )}
      {pendingCount > 0 ? (
        <Badge className="bg-amber-100 text-amber-700">Syncing {pendingCount}…</Badge>
      ) : (
        !lastError && <span className="text-slate-500">All synced</span>
      )}
    </div>
  );
}
