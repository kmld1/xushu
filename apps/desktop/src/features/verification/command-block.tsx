import { TerminalSquare } from "lucide-react";

import { formatCommand } from "@/lib/verification-display";
import { cn } from "@/lib/utils";

export function CommandBlock({
  executable,
  args,
  cwd,
  label = "命令",
  className,
}: {
  executable: string;
  args: string[];
  cwd?: string;
  label?: string;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "overflow-hidden rounded-md border border-code-border bg-code text-code-foreground",
        className,
      )}
    >
      <div className="flex items-center justify-between gap-3 border-b border-code-border px-3 py-1.5">
        <span className="inline-flex items-center gap-1.5 text-[10px] font-medium text-code-muted">
          <TerminalSquare className="size-3" aria-hidden="true" />
          {label}
        </span>
        {cwd ? (
          <span className="truncate font-mono text-[10px] text-code-muted" title={cwd}>
            cwd {cwd}
          </span>
        ) : null}
      </div>
      <pre className="overflow-x-auto whitespace-pre-wrap break-all px-3 py-2.5 font-mono text-xs leading-5">
        {formatCommand(executable, args)}
      </pre>
    </div>
  );
}
