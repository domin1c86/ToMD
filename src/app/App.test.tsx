import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
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

  it("does not expose raw desktop bridge errors in the project list", async () => {
    mockedDesktop.listProjects.mockRejectedValue(
      new TypeError("Cannot read properties of undefined (reading 'invoke')"),
    );

    render(<App />);

    expect(await screen.findByText("桌面后端暂不可用，请在桌面应用中运行。")).toBeVisible();
    expect(screen.queryByText("Cannot read properties of undefined (reading 'invoke')")).not.toBeInTheDocument();
  });

  it("renders the productized shell with Chinese defaults, workflow, language, and theme controls", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByText("设计规范提取器")).toBeVisible();
    expect(screen.getAllByText("项目").length).toBeGreaterThan(0);
    expect(screen.getAllByText("截图").length).toBeGreaterThan(0);
    expect(screen.getAllByText("模型").length).toBeGreaterThan(0);
    expect(screen.getAllByText("分析").length).toBeGreaterThan(0);
    expect(screen.getAllByText("审核导出").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("button", { name: "Switch to English" }));
    expect((await screen.findAllByText("Design Language Extractor")).length).toBeGreaterThan(0);
    expect(localStorage.getItem("design-md-locale")).toBe("en-US");

    await user.click(screen.getByRole("button", { name: "Switch to dark theme" }));
    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
    expect(localStorage.getItem("design-md-theme")).toBe("dark");
  });
});
