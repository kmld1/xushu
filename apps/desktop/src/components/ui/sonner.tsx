import { Toaster as SonnerToaster } from "sonner";

import { useThemeStore } from "@/stores/theme-store";

export function Toaster() {
  const mode = useThemeStore((state) => state.mode);

  return (
    <SonnerToaster
      theme={mode}
      position="bottom-right"
      closeButton
      toastOptions={{
        classNames: {
          toast: "border bg-popover text-popover-foreground shadow-overlay",
          description: "text-muted-foreground",
          actionButton: "bg-primary text-primary-foreground",
          cancelButton: "bg-secondary text-secondary-foreground",
        },
      }}
    />
  );
}
