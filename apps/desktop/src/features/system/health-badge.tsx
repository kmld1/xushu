import { AlertTriangle, CheckCircle2 } from "lucide-react";

import { Badge } from "@/components/ui/badge";

export function HealthBadge({ ok }: { ok: boolean }) {
  if (ok) {
    return (
      <Badge variant="success">
        <CheckCircle2 className="size-3" aria-hidden="true" />
        正常
      </Badge>
    );
  }

  return (
    <Badge variant="warning">
      <AlertTriangle className="size-3" aria-hidden="true" />
      需关注
    </Badge>
  );
}
