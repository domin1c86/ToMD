import React from "react";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../src/app/App";
import type { DesignSpec, Rule } from "../src/generated/bindings";
import { desktop, type ExportVersion, type Project, type Provider, type Screenshot } from "../src/lib/desktop";

vi.mock("../src/lib/desktop", () => ({
  desktop: {
    listProjects: vi.fn(),
    createProject: vi.fn(),
    listScreenshots: vi.fn(),
    importScreenshots: vi.fn(),
    updateScreenshotMetadata: vi.fn(),
    removeScreenshot: vi.fn(),
    screenshotUrl: vi.fn(),
    listProviders: vi.fn(),
    saveProvider: vi.fn(),
    testProvider: vi.fn(),
    previewAnalysisRequest: vi.fn(),
    analyzeProject: vi.fn(),
    getDesignSpec: vi.fn(),
    updateRule: vi.fn(),
    listExports: vi.fn(),
    exportDesignMarkdown: vi.fn(),
  },
}));

const mockedDesktop = vi.mocked(desktop);

describe("local-first MVP privacy flow", () => {
  let projects: Project[];
  let screenshots: Screenshot[];
  let providers: Provider[];
  let exportsHistory: ExportVersion[];
  let spec: DesignSpec;
  let fetchSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.resetAllMocks();
    window.history.pushState({}, "", "/");
    fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
    });

    projects = [];
    screenshots = [];
    providers = [];
    exportsHistory = [];
    spec = specFixture();

    mockedDesktop.listProjects.mockImplementation(async () => projects);
    mockedDesktop.createProject.mockImplementation(async ({ name, platform }) => {
      const project = projectFixture({ name, platform });
      projects = [project];
      return project;
    });
    mockedDesktop.listScreenshots.mockImplementation(async () => screenshots);
    mockedDesktop.screenshotUrl.mockReturnValue("asset://localhost/shot-1.png");
    mockedDesktop.importScreenshots.mockImplementation(async ({ projectId }) => {
      screenshots = [screenshotFixture({ project_id: projectId })];
      return screenshots;
    });
    mockedDesktop.listProviders.mockImplementation(async () => providers);
    mockedDesktop.saveProvider.mockImplementation(async ({ name, kind, baseUrl, model }) => {
      const provider = providerFixture({ name, kind, base_url: baseUrl, model });
      providers = [provider];
      return provider;
    });
    mockedDesktop.testProvider.mockResolvedValue({ image_input: true, structured_output: true });
    mockedDesktop.previewAnalysisRequest.mockImplementation(async () => ({
      provider_name: providers[0].name,
      model: providers[0].model,
      image_ids: screenshots.map((screenshot) => screenshot.id),
      image_count: screenshots.length,
      estimated_encoded_bytes: 2048,
    }));
    mockedDesktop.analyzeProject.mockResolvedValue({
      version_id: "version-1",
      repair_attempted: false,
      spec,
    });
    mockedDesktop.getDesignSpec.mockImplementation(async () => spec);
    mockedDesktop.updateRule.mockImplementation(async ({ ruleId, statement, status }) => {
      spec = replaceRule(spec, ruleId, (rule) => ({
        ...rule,
        statement: statement ?? rule.statement,
        status: status ?? rule.status,
        source: statement ? "user" : rule.source,
      }));
      return spec;
    });
    mockedDesktop.listExports.mockImplementation(async () => exportsHistory);
    mockedDesktop.exportDesignMarkdown.mockImplementation(async ({ projectId }) => {
      const exported = exportFixture({ project_id: projectId });
      exportsHistory = [exported];
      return exported;
    });
  });

  it("covers create project through export history without direct frontend network calls", async () => {
    const user = userEvent.setup();
    render(React.createElement(App));

    await user.click(await screen.findByRole("button", { name: "New project" }));
    await user.type(screen.getByLabelText("Project name"), "Finance");
    await user.selectOptions(screen.getByLabelText("Target platform"), "mobile");
    await user.click(screen.getByRole("button", { name: "Create project" }));

    await user.type(await screen.findByLabelText("Local screenshot paths"), "C:/safe/dashboard.png");
    await user.click(screen.getByRole("button", { name: "Import screenshots" }));
    await user.click(await screen.findByRole("button", { name: "Configure analysis" }));

    await user.selectOptions(await screen.findByLabelText("Provider type"), "open_ai_compatible");
    await user.type(screen.getByLabelText("Provider name"), "My endpoint");
    await user.type(screen.getByLabelText("Base URL"), "https://ai.example.com/v1");
    await user.type(screen.getByLabelText("Model name"), "vision-model");
    await user.type(screen.getByLabelText("API key"), "sk-secret");
    await user.click(screen.getByRole("button", { name: "Save provider" }));
    await user.click(await screen.findByRole("button", { name: "Test connection for My endpoint" }));
    await user.click(await screen.findByRole("button", { name: "Continue to disclosure" }));

    expect(await screen.findByText("Provider: My endpoint")).toBeVisible();
    expect(screen.getByText("Model: vision-model")).toBeVisible();
    expect(screen.getByText("1 images will be sent")).toBeVisible();
    await user.click(screen.getByRole("button", { name: "Send and analyze" }));

    await user.click(await screen.findByRole("button", { name: "Use 12px card radii" }));
    await user.click(screen.getByRole("button", { name: "Accept rule" }));

    await user.click(screen.getByRole("button", { name: "Use compact navigation" }));
    await user.clear(screen.getByLabelText("Rule statement"));
    await user.type(screen.getByLabelText("Rule statement"), "Use compact bottom navigation");
    await user.click(screen.getByRole("button", { name: "Save edit" }));

    await user.click(screen.getByRole("button", { name: "Avoid ornamental gradients" }));
    await user.click(screen.getByRole("button", { name: "Reject rule" }));
    expect(screen.getByTestId("markdown-preview")).not.toHaveTextContent("Avoid ornamental gradients");

    window.history.pushState({}, "", "/projects/project-1/exports");
    window.dispatchEvent(new Event("popstate"));

    await user.click(await screen.findByRole("button", { name: "Export current version" }));
    const history = await screen.findByRole("list", { name: "Export history" });
    expect(within(history).getByText("Source spec version: version-1")).toBeVisible();
    expect(within(history).getByText("exports/20260702T120000.000000000Z-DESIGN.md")).toBeVisible();
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});

function projectFixture(overrides: Partial<Project> = {}): Project {
  return {
    id: "project-1",
    name: "Finance",
    platform: "mobile",
    archived_at: null,
    created_at: "2026-07-02T12:00:00Z",
    updated_at: "2026-07-02T12:00:00Z",
    ...overrides,
  };
}

function screenshotFixture(overrides: Partial<Screenshot> = {}): Screenshot {
  return {
    id: "shot-1",
    project_id: "project-1",
    relative_path: "screenshots/shot-1.png",
    absolute_path: "C:/app-data/projects/project-1/screenshots/shot-1.png",
    sha256: "hash",
    media_type: "image/png",
    width: 1200,
    height: 800,
    page_name: "Dashboard",
    scene: "Logged in",
    sort_order: 1,
    created_at: "2026-07-02T12:00:00Z",
    ...overrides,
  };
}

function providerFixture(overrides: Partial<Provider> = {}): Provider {
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

function exportFixture(overrides: Partial<ExportVersion> = {}): ExportVersion {
  return {
    id: "export-1",
    project_id: "project-1",
    spec_version_id: "version-1",
    relative_path: "exports/20260702T120000.000000000Z-DESIGN.md",
    created_at: "2026-07-02T12:00:00Z",
    ...overrides,
  };
}

function specFixture(): DesignSpec {
  return {
    metadata: {
      schema_version: "1.0.0",
      project_id: "project-1",
      platform: "mobile",
      provider_id: "provider-1",
      model: "vision-model",
      source_screenshot_ids: ["shot-1"],
      excluded_terms: [],
      created_at: "2026-07-02T12:00:00Z",
    },
    intent: [],
    tokens: [
      rule({ id: "rule-radius", category: "tokens", statement: "Use 12px card radii" }),
      rule({ id: "rule-gradient", category: "tokens", statement: "Avoid ornamental gradients" }),
    ],
    layout: [rule({ id: "rule-nav", category: "layout", statement: "Use compact navigation" })],
    components: [],
    assets: [],
    motion: [],
    constraints: [],
    evidence: [
      {
        id: "ev-1",
        screenshot_id: "shot-1",
        region: null,
        description: "Visible dashboard evidence",
      },
    ],
    uncertainties: [],
  };
}

function rule(overrides: Partial<Rule>): Rule {
  return {
    id: "rule",
    category: "tokens",
    statement: "Rule statement",
    kind: "recommendation",
    scope: "global",
    value: null,
    evidence_ids: ["ev-1"],
    confidence: 0.9,
    status: "pending",
    source: "model",
    ...overrides,
  };
}

function replaceRule(
  currentSpec: DesignSpec,
  ruleId: string,
  update: (rule: Rule) => Rule,
): DesignSpec {
  return {
    ...currentSpec,
    intent: currentSpec.intent.map((rule) => (rule.id === ruleId ? update(rule) : rule)),
    tokens: currentSpec.tokens.map((rule) => (rule.id === ruleId ? update(rule) : rule)),
    layout: currentSpec.layout.map((rule) => (rule.id === ruleId ? update(rule) : rule)),
    components: currentSpec.components.map((rule) => (rule.id === ruleId ? update(rule) : rule)),
    assets: currentSpec.assets.map((rule) => (rule.id === ruleId ? update(rule) : rule)),
    motion: currentSpec.motion.map((rule) => (rule.id === ruleId ? update(rule) : rule)),
    constraints: currentSpec.constraints.map((rule) => (rule.id === ruleId ? update(rule) : rule)),
  };
}
