// src/providers/EngineProvider.tsx
"use client";
import { createContext, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { WorkerClient } from "../engine/worker-client.js";
import { captureAuthTokenFromLocation } from "../lib/auth-client.js";
import { putHistoryEntry } from "../lib/ingest-history.js";
import type { IngestHistoryEntry } from "../lib/types.js";

interface EngineApi {
  ready: boolean;
  pendingCount: number;
  lastError: string | null;
  ingestFile(file: File, collection: string, passphrase: string): Promise<IngestHistoryEntry>;
}

const EngineContext = createContext<EngineApi | null>(null);

export function useEngine(): EngineApi {
  const api = useContext(EngineContext);
  if (!api) throw new Error("useEngine() must be called inside an <EngineProvider>");
  return api;
}

interface EngineProviderProps {
  baseUrl?: string;
  children: ReactNode;
  /** Test-only escape hatch — production callers never pass this. */
  workerClient?: Pick<WorkerClient, "ingestFile" | "dispose">;
}

export function EngineProvider({ baseUrl: baseProp, children, workerClient }: EngineProviderProps) {
  const clientRef = useRef<Pick<WorkerClient, "ingestFile" | "dispose"> | null>(workerClient ?? null);
  const [ready, setReady] = useState(Boolean(workerClient));
  const [pendingCount, setPendingCount] = useState(0);
  const [lastError, setLastError] = useState<string | null>(null);

  useEffect(() => {
    captureAuthTokenFromLocation();
    if (clientRef.current) return;
    const baseUrl = baseProp ?? (typeof window !== "undefined" ? window.location.origin : "http://127.0.0.1:8080");
    const worker = new Worker(new URL("../engine/engine.worker.ts", import.meta.url), { type: "module" });
    clientRef.current = new WorkerClient(worker, baseUrl);
    setReady(true);
    return () => {
      clientRef.current?.dispose?.();
      clientRef.current = null;
      setReady(false);
      worker.terminate();
    };
  }, [baseProp]);

  const api = useMemo<EngineApi>(
    () => ({
      ready,
      pendingCount,
      lastError,
      async ingestFile(file, collection, passphrase) {
        if (!clientRef.current) throw new Error("engine worker not ready yet");
        setPendingCount((n) => n + 1);
        setLastError(null);
        try {
          const entry = await clientRef.current.ingestFile(file, collection, passphrase);
          setLastError(null);
          try {
            await putHistoryEntry(entry);
          } catch (persistErr) {
            console.error("Failed to persist ingest history:", persistErr instanceof Error ? persistErr.message : String(persistErr));
          }
          return entry;
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          setLastError(message);
          throw err;
        } finally {
          setPendingCount((n) => n - 1);
        }
      },
    }),
    [ready, pendingCount, lastError]
  );

  return <EngineContext.Provider value={api}>{children}</EngineContext.Provider>;
}
