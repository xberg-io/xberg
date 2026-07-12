"use client";
import { createContext, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { cn } from "@/lib/utils.js";

const DialogCtx = createContext<{ open: boolean; setOpen: (v: boolean) => void } | null>(null);

export function Dialog({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  return <DialogCtx.Provider value={{ open, setOpen }}>{children}</DialogCtx.Provider>;
}
export function DialogTrigger({ children }: { children: ReactNode; asChild?: boolean }) {
  const ctx = useContext(DialogCtx)!;
  return <span onClick={() => ctx.setOpen(true)}>{children}</span>;
}
export function DialogContent({ className, children }: { className?: string; children: ReactNode }) {
  const ctx = useContext(DialogCtx)!;
  const contentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!ctx.open) return;

    const prevFocus = document.activeElement as HTMLElement;
    contentRef.current?.focus();

    const getFocusableElements = () => {
      const selector = 'button:not([disabled]), [href], input, select, textarea, [tabindex]:not([tabindex="-1"])';
      return Array.from(contentRef.current?.querySelectorAll(selector) ?? []) as HTMLElement[];
    };

    const handleTab = (e: KeyboardEvent) => {
      if (e.key !== "Tab") return;
      const focusable = getFocusableElements();
      if (focusable.length === 0) {
        e.preventDefault();
        return;
      }
      const currentIndex = focusable.indexOf(document.activeElement as HTMLElement);
      if (e.shiftKey) {
        if (currentIndex <= 0) {
          e.preventDefault();
          focusable[focusable.length - 1]?.focus();
        }
      } else {
        if (currentIndex >= focusable.length - 1) {
          e.preventDefault();
          focusable[0]?.focus();
        }
      }
    };

    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        ctx.setOpen(false);
      }
    };

    document.addEventListener("keydown", handleTab);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("keydown", handleTab);
      document.removeEventListener("keydown", handleEscape);
      prevFocus?.focus();
    };
  }, [ctx.open]);

  if (!ctx.open) return null;

  return (
    <div className={cn("fixed inset-0 z-50 flex items-center justify-center bg-black/40")} onClick={() => ctx.setOpen(false)}>
      <div
        ref={contentRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        className={cn("rounded-lg bg-white p-6 shadow-lg focus:outline-none", className)}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}
export function DialogHeader({ children }: { children: ReactNode }) {
  return <div className="mb-4">{children}</div>;
}
export function DialogTitle({ children }: { children: ReactNode }) {
  return <h2 className="text-lg font-semibold">{children}</h2>;
}
export function DialogFooter({ children }: { children: ReactNode }) {
  return <div className="mt-4 flex justify-end gap-2">{children}</div>;
}
export function DialogClose({ children }: { children: ReactNode; asChild?: boolean }) {
  const ctx = useContext(DialogCtx)!;
  return <span onClick={() => ctx.setOpen(false)}>{children}</span>;
}
