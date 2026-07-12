import * as React from "react"

import {
  findReviewField,
  getMetadataLocation,
  getReviewFieldLocation,
  getReviewLocationKey,
  HumanReviewHighlight,
  HumanReviewPanel,
  REVIEW_FIELDS,
  type HumanReviewTheme,
  type JsonValue,
  type ReviewField,
  type ReviewLocation,
  type ReviewMetadataEntry,
} from "@/components/ui/bounding-box-citations"
import { PDFViewer, type PDFViewerHandle } from "@/components/ui/pdf-viewer"
import { PdfBlockResizableShell } from "@/components/pdf-block-resizable-shell"

const DEFAULT_ZOOM = 1

export function HumanReviewBlock({
  file,
  fields = REVIEW_FIELDS,
  className,
  metadata,
  resolveArrayItemMetadataPath,
  resolveLocation,
  showExpected = true,
  theme,
}: {
  file?: string
  fields?: ReviewField[]
  className?: string
  metadata?: Record<string, ReviewMetadataEntry>
  resolveArrayItemMetadataPath?: (
    metadataPath: string,
    rowIndex: number,
    rowValue: JsonValue
  ) => string | undefined
  resolveLocation?: (metadataPath: string) => ReviewLocation | undefined
  showExpected?: boolean
  theme?: HumanReviewTheme
}) {
  const [activeFieldKey, setActiveFieldKey] = React.useState(fields[0]?.key)
  const [hoverLocation, setHoverLocation] =
    React.useState<ReviewLocation | null>(null)
  const viewerRef = React.useRef<PDFViewerHandle>(null)
  const hoverLocationKeyRef = React.useRef<string | null>(null)
  const resolveFieldLocation = React.useCallback(
    (metadataPath: string) =>
      resolveLocation?.(metadataPath) ??
      getMetadataLocation(metadata, metadataPath),
    [metadata, resolveLocation]
  )
  const activeField = findReviewField(fields, activeFieldKey) ?? fields[0]
  const activeLocation =
    hoverLocation ?? getReviewFieldLocation(activeField, resolveFieldLocation)

  React.useEffect(() => {
    if (activeFieldKey || !fields[0]) return
    setActiveFieldKey(fields[0].key)
  }, [activeFieldKey, fields])

  const focusField = React.useCallback(
    (field: ReviewField) => {
      if (field.key === activeFieldKey) return

      setActiveFieldKey(field.key)

      const location = getReviewFieldLocation(field, resolveFieldLocation)
      if (location) {
        viewerRef.current?.scrollToPageArea(location.page, location.area)
      }
    },
    [activeFieldKey, resolveFieldLocation]
  )
  const handleLocationHover = React.useCallback((location?: ReviewLocation) => {
    setHoverLocation(location ?? null)

    const locationKey = getReviewLocationKey(location)
    if (!location) {
      hoverLocationKeyRef.current = null
      return
    }
    if (locationKey === hoverLocationKeyRef.current) return

    hoverLocationKeyRef.current = locationKey
    viewerRef.current?.scrollToPageArea(location.page, location.area, {
      behavior: "auto",
    })
  }, [])

  return (
    <PdfBlockResizableShell
      autoSaveId="pdf-block-bounding-box-citations"
      className={className}
      rightDefaultSize={42}
      rightMaxSize={60}
      rightMinSize={30}
      left={
        <PDFViewer
          ref={viewerRef}
          src={file}
          defaultZoom={DEFAULT_ZOOM}
          renderPageOverlay={({ pageNumber }) =>
            activeLocation?.page === pageNumber ? (
              <HumanReviewHighlight location={activeLocation} />
            ) : null
          }
        />
      }
      right={
        <aside className="min-h-0 bg-background">
          <HumanReviewPanel
            fields={fields}
            activeFieldKey={activeField?.key}
            className="h-full min-h-0"
            theme={theme}
            showExpected={showExpected}
            onFieldFocus={focusField}
            onLocationHover={handleLocationHover}
            resolveArrayItemMetadataPath={resolveArrayItemMetadataPath}
            resolveLocation={resolveFieldLocation}
          />
        </aside>
      }
    />
  )
}
