import { cva, type VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

const alertVariants = cva(
  "relative grid w-full grid-cols-[auto_1fr] items-start gap-x-3 gap-y-1 rounded-lg border px-4 py-3 text-sm",
  {
    variants: {
      variant: {
        default: "bg-card text-card-foreground",
        destructive:
          "border-destructive/40 bg-destructive/5 text-destructive [&>svg]:text-destructive",
        warning: "border-warning/40 bg-warning/5 text-warning [&>svg]:text-warning",
        success: "border-success/40 bg-success/5 text-success [&>svg]:text-success",
        info: "border-info/40 bg-info/5 text-info [&>svg]:text-info",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
);

function Alert({
  className,
  variant,
  ...props
}: ComponentProps<"div"> & VariantProps<typeof alertVariants>) {
  return <div role="alert" className={cn(alertVariants({ variant }), className)} {...props} />;
}

function AlertTitle({ className, ...props }: ComponentProps<"div">) {
  return <div className={cn("col-start-2 font-medium", className)} {...props} />;
}

function AlertDescription({ className, ...props }: ComponentProps<"div">) {
  return <div className={cn("col-start-2 text-sm leading-6 opacity-90", className)} {...props} />;
}

export { Alert, AlertDescription, AlertTitle };
