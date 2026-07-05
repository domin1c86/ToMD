import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../../app/App";
import { desktop, type Provider, type Screenshot } from "../../lib/desktop";
import { markProviderVerified } from "../../lib/providerVerification";

vi.mock("../../lib/desktop", () => ({
  desktop: {
    listProviders: vi.fn(),
    listScreenshots: vi.fn(),
    previewAnalysisRequest: vi.fn(),
    analyzeProject: vi.fn(),
  },
}));

const mockedDesktop = vi.mocked(desktop);

describe("AnalysisStartPage", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    localStorage.clear();
    markProviderVerified("provider-1");
    window.history.pushState({}, "", "/projects/project-1/analyze");
    mockedDesktop.listProviders.mockResolvedValue([provider()]);
    mockedDesktop.listScreenshots.mockResolvedValue([
      screenshot({ id: "shot-1", page_name: "Home" }),
      screenshot({ id: "shot-2", page_name: "Settings" }),
      screenshot({ id: "shot-3", page_name: "Checkout" }),
    ]);
    mockedDesktop.previewAnalysisRequest.mockResolvedValue({
      provider_name: "My endpoint",
      model: "vision-model",
      image_ids: ["shot-1", "shot-2", "shot-3"],
      image_count: 3,
      estimated_encoded_bytes: 123456,
    });
    mockedDesktop.analyzeProject.mockResolvedValue({
      version_id: "version-1",
      repair_attempted: false,
      spec: {
        metadata: {
          schema_version: "1.0.0",
          project_id: "project-1",
          platform: "web",
          provider_id: "provider-1",
          model: "vision-model",
          source_screenshot_ids: ["shot-1", "shot-2", "shot-3"],
          excluded_terms: [],
          created_at: "2026-07-01T00:00:00Z",
        },
        intent: [],
        tokens: [],
        layout: [],
        components: [],
        assets: [],
        motion: [],
        constraints: [],
        evidence: [],
        uncertainties: [],
      },
    });
  });

  it("shows exactly what leaves the device before analysis", async () => {
    render(<App />);

    expect(await screen.findByText("Provider: My endpoint")).toBeVisible();
    expect(screen.getByText("Model: vision-model")).toBeVisible();
    expect(screen.getByText("3 images will be sent")).toBeVisible();
    expect(screen.getByText("Selected image IDs: shot-1, shot-2, shot-3")).toBeVisible();
    expect(screen.getByText("Estimated encoded payload: 123456 bytes")).toBeVisible();
    expect(screen.getByRole("button", { name: "Send and analyze" })).toBeEnabled();
    expect(mockedDesktop.previewAnalysisRequest).toHaveBeenCalledWith({
      projectId: "project-1",
      providerId: "provider-1",
      screenshotIds: ["shot-1", "shot-2", "shot-3"],
    });
  });

  it("sends the disclosed request and opens the workbench", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Send and analyze" }));

    expect(mockedDesktop.analyzeProject).toHaveBeenCalledWith({
      projectId: "project-1",
      providerId: "provider-1",
      screenshotIds: ["shot-1", "shot-2", "shot-3"],
    });
    expect(await screen.findByText("Rule workbench")).toBeVisible();
  });

  it("blocks sending until the selected provider passed a connection test", async () => {
    localStorage.clear();
    render(<App />);

    expect(await screen.findByText("Provider: My endpoint")).toBeVisible();
    expect(screen.getByText("该 Provider 尚未通过连接测试。请先在模型配置页测试连接。")).toBeVisible();
    expect(screen.getByRole("button", { name: "Send and analyze" })).toBeDisabled();
    expect(mockedDesktop.analyzeProject).not.toHaveBeenCalled();
  });

  it("uses the verified provider instead of the first saved provider", async () => {
    const user = userEvent.setup();
    localStorage.clear();
    markProviderVerified("provider-2");
    mockedDesktop.listProviders.mockResolvedValue([
      provider(),
      provider({ id: "provider-2", name: "Second endpoint", model: "other-model" }),
    ]);
    mockedDesktop.previewAnalysisRequest.mockResolvedValue({
      provider_name: "Second endpoint",
      model: "other-model",
      image_ids: ["shot-1", "shot-2", "shot-3"],
      image_count: 3,
      estimated_encoded_bytes: 123456,
    });

    render(<App />);

    expect(await screen.findByText("Provider: Second endpoint")).toBeVisible();
    expect(screen.getByLabelText("Analysis provider")).toHaveValue("provider-2");
    expect(mockedDesktop.previewAnalysisRequest).toHaveBeenCalledWith({
      projectId: "project-1",
      providerId: "provider-2",
      screenshotIds: ["shot-1", "shot-2", "shot-3"],
    });

    await user.click(screen.getByRole("button", { name: "Send and analyze" }));

    expect(mockedDesktop.analyzeProject).toHaveBeenCalledWith({
      projectId: "project-1",
      providerId: "provider-2",
      screenshotIds: ["shot-1", "shot-2", "shot-3"],
    });
  });
});

function provider(overrides: Partial<Provider> = {}): Provider {
  return {
    id: "provider-1",
    name: "My endpoint",
    kind: "open_ai_compatible",
    base_url: "https://ai.example.com/v1",
    model: "vision-model",
    has_credential: true,
    ...overrides,
  };
}

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
