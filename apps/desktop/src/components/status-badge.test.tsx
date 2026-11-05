import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { PriorityBadge, StatusBadge, VerificationBadge } from "@/components/status-badge";
import type { TaskPriority, TaskStatus } from "@/lib/domain";
import {
  TASK_PRIORITY_META,
  TASK_STATUS_META,
  VERIFICATION_META,
  type VerificationStatus,
} from "@/lib/task-display";

const STATUSES: TaskStatus[] = [
  "draft",
  "ready",
  "running",
  "awaiting_verification",
  "done",
  "blocked",
  "cancelled",
];

const PRIORITIES: TaskPriority[] = ["low", "medium", "high", "urgent"];

const VERIFICATION_STATUSES: VerificationStatus[] = [
  "pass",
  "fail",
  "not_run",
  "inconclusive",
  "stale",
];

describe("status display", () => {
  it.each(STATUSES)("renders text and icon for %s", (status) => {
    const { container } = render(<StatusBadge status={status} />);
    expect(screen.getByText(TASK_STATUS_META[status].label)).toBeInTheDocument();
    expect(container.querySelector("svg")).toBeInTheDocument();
  });

  it.each(PRIORITIES)("renders text and icon for %s priority", (priority) => {
    const { container } = render(<PriorityBadge priority={priority} />);
    expect(screen.getByText(TASK_PRIORITY_META[priority].label)).toBeInTheDocument();
    expect(container.querySelector("svg")).toBeInTheDocument();
  });

  it.each(VERIFICATION_STATUSES)("renders future verification status %s", (status) => {
    render(<VerificationBadge status={status} />);
    expect(screen.getByText(VERIFICATION_META[status].label)).toBeInTheDocument();
  });
});
