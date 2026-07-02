import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../../app/App";
import type { DesignSpec } from "../../generated/bindings";
import { desktop, type ExportVersion } from "../../lib/desktop";

vi.mock("../../lib/desktop", () => ({
  desktop: {
    getDesignSpec: vi.fn(),
    listExports: vi.fn(),
    exportDesignMarkdown: vi.fn(),
  },
}));

const mockedDesktop = vi.mocked(desktop);
let writeClipboardText: ReturnType<typeof vi.fn>;

describe("ExportHistoryPage", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    window.history.pushState({}, "", "/projects/project-1/exports");
    writeClipboardText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: writeClipboardText,
      },
    });
    mockedDesktop.getDesignSpec.mockResolvedValue(specFixture());
    mockedDesktop.listExports.mockResolvedValue([exportVersion()]);
    mockedDesktop.exportDesignMarkdown.mockResolvedValue(
      exportVersion({
        id: "export-2",
        spec_version_id: "version-2",
        relative_path: "exports/20260701T101600.000000000Z-DESIGN.md",
        created_at: "2026-07-01T10:16:00Z",
      }),
    );
  });

  it("previews exportable markdown without showing rejected rules", async () => {
    render(<App />);

    expect(await screen.findByTestId("export-preview")).toHaveTextContent("Use 12px card radii");
    expect(screen.getByTestId("export-preview")).toHaveTextContent("Use compact navigation");
    expect(screen.getByTestId("export-preview")).not.toHaveTextContent("Do not export this");
  });

  it("lists export timestamp and source spec version", async () => {
    render(<App />);

    expect(await screen.findByText("2026-07-01T10:15:00Z")).toBeVisible();
    expect(screen.getByText("Source spec version: version-1")).toBeVisible();
    expect(screen.getByText("exports/20260701T101500.000000000Z-DESIGN.md")).toBeVisible();
  });

  it("exports the current version and refreshes history", async () => {
    const user = userEvent.setup();
    mockedDesktop.listExports
      .mockResolvedValueOnce([exportVersion()])
      .mockResolvedValueOnce([
        exportVersion({
          id: "export-2",
          spec_version_id: "version-2",
          relative_path: "exports/20260701T101600.000000000Z-DESIGN.md",
          created_at: "2026-07-01T10:16:00Z",
        }),
        exportVersion(),
      ]);

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Export current version" }));

    expect(mockedDesktop.exportDesignMarkdown).toHaveBeenCalledWith({ projectId: "project-1" });
    expect(await screen.findByText("Source spec version: version-2")).toBeVisible();
  });

  it("copies and reveals an exported file path", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Copy path for export-1" }));
    expect(await screen.findByText("Copied path: exports/20260701T101500.000000000Z-DESIGN.md")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "Reveal in folder for export-1" }));
    expect(screen.getByText("Reveal in folder: exports/20260701T101500.000000000Z-DESIGN.md")).toBeVisible();
  });
});

function exportVersion(overrides: Partial<ExportVersion> = {}): ExportVersion {
  return {
    id: "export-1",
    project_id: "project-1",
    spec_version_id: "version-1",
    relative_path: "exports/20260701T101500.000000000Z-DESIGN.md",
    created_at: "2026-07-01T10:15:00Z",
    ...overrides,
  };
}

function specFixture(): DesignSpec {
  return {
    metadata: {
      schema_version: "1.0.0",
      project_id: "project-1",
      platform: "web",
      provider_id: "provider-1",
      model: "vision-model",
      source_screenshot_ids: ["shot-1"],
      excluded_terms: [],
      created_at: "2026-07-01T10:00:00Z",
    },
    intent: [],
    tokens: [
      rule({ id: "rule-radius", statement: "Use 12px card radii", status: "accepted" }),
      rule({ id: "rule-rejected", statement: "Do not export this", status: "rejected" }),
    ],
    layout: [rule({ id: "rule-nav", statement: "Use compact navigation", status: "edited" })],
    components: [],
    assets: [],
    motion: [],
    constraints: [],
    evidence: [],
    uncertainties: [],
  };
}

function rule(overrides: Partial<DesignSpec["tokens"][number]>): DesignSpec["tokens"][number] {
  return {
    id: "rule",
    category: "tokens",
    statement: "Rule statement",
    kind: "recommendation",
    scope: "global",
    value: null,
    evidence_ids: [],
    confidence: 0.9,
    status: "pending",
    source: "model",
    ...overrides,
  };
}
