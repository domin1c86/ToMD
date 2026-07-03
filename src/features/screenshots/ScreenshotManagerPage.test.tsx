import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../../app/App";
import { desktop, type Screenshot } from "../../lib/desktop";

vi.mock("../../lib/desktop", () => ({
  desktop: {
    listProjects: vi.fn(),
    listScreenshots: vi.fn(),
    importScreenshots: vi.fn(),
    updateScreenshotMetadata: vi.fn(),
    removeScreenshot: vi.fn(),
    listProviders: vi.fn(),
  },
}));

const mockedDesktop = vi.mocked(desktop);

describe("ScreenshotManagerPage", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    window.history.pushState({}, "", "/projects/project-1");
    mockedDesktop.listProviders.mockRejectedValue(new Error("providers unavailable"));
    mockedDesktop.listProjects.mockResolvedValue([
      {
        id: "project-1",
        name: "Finance app",
        platform: "mobile",
        archived_at: null,
        created_at: "2026-07-01T00:00:00Z",
        updated_at: "2026-07-01T00:00:00Z",
      },
    ]);
    mockedDesktop.listScreenshots.mockResolvedValue([]);
  });

  it("shows screenshot empty state and disables analysis until at least one screenshot exists", async () => {
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Reference screenshots" })).toBeVisible();
    expect(screen.getByText("导入参考截图")).toBeVisible();
    expect(screen.getByText("分析前不会发送任何图片。")).toBeVisible();
    expect(screen.getByText("No screenshots imported yet.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Configure analysis" })).toBeDisabled();
  });

  it("displays screenshot metadata and recommends three or more screenshots", async () => {
    const user = userEvent.setup();
    mockedDesktop.listScreenshots.mockResolvedValue([
      screenshot({ id: "shot-2", sort_order: 2, page_name: "Settings" }),
      screenshot({ id: "shot-1", sort_order: 1, page_name: "Home" }),
    ]);

    render(<App />);

    const rows = await screen.findAllByRole("row");
    expect(within(rows[1]).getByDisplayValue("Home")).toBeVisible();
    expect(within(rows[2]).getByDisplayValue("Settings")).toBeVisible();
    expect(screen.getByText("Recommendation: import at least 3 screenshots for stronger patterns.")).toBeVisible();
    const configureButton = screen.getByRole("button", { name: "Configure analysis" });
    expect(configureButton).toBeEnabled();

    await user.click(configureButton);
    expect(await screen.findByText("Provider setup")).toBeVisible();
  });

  it("keeps local screenshot management available when provider commands are unavailable", async () => {
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Reference screenshots" })).toBeVisible();
    expect(screen.getByLabelText("Local screenshot paths")).toBeVisible();
    expect(mockedDesktop.listProviders).not.toHaveBeenCalled();
  });

  it("imports screenshots and shows import errors", async () => {
    const user = userEvent.setup();
    mockedDesktop.importScreenshots.mockRejectedValueOnce(new Error("duplicate screenshot"));

    render(<App />);

    await user.type(await screen.findByLabelText("Local screenshot paths"), "C:/shots/home.png");
    await user.click(screen.getByRole("button", { name: "Import screenshots" }));

    expect(mockedDesktop.importScreenshots).toHaveBeenCalledWith({
      projectId: "project-1",
      paths: ["C:/shots/home.png"],
    });
    expect(await screen.findByText("duplicate screenshot")).toBeVisible();
  });

  it("updates screenshot metadata and removes screenshots", async () => {
    const user = userEvent.setup();
    mockedDesktop.listScreenshots.mockResolvedValue([screenshot({ id: "shot-1" })]);
    mockedDesktop.updateScreenshotMetadata.mockResolvedValue(
      screenshot({ id: "shot-1", page_name: "Dashboard", scene: "Logged in" }),
    );
    mockedDesktop.removeScreenshot.mockResolvedValue(undefined);

    render(<App />);

    await user.clear(await screen.findByLabelText("Page name for Home"));
    await user.type(screen.getByLabelText("Page name for Home"), "Dashboard");
    await user.clear(screen.getByLabelText("Scene for Home"));
    await user.type(screen.getByLabelText("Scene for Home"), "Logged in");
    await user.click(screen.getByRole("button", { name: "Save metadata for Home" }));

    expect(mockedDesktop.updateScreenshotMetadata).toHaveBeenCalledWith({
      projectId: "project-1",
      screenshotId: "shot-1",
      pageName: "Dashboard",
      scene: "Logged in",
      sortOrder: 1,
    });

    await user.click(screen.getByRole("button", { name: "Remove Home" }));
    expect(mockedDesktop.removeScreenshot).toHaveBeenCalledWith({
      projectId: "project-1",
      screenshotId: "shot-1",
    });
  });
});

function screenshot(overrides: Partial<Screenshot> = {}): Screenshot {
  return {
    id: "shot-1",
    project_id: "project-1",
    relative_path: "screenshots/shot-1.png",
    sha256: "hash",
    media_type: "image/png",
    width: 1200,
    height: 800,
    page_name: "Home",
    scene: "Logged out",
    sort_order: 1,
    created_at: "2026-07-01T00:00:00Z",
    ...overrides,
  };
}
