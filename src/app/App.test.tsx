import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { App } from "./App";

describe("App", () => {
  it("renders the application heading and Projects navigation link", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", { name: "Design Language Extractor" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Projects" })).toHaveAttribute(
      "href",
      "/",
    );
  });

  it("renders the empty projects state at the root route", () => {
    render(<App />);

    expect(screen.getByText("No projects yet.")).toBeInTheDocument();
  });
});
