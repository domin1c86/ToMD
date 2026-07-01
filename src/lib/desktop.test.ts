import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { desktop } from "./desktop";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("desktop command wrappers", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedInvoke.mockResolvedValue(undefined);
  });

  it("wraps project commands with typed input payloads", async () => {
    await desktop.createProject({ name: "Finance", platform: "mobile" });
    expect(mockedInvoke).toHaveBeenLastCalledWith("create_project", {
      input: { name: "Finance", platform: "mobile" },
    });

    await desktop.renameProject({ projectId: "project-1", name: "New name" });
    expect(mockedInvoke).toHaveBeenLastCalledWith("rename_project", {
      input: { projectId: "project-1", name: "New name" },
    });

    await desktop.archiveProject({ projectId: "project-1" });
    expect(mockedInvoke).toHaveBeenLastCalledWith("archive_project", {
      input: { projectId: "project-1" },
    });

    await desktop.deleteProject({ projectId: "project-1" });
    expect(mockedInvoke).toHaveBeenLastCalledWith("delete_project", {
      input: { projectId: "project-1" },
    });
  });

  it("wraps all command names without leaking invoke to callers", async () => {
    await desktop.listProjects({ includeArchived: false });
    await desktop.listScreenshots({ projectId: "project-1" });
    await desktop.importScreenshots({
      projectId: "project-1",
      paths: ["C:/shots/home.png"],
    });
    await desktop.updateScreenshotMetadata({
      projectId: "project-1",
      screenshotId: "shot-1",
      pageName: "Home",
      scene: "Logged in",
      sortOrder: 1,
    });
    await desktop.removeScreenshot({
      projectId: "project-1",
      screenshotId: "shot-1",
    });
    await desktop.listProviders();
    await desktop.saveProvider({
      name: "Local",
      kind: "open_ai_compatible",
      baseUrl: "http://localhost:11434/v1",
      model: "vision",
      apiKey: "secret",
    });
    await desktop.deleteProvider({ providerId: "provider-1" });
    await desktop.testProvider({ providerId: "provider-1" });
    await desktop.previewAnalysisRequest({
      projectId: "project-1",
      providerId: "provider-1",
      screenshotIds: ["shot-1"],
    });
    await desktop.analyzeProject({
      projectId: "project-1",
      providerId: "provider-1",
      screenshotIds: ["shot-1"],
    });
    await desktop.getDesignSpec({ projectId: "project-1" });
    await desktop.updateRule({
      projectId: "project-1",
      ruleId: "rule-1",
      status: "accepted",
      statement: "Use compact cards.",
    });
    await desktop.listExports({ projectId: "project-1" });
    await desktop.exportDesignMarkdown({ projectId: "project-1" });

    expect(mockedInvoke.mock.calls.map(([command]) => command)).toEqual([
      "list_projects",
      "list_screenshots",
      "import_screenshots",
      "update_screenshot_metadata",
      "remove_screenshot",
      "list_providers",
      "save_provider",
      "delete_provider",
      "test_provider",
      "preview_analysis_request",
      "analyze_project",
      "get_design_spec",
      "update_rule",
      "list_exports",
      "export_design_markdown",
    ]);
  });
});
