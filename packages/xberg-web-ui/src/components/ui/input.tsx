import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "@/lib/utils.js";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(({ className, ...props }, ref) => (
  <input
    ref={ref}
    className={cn("h-9 w-full rounded-md border border-slate-300 px-3 text-sm outline-none focus:ring-2 focus:ring-slate-400", className)}
    {...props}
  />
));
Input.displayName = "Input";
