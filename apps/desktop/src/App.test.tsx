import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import App from "./App";

describe("App", () => {
  it("renders the Phase 0 scaffold notice", () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "序枢 XUSHU" })).toBeInTheDocument();
    expect(screen.getByText(/Phase 0/)).toBeInTheDocument();
    expect(screen.getByText(/不代表任何产品功能已完成/)).toBeInTheDocument();
  });
});
