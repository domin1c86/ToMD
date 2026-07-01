import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "./App";
import { desktop } from "../lib/desktop";

vi.mock("../lib/desktop", () => ({
  desktop: {
    listProjects: vi.fn(),
  },
}));

const mockedDesktop = vi.mocked(desktop);

describe("App", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    window.history.pushState({}, "", "/");
    mockedDesktop.listProjects.mockResolvedValue([]);
  });

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

  it("renders the empty projects state at the root route", async () => {
    render(<App />);

    expect(await screen.findByText("No projects yet.")).toBeInTheDocument();
  });
});
