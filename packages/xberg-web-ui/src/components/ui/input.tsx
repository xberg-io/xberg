import * as React from "react"

import { cn } from "@/lib/utils"

export interface InputProps
  extends Omit<React.ComponentProps<"input">, "size"> {
  /** Design-system size variant. Shadows (and replaces) the native numeric HTML `size` attribute. */
  size?: "default" | "sm"
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, size = "default", ...props }, ref) => {
    return (
      <input
        type={type}
        className={cn(
          "flex w-full rounded-md border border-input bg-transparent px-3 py-1 text-base shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
          size === "sm" ? "h-8" : "h-9",
          className
        )}
        ref={ref}
        {...props}
      />
    )
  }
)
Input.displayName = "Input"

export { Input }
