import "./globals.css";
import type { ReactNode } from "react";
import { EngineProvider } from "@/providers/EngineProvider.js";
import { SyncBar } from "@/components/SyncBar.js";

const MCP_BASE_URL = process.env.NEXT_PUBLIC_MCP_BASE_URL;

export const metadata = { title: "Xberg" };

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <body>
        <EngineProvider baseUrl={MCP_BASE_URL ?? undefined}>
          <SyncBar />
          {children}
        </EngineProvider>
      </body>
    </html>
  );
}
