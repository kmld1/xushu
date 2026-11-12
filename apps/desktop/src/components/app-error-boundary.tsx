import { AlertTriangle, RotateCcw } from "lucide-react";
import { Component, type ErrorInfo, type ReactNode } from "react";

import { Button } from "@/components/ui/button";

type AppErrorBoundaryProps = {
  children: ReactNode;
};

type AppErrorBoundaryState = {
  error: Error | null;
};

export class AppErrorBoundary extends Component<AppErrorBoundaryProps, AppErrorBoundaryState> {
  state: AppErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): AppErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[xushu] unhandled UI error", error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      return (
        <main className="grid min-h-screen place-items-center bg-workspace p-6">
          <section
            role="alert"
            className="w-full max-w-lg rounded-lg border border-line bg-card p-6 text-card-foreground"
          >
            <div className="flex items-center gap-3 text-destructive">
              <AlertTriangle className="size-5" aria-hidden="true" />
              <h1 className="text-lg font-semibold">界面发生错误</h1>
            </div>
            <p className="mt-3 text-sm leading-6 text-muted-foreground">
              当前页面无法继续渲染。你的本地数据没有被删除，可以重新加载界面后重试。
            </p>
            <details className="mt-4 rounded-md border bg-muted/40 p-3 text-xs text-muted-foreground">
              <summary className="cursor-pointer font-medium">技术信息</summary>
              <pre className="mt-2 whitespace-pre-wrap break-all font-mono">
                {this.state.error.message}
              </pre>
            </details>
            <Button className="mt-6" onClick={() => window.location.reload()}>
              <RotateCcw aria-hidden="true" />
              重新加载
            </Button>
          </section>
        </main>
      );
    }

    return this.props.children;
  }
}
