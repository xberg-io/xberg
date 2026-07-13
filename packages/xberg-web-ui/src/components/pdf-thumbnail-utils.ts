import type { PdfDocumentObject, PdfEngine } from "@embedpdf/models"

const PDFIUM_VERSION = "2.14.4"
const PDFIUM_WASM_URL = `https://cdn.jsdelivr.net/npm/@embedpdf/pdfium@${PDFIUM_VERSION}/dist/pdfium.wasm`

let sharedEnginePromise: Promise<PdfEngine> | null = null
const pdfDocumentCache = new Map<string, Promise<PdfDocumentObject>>()
const thumbnailUrlCache = new Map<string, Promise<string | null>>()

export function loadSharedPdfEngine() {
  sharedEnginePromise ??= import("@embedpdf/engines/pdfium-worker-engine").then(
    ({ createPdfiumEngine }) => createPdfiumEngine(PDFIUM_WASM_URL, {})
  )

  return sharedEnginePromise
}

export async function loadPdfDocument(url: string) {
  let documentPromise = pdfDocumentCache.get(url)

  if (!documentPromise) {
    documentPromise = loadSharedPdfEngine()
      .then((engine) =>
        engine
          .openDocumentUrl(
            { id: url, url },
            { mode: url.startsWith("blob:") ? "full-fetch" : "auto" }
          )
          .toPromise()
      )
      .catch((err) => {
        // Don't let a failed load poison the cache forever — the next
        // caller should get a fresh attempt.
        pdfDocumentCache.delete(url)
        throw err
      })
    pdfDocumentCache.set(url, documentPromise)
  }

  return documentPromise
}

export async function getPdfPageCount(url: string) {
  return (await loadPdfDocument(url)).pageCount
}

export function renderPdfThumbnailUrl({
  dpr = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1,
  pageIndex,
  url,
  width,
}: {
  dpr?: number
  pageIndex: number
  url: string
  width: number
}) {
  const cacheKey = `${url}#${pageIndex}@${width}x${dpr}`
  let thumbnailPromise = thumbnailUrlCache.get(cacheKey)

  if (!thumbnailPromise) {
    thumbnailPromise = (async () => {
      const [engine, document] = await Promise.all([
        loadSharedPdfEngine(),
        loadPdfDocument(url),
      ])
      const page = document.pages[pageIndex]

      if (!page) return null

      const blob = await engine
        .renderThumbnail(document, page, {
          dpr,
          imageType: "image/png",
          scaleFactor: width / page.size.width,
          withAnnotations: true,
        })
        .toPromise()

      return URL.createObjectURL(blob)
    })().catch((err) => {
      // Same reasoning as loadPdfDocument: a failed render shouldn't
      // permanently block retries for this page/size.
      thumbnailUrlCache.delete(cacheKey)
      throw err
    })
    thumbnailUrlCache.set(cacheKey, thumbnailPromise)
  }

  return thumbnailPromise
}

/**
 * Release everything cached for `url`: closes the underlying WASM document
 * handle via `engine.closeDocument` and revokes every thumbnail `blob:` URL
 * generated for it. Call this when a document is no longer displayed
 * (e.g. the viewer unmounts or the file is replaced) — the caches above
 * never evict on their own.
 */
export async function releasePdfDocument(url: string) {
  const documentPromise = pdfDocumentCache.get(url)
  pdfDocumentCache.delete(url)

  const thumbnailEntries = [...thumbnailUrlCache.entries()].filter(([key]) =>
    key.startsWith(`${url}#`)
  )
  for (const [key] of thumbnailEntries) thumbnailUrlCache.delete(key)

  await Promise.all(
    thumbnailEntries.map(async ([, promise]) => {
      try {
        const thumbnailUrl = await promise
        if (thumbnailUrl) URL.revokeObjectURL(thumbnailUrl)
      } catch {
        // Failed render, nothing to revoke.
      }
    })
  )

  if (!documentPromise) return
  try {
    const [engine, document] = await Promise.all([loadSharedPdfEngine(), documentPromise])
    await engine.closeDocument(document).toPromise()
  } catch {
    // Load already failed or engine unavailable — nothing left to close.
  }
}
