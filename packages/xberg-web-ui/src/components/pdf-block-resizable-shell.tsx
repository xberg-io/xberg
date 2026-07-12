"use client"

import * as React from "react"
import {
  useDefaultLayout,
  type GroupImperativeHandle,
  type LayoutStorage,
} from "react-resizable-panels"

import { cn } from "@/lib/utils"
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable"

type PdfBlockResizableShellProps = {
  autoSaveId: string
  className?: string
  heightClassName?: string
  initialOrientation?: React.ComponentProps<
    typeof ResizablePanelGroup
  >["orientation"]
  left: React.ReactNode
  leftDefaultSize?: number
  leftMinSize?: number
  right: React.ReactNode
  rightDefaultSize?: number
  rightMaxSize?: number
  rightMinSize?: number
}

function toPercentSize(size: number) {
  return `${size}%`
}

function getLayoutSize(
  layout: React.ComponentProps<typeof ResizablePanelGroup>["defaultLayout"],
  panelId: string,
  fallbackSize: number
) {
  const size = layout?.[panelId]

  return typeof size === "number" && Number.isFinite(size)
    ? size
    : fallbackSize
}

const HORIZONTAL_LAYOUT_MIN_WIDTH = 900

function useElementWidth<T extends HTMLElement>() {
  const [node, setNode] = React.useState<T | null>(null)
  const [width, setWidth] = React.useState<number>()

  React.useEffect(() => {
    if (!node || typeof ResizeObserver === "undefined") return

    const updateWidth = () => {
      const nextWidth = node.getBoundingClientRect().width

      setWidth((currentWidth) =>
        currentWidth !== undefined && Math.abs(currentWidth - nextWidth) < 0.5
          ? currentWidth
          : nextWidth
      )
    }

    updateWidth()

    const observer = new ResizeObserver(updateWidth)
    observer.observe(node)

    return () => observer.disconnect()
  }, [node])

  return [setNode, width] as const
}

function useMounted() {
  const [mounted, setMounted] = React.useState(false)

  React.useEffect(() => {
    setMounted(true)
  }, [])

  return mounted
}

function getSavedLayout(key: string) {
  if (typeof window === "undefined") return null

  try {
    return window.localStorage.getItem(key)
  } catch {
    return null
  }
}

function setSavedLayout(key: string, value: string) {
  if (typeof window === "undefined") return

  try {
    window.localStorage.setItem(key, value)
  } catch {
    // Ignore storage failures so previews still render in restricted contexts.
  }
}

function useHydrationSafeLayoutStorage() {
  const mounted = useMounted()

  return React.useMemo<LayoutStorage>(
    () => ({
      getItem(key) {
        return mounted ? getSavedLayout(key) : null
      },
      setItem(key, value) {
        if (mounted) setSavedLayout(key, value)
      },
    }),
    [mounted]
  )
}

export function PdfBlockResizableShell({
  autoSaveId,
  className,
  heightClassName = "h-[680px]",
  initialOrientation = "horizontal",
  left,
  leftDefaultSize,
  leftMinSize,
  right,
  rightDefaultSize = 34,
  rightMaxSize = 52,
  rightMinSize = 24,
}: PdfBlockResizableShellProps) {
  return (
    <PdfBlockResizableShellWithSavedLayout
      autoSaveId={autoSaveId}
      className={className}
      heightClassName={heightClassName}
      initialOrientation={initialOrientation}
      left={left}
      leftDefaultSize={leftDefaultSize}
      leftMinSize={leftMinSize}
      right={right}
      rightDefaultSize={rightDefaultSize}
      rightMaxSize={rightMaxSize}
      rightMinSize={rightMinSize}
    />
  )
}

function PdfBlockResizableShellWithSavedLayout({
  autoSaveId,
  className,
  heightClassName,
  initialOrientation = "horizontal",
  left,
  leftDefaultSize,
  leftMinSize,
  right,
  rightDefaultSize = 34,
  rightMaxSize = 52,
  rightMinSize = 24,
}: Required<Pick<PdfBlockResizableShellProps, "autoSaveId">> &
  Omit<PdfBlockResizableShellProps, "autoSaveId">) {
  const [containerRef, containerWidth] = useElementWidth<HTMLDivElement>()
  const layoutStorage = useHydrationSafeLayoutStorage()
  const isHorizontal =
    containerWidth === undefined
      ? initialOrientation === "horizontal"
      : containerWidth >= HORIZONTAL_LAYOUT_MIN_WIDTH
  const orientation = isHorizontal ? "horizontal" : "vertical"
  const resolvedLeftDefaultSize =
    leftDefaultSize ?? (isHorizontal ? 100 - rightDefaultSize : 62)
  const layoutId = `${autoSaveId}-${orientation}`
  const leftPanelId = `${layoutId}-left`
  const rightPanelId = `${layoutId}-right`
  const panelIds = React.useMemo(
    () => [leftPanelId, rightPanelId],
    [leftPanelId, rightPanelId]
  )
  const { defaultLayout, onLayoutChanged } = useDefaultLayout({
    id: layoutId,
    panelIds,
    storage: layoutStorage,
  })
  const resolvedRightDefaultSize = isHorizontal ? rightDefaultSize : 38

  return (
    <PdfBlockResizableShellLayout
      className={className}
      containerRef={containerRef}
      defaultLayout={defaultLayout}
      groupId={autoSaveId}
      heightClassName={heightClassName}
      left={left}
      leftDefaultSize={resolvedLeftDefaultSize}
      leftMinSize={leftMinSize}
      leftPanelId={leftPanelId}
      onLayoutChanged={onLayoutChanged}
      orientation={orientation}
      right={right}
      rightDefaultSize={resolvedRightDefaultSize}
      rightMaxSize={isHorizontal ? rightMaxSize : 66}
      rightMinSize={isHorizontal ? rightMinSize : 24}
      rightPanelId={rightPanelId}
      targetLayout={{
        key: `${layoutId}:${defaultLayout ? "saved" : "default"}`,
        leftSize: getLayoutSize(
          defaultLayout,
          leftPanelId,
          resolvedLeftDefaultSize
        ),
        rightSize: getLayoutSize(
          defaultLayout,
          rightPanelId,
          resolvedRightDefaultSize
        ),
      }}
    />
  )
}

function PdfBlockResizableShellLayout({
  className,
  containerRef,
  defaultLayout,
  groupId,
  heightClassName,
  left,
  leftDefaultSize,
  leftMinSize,
  leftPanelId,
  onLayoutChanged,
  orientation = "vertical",
  right,
  rightDefaultSize = 34,
  rightMaxSize = 52,
  rightMinSize = 24,
  rightPanelId,
  targetLayout,
}: Omit<PdfBlockResizableShellProps, "autoSaveId"> & {
  containerRef?: React.Ref<HTMLDivElement>
  defaultLayout?: React.ComponentProps<
    typeof ResizablePanelGroup
  >["defaultLayout"]
  groupId?: string
  leftPanelId?: string
  onLayoutChanged?: React.ComponentProps<
    typeof ResizablePanelGroup
  >["onLayoutChanged"]
  orientation?: React.ComponentProps<typeof ResizablePanelGroup>["orientation"]
  rightPanelId?: string
  targetLayout?: {
    key: string
    leftSize: number
    rightSize: number
  }
}) {
  const resolvedLeftDefaultSize = leftDefaultSize ?? 62
  const groupRef = React.useRef<GroupImperativeHandle | null>(null)
  const appliedTargetLayoutKeyRef = React.useRef<string | null>(null)

  React.useLayoutEffect(() => {
    if (!targetLayout || !leftPanelId || !rightPanelId) return

    const targetLayoutKey = `${targetLayout.key}:${targetLayout.leftSize}:${targetLayout.rightSize}`
    if (appliedTargetLayoutKeyRef.current === targetLayoutKey) return

    appliedTargetLayoutKeyRef.current = targetLayoutKey
    groupRef.current?.setLayout({
      [leftPanelId]: targetLayout.leftSize,
      [rightPanelId]: targetLayout.rightSize,
    })
  }, [leftPanelId, rightPanelId, targetLayout])

  return (
    <div
      ref={containerRef}
      className={cn(
        heightClassName,
        "relative min-h-[420px] overflow-hidden bg-background",
        className
      )}
    >
      <ResizablePanelGroup
        id={groupId}
        groupRef={groupRef}
        orientation={orientation}
        defaultLayout={defaultLayout}
        onLayoutChanged={onLayoutChanged}
        className="h-full min-h-0"
      >
        <ResizablePanel
          id={leftPanelId ?? "left"}
          defaultSize={toPercentSize(resolvedLeftDefaultSize)}
          minSize={toPercentSize(leftMinSize ?? 34)}
          className="min-h-0 min-w-0 overflow-hidden"
        >
          {left}
        </ResizablePanel>
        <ResizableHandle className="group z-10" withHandle />
        <ResizablePanel
          id={rightPanelId ?? "right"}
          defaultSize={toPercentSize(rightDefaultSize)}
          minSize={toPercentSize(rightMinSize)}
          maxSize={toPercentSize(rightMaxSize)}
          className="min-h-0 min-w-0 overflow-hidden"
        >
          {right}
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  )
}
