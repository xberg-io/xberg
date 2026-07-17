"use client"

import * as React from "react"
import * as ScrollAreaPrimitive from "@radix-ui/react-scroll-area"

import { cn } from "@/lib/utils"

export interface ScrollAreaProps
  extends React.ComponentPropsWithoutRef<typeof ScrollAreaPrimitive.Root> {
  /** Which scrollbar(s) to render. Defaults to "vertical" (prior behavior). */
  orientation?: "vertical" | "horizontal" | "both"
  /** Cosmetic edge fade over the scrollable content. */
  scrollFade?: boolean
  /** Reserve stable space for the scrollbar so content doesn't reflow when it appears. */
  scrollbarGutter?: boolean
  /** className applied to the inner Viewport element (not the Root). */
  viewportClassName?: string
  /** Additional props spread onto the inner Viewport element. */
  viewportProps?: React.HTMLAttributes<HTMLDivElement>
  /** Ref to the inner Viewport element (distinct from the forwarded Root ref). */
  viewportRef?: React.Ref<HTMLDivElement>
}

const ScrollArea = React.forwardRef<
  React.ElementRef<typeof ScrollAreaPrimitive.Root>,
  ScrollAreaProps
>(
  (
    {
      className,
      children,
      orientation = "vertical",
      scrollFade = false,
      scrollbarGutter = false,
      viewportClassName,
      viewportProps,
      viewportRef,
      ...props
    },
    ref
  ) => {
    const innerViewportRef = React.useRef<HTMLDivElement | null>(null)

    const setViewportRef = React.useCallback(
      (node: HTMLDivElement | null) => {
        innerViewportRef.current = node
        if (typeof viewportRef === "function") viewportRef(node)
        else if (viewportRef && "current" in viewportRef) {
          // `RefObject.current` is `readonly` in current @types/react (only
          // React itself is meant to assign it) -- casting to `RefObject`
          // keeps that readonly modifier, so cast to a plain writable shape
          // instead; this is the standard escape hatch for the "forward a
          // second, non-root ref into a child's internal DOM node" pattern.
          ;(viewportRef as unknown as { current: HTMLDivElement | null }).current = node
        }
      },
      [viewportRef]
    )

    React.useEffect(() => {
      const viewport = innerViewportRef.current
      if (!viewport) return

      const updateOverflow = () => {
        viewport.toggleAttribute(
          "data-has-overflow-x",
          viewport.scrollWidth > viewport.clientWidth
        )
      }

      updateOverflow()

      const observer = new ResizeObserver(updateOverflow)
      observer.observe(viewport)
      if (viewport.firstElementChild) observer.observe(viewport.firstElementChild)

      return () => observer.disconnect()
    }, [children])

    return (
    <ScrollAreaPrimitive.Root
      ref={ref}
      className={cn(
        "relative overflow-hidden",
        scrollFade &&
          "[mask-image:linear-gradient(to_bottom,transparent,black_12px,black_calc(100%-12px),transparent)]",
        className
      )}
      {...props}
    >
      <ScrollAreaPrimitive.Viewport
        ref={setViewportRef}
        data-slot="scroll-area-viewport"
        className={cn(
          "h-full w-full rounded-[inherit]",
          scrollbarGutter && "[scrollbar-gutter:stable]",
          viewportClassName
        )}
        {...viewportProps}
      >
        {children}
      </ScrollAreaPrimitive.Viewport>
      {orientation === "both" ? (
        <>
          <ScrollBar orientation="vertical" />
          <ScrollBar orientation="horizontal" />
        </>
      ) : (
        <ScrollBar orientation={orientation} />
      )}
      <ScrollAreaPrimitive.Corner />
    </ScrollAreaPrimitive.Root>
    )
  }
)
ScrollArea.displayName = ScrollAreaPrimitive.Root.displayName

const ScrollBar = React.forwardRef<
  React.ElementRef<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>,
  React.ComponentPropsWithoutRef<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>
>(({ className, orientation = "vertical", ...props }, ref) => (
  <ScrollAreaPrimitive.ScrollAreaScrollbar
    ref={ref}
    orientation={orientation}
    className={cn(
      "flex touch-none select-none transition-colors",
      orientation === "vertical" &&
        "h-full w-2.5 border-l border-l-transparent p-[1px]",
      orientation === "horizontal" &&
        "h-2.5 flex-col border-t border-t-transparent p-[1px]",
      className
    )}
    {...props}
  >
    <ScrollAreaPrimitive.ScrollAreaThumb className="relative flex-1 rounded-full bg-border" />
  </ScrollAreaPrimitive.ScrollAreaScrollbar>
))
ScrollBar.displayName = ScrollAreaPrimitive.ScrollAreaScrollbar.displayName

export { ScrollArea, ScrollBar }
