import { Compass } from "lucide-react";
import { Link } from "react-router";

import { Button } from "@/components/ui/button";

export function NotFoundPage() {
  return (
    <div className="grid min-h-[60vh] place-items-center">
      <div className="text-center">
        <Compass className="mx-auto size-10 text-muted-foreground" aria-hidden="true" />
        <h1 className="mt-4 text-xl font-semibold">页面不存在</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          当前地址没有对应的页面，可能已被移动或尚未实现。
        </p>
        <Button asChild className="mt-6" variant="outline">
          <Link to="/">返回概览</Link>
        </Button>
      </div>
    </div>
  );
}
