import { FolderPageClient } from "./FolderPageClient.js";

// Required by `output: "export"` for dynamic route segments: Next.js needs
// at least one static param set at build time to produce an HTML+JS shell.
// Collection names are created at runtime and unknowable at build time; this
// page's actual UI is 100% client-side, so the shell just needs to exist —
// the client router hydrates it with the real URL's params.
//
// This file MUST stay a server component (no "use client") — Next.js's
// static export rejects a page that both exports `generateStaticParams` and
// is a client component. The client-side logic lives in `FolderPageClient`.
export function generateStaticParams() {
  return [{ collection: "placeholder" }];
}

export default function FolderPage({
  params,
}: {
  params: { collection: string };
}) {
  return <FolderPageClient collection={params.collection} />;
}
