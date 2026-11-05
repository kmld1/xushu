import { AlertCircle, RefreshCw } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";

export function ErrorState({
  title = "加载失败",
  message,
  onRetry,
}: {
  title?: string;
  message: string;
  onRetry?: () => void;
}) {
  return (
    <Alert variant="destructive">
      <AlertCircle className="mt-0.5 size-4" aria-hidden="true" />
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription>
        <span className="block">{message}</span>
        {onRetry ? (
          <Button type="button" variant="outline" size="sm" className="mt-3" onClick={onRetry}>
            <RefreshCw aria-hidden="true" />
            重试
          </Button>
        ) : null}
      </AlertDescription>
    </Alert>
  );
}
