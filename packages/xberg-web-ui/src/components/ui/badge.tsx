import { cn } from "@/lib/utils.js";
import type { HTMLAttributes } from "react";

export function Badge({ className, ...props }: HTMLAttributes<HTMLSpanElement>) {
  return <span className={cn("inline-flex items-center rounded-full bg-slate-100 px-2 py-0.5 text-xs font-medium", className)} {...props} />;
}
