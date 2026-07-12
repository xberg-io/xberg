"use client"

import * as React from "react"

import {
  ATTENTION_OCR_OUTPUT,
  blockToHighlightArea,
  getOcrBlocks,
  OcrBlockOverlay,
  OcrBlocksPanel,
  PDF_URL,
  type OcrBlock,
  type ParsedOcrOutput,
} from "@/components/ui/layout-blocks"
import { PDFViewer, type PDFViewerHandle } from "@/components/ui/pdf-viewer"
import { PdfBlockResizableShell } from "@/components/pdf-block-resizable-shell"

export function OcrBlocksBlock({
  file = PDF_URL,
  output = ATTENTION_OCR_OUTPUT,
}: {
  file?: string
  output?: ParsedOcrOutput
}) {
  const blocks = React.useMemo(() => getOcrBlocks(output), [output])
  const [activeBlockId, setActiveBlockId] = React.useState(blocks[0]?.id)
  const viewerRef = React.useRef<PDFViewerHandle>(null)
  const activeBlockIdRef = React.useRef(activeBlockId)
  const activeBlock = blocks.find((block) => block.id === activeBlockId)

  React.useEffect(() => {
    activeBlockIdRef.current = activeBlockId
  }, [activeBlockId])

  React.useEffect(() => {
    if (!blocks.length || blocks.some((block) => block.id === activeBlockId)) {
      return
    }

    setActiveBlockId(blocks[0].id)
  }, [activeBlockId, blocks])

  const focusBlock = React.useCallback((block: OcrBlock) => {
    if (block.id === activeBlockIdRef.current) return

    const area = blockToHighlightArea(block)

    activeBlockIdRef.current = block.id
    setActiveBlockId(block.id)
    viewerRef.current?.scrollToPageArea(
      block.page,
      {
        left: Number.parseFloat(String(area.left)),
        top: Number.parseFloat(String(area.top)),
        width: Number.parseFloat(String(area.width)),
        height: Number.parseFloat(String(area.height)),
      },
      { behavior: "auto" }
    )
  }, [])

  const activeOverlayBlockId = activeBlock?.id
  const renderPageOverlay = React.useCallback(
    ({
      pageHeight,
      pageNumber,
      pageWidth,
    }: {
      pageHeight: number
      pageNumber: number
      pageWidth: number
    }) =>
      blocks
        .filter((block) => block.page === pageNumber)
        .map((block) => (
          <OcrBlockOverlay
            key={block.id}
            block={block}
            isActive={block.id === activeOverlayBlockId}
            pageHeight={pageHeight}
            pageWidth={pageWidth}
          />
        )),
    [activeOverlayBlockId, blocks]
  )

  return (
    <PdfBlockResizableShell
      autoSaveId="pdf-block-ocr"
      left={
        <PDFViewer
          ref={viewerRef}
          src={file}
          defaultZoom={1}
          renderPageOverlay={renderPageOverlay}
        />
      }
      right={
        <OcrBlocksPanel
          activeBlockId={activeBlock?.id}
          blocks={blocks}
          className="h-full"
          onBlockFocus={focusBlock}
        />
      }
    />
  )
}
