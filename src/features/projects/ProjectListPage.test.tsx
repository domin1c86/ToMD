import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../../app/App";
import { desktop } from "../../lib/desktop";

vi.mock("../../lib/desktop", () => ({
  desktop: {
    listProjects: vi.fn(),
    createProject: vi.fn(),
    archiveProject: vi.fn(),
    deleteProject: vi.fn(),
    listScreenshots: vi.fn(),
    listProviders: vi.fn(),
  },
}));

const mockedDesktop = vi.mocked(desktop);

describe("ProjectListPage", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    window.history.pushState({}, "", "/");
    mockedDesktop.listProjects.mockResolvedValue([]);
    mockedDesktop.listScreenshots.mockResolvedValue([]);
    mockedDesktop.listProviders.mockRejectedValue(new Error("providers unavailable"));
  });

  it("creates a project and opens screenshot management", async () => {
    const user = userEvent.setup();
    mockedDesktop.createProject.mockResolvedValue({
      id: "project-1",
      name: "Finance app",
      platform: "mobile",
      archived_at: null,
      created_at: "2026-07-01T00:00:00Z",
      updated_at: "2026-07-01T00:00:00Z",
    });

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "New project" }));
    await user.type(screen.getByLabelText("Project name"), "Finance app");
    await user.selectOptions(screen.getByLabelText("Target platform"), "mobile");
    await user.click(screen.getByRole("button", { name: "Create project" }));

    expect(await screen.findByRole("heading", { name: "Reference screenshots" })).toBeVisible();
    expect(mockedDesktop.createProject).toHaveBeenCalledWith({
      name: "Finance app",
      platform: "mobile",
    });
  });

  it("shows empty state and validates project name", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByText("No projects yet.")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "New project" }));
    await user.click(screen.getByRole("button", { name: "Create project" }));

    expect(screen.getByText("Project name is required.")).toBeVisible();
    expect(mockedDesktop.createProject).not.toHaveBeenCalled();
  });

  it("archives and deletes projects with confirmation", async () => {
    const user = userEvent.setup();
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

    render(<App />);

    expect(await screen.findByRole("link", { name: "Finance app" })).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Archive Finance app" }));
    expect(mockedDesktop.archiveProject).toHaveBeenCalledWith({ projectId: "project-1" });

    await user.click(screen.getByRole("button", { name: "Delete Finance app" }));
    expect(screen.getByText("Delete Finance app?")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Confirm delete" }));

    expect(mockedDesktop.deleteProject).toHaveBeenCalledWith({ projectId: "project-1" });
    await waitFor(() => expect(mockedDesktop.listProjects).toHaveBeenCalledTimes(3));
  });

  it("keeps local project management available when provider commands are unavailable", async () => {
    render(<App />);

    expect(await screen.findByRole("button", { name: "New project" })).toBeVisible();
    expect(mockedDesktop.listProviders).not.toHaveBeenCalled();
  });
});
