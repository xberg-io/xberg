/**
 * Re-derive a dynamic route param from the real browser URL.
 *
 * Static export (`output: "export"`) generates exactly one file per dynamic
 * route -- a `placeholder` shell -- and mcp-server's static file server
 * (mcp-server/src/http/ui-route-resolver.ts) serves that same file for
 * every matching real request (e.g. `/folder/<any-name>` all resolve to
 * `folder/placeholder.html`). That file's embedded Next.js router state
 * says the route is `/folder/placeholder`, so `useParams()` returns
 * `{collection: "placeholder"}` regardless of what's actually in the
 * address bar on a hard navigation to a URL Next itself never generated --
 * confirmed live: `window.location.pathname` reports the real path while
 * `useParams()` does not. Parsing `window.location.pathname` directly is
 * the only way to recover the real value.
 *
 * `typeof window === "undefined"` guards the one call site that matters
 * here at all: static generation (`next build`) invokes this module in a
 * Node context with no `window`, and must not throw.
 */
function pathSegments(): string[] | null {
	if (typeof window === "undefined") return null;
	const basePath = "/ui";
	let pathname = window.location.pathname;
	if (pathname.startsWith(basePath)) pathname = pathname.slice(basePath.length);
	return pathname.split("/").filter(Boolean);
}

/** Real value of a `/<routeSegment>/<value>` route's dynamic segment, or `null`. */
export function collectionFromPathname(routeSegment: string): string | null {
	const segments = pathSegments();
	if (!segments) return null;
	const i = segments.indexOf(routeSegment);
	return i >= 0 ? (segments[i + 1] ?? null) : null;
}

/** Real `{collection, id}` from a `/document/<collection>/<id>` route, or `null` for either. */
export function documentParamsFromPathname(): { collection: string | null; id: string | null } {
	const segments = pathSegments();
	if (!segments) return { collection: null, id: null };
	const i = segments.indexOf("document");
	if (i < 0) return { collection: null, id: null };
	return { collection: segments[i + 1] ?? null, id: segments[i + 2] ?? null };
}
