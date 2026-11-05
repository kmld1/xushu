import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import type { ComponentProps, ReactNode } from "react";

import { cn } from "@/lib/utils";

const TooltipProvider = TooltipPrimitive.Provider;
const TooltipRoot = TooltipPrimitive.Root;
const TooltipTrigger = TooltipPrimitive.Trigger;

function TooltipContent({
  className,
  sideOffset = 6,
  ...props
}: ComponentProps<typeof TooltipPrimitive.Content>) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        sideOffset={sideOffset}
        className={cn(
          "z-50 rounded-md border bg-popover px-2.5 py-1.5 text-xs text-popover-foreground shadow-overlay",
          className,
        )}
        {...props}
      />
    </TooltipPrimitive.Portal>
  );
}

export function Tooltip({
  children,
  content,
  side = "bottom",
}: {
  children: ReactNode;
  content: string;
  side?: "top" | "right" | "bottom" | "left";
}) {
  return (
    <TooltipRoot>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side={side}>{content}</TooltipContent>
    </TooltipRoot>
  );
}

export { TooltipContent, TooltipProvider, TooltipRoot, TooltipTrigger };
