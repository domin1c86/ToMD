import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../../app/App";
import { desktop } from "../../lib/desktop";
import type { DesignSpec, Rule } from "../../generated/bindings";

vi.mock("../../lib/desktop", () => ({
  desktop: {
    getDesignSpec: vi.fn(),
    updateRule: vi.fn(),
    listScreenshots: vi.fn(),
    screenshotUrl: vi.fn(),
  },
}));

const mockedDesktop = vi.mocked(desktop);

describe("WorkbenchPage", () => {
  let currentSpec: DesignSpec;

  beforeEach(() => {
    vi.resetAllMocks();
    window.history.pushState({}, "", "/projects/project-1/workbench");
    currentSpec = specFixture();
    mockedDesktop.getDesignSpec.mockImplementation(async () => currentSpec);
    mockedDesktop.listScreenshots.mockResolvedValue([]);
    mockedDesktop.screenshotUrl.mockReturnValue("asset://localhost/shot-1.png");
    mockedDesktop.updateRule.mockImplementation(async ({ ruleId, statement, status }) => {
      currentSpec = replaceRule(currentSpec, ruleId, (rule) => ({
        ...rule,
        statement: statement ?? rule.statement,
        status: status ?? rule.status,
        source: statement ? "user" : rule.source,
      }));
      return currentSpec;
    });
  });

  it("selects rules and highlights matching evidence screenshots", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Use 12px card radii" }));

    expect(screen.getByText("Highlighted screenshot: shot-1")).toBeVisible();
    expect(screen.getByText("Card corner evidence")).toBeVisible();
    expect(screen.getByText("Low confidence")).toBeVisible();
  });

  it("accepts, edits, and rejects rules while updating the markdown preview", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Use 12px card radii" }));
    expect(screen.getByTestId("markdown-preview")).not.toHaveTextContent("Use 12px card radii");
    await user.click(screen.getByRole("button", { name: "Accept rule" }));

    expect(mockedDesktop.updateRule).toHaveBeenCalledWith({
      projectId: "project-1",
      ruleId: "rule-radius",
      status: "accepted",
    });
    expect(await screen.findByText("Status: accepted")).toBeVisible();

    await user.clear(screen.getByLabelText("Rule statement"));
    await user.type(screen.getByLabelText("Rule statement"), "Use 16px card radii");
    await user.click(screen.getByRole("button", { name: "Save edit" }));

    expect(mockedDesktop.updateRule).toHaveBeenLastCalledWith({
      projectId: "project-1",
      ruleId: "rule-radius",
      statement: "Use 16px card radii",
      status: "edited",
    });
    expect(await screen.findByText("Source: user")).toBeVisible();
    expect(screen.getByTestId("markdown-preview")).toHaveTextContent("Use 16px card radii");

    await user.click(screen.getByRole("button", { name: "Reject rule" }));

    expect(mockedDesktop.updateRule).toHaveBeenLastCalledWith({
      projectId: "project-1",
      ruleId: "rule-radius",
      status: "rejected",
    });
    expect(await screen.findByText("Status: rejected")).toBeVisible();
    expect(screen.getByTestId("markdown-preview")).not.toHaveTextContent("Use 16px card radii");
    expect(screen.getByRole("button", { name: "Use 16px card radii" })).toBeVisible();
  });

  it("marks missing evidence and rolls back optimistic edits when persistence fails", async () => {
    const user = userEvent.setup();
    mockedDesktop.updateRule.mockRejectedValueOnce(new Error("database unavailable"));

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Use compact top navigation" }));
    expect(screen.getByText("Missing evidence")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "Accept rule" }));

    expect(await screen.findByText("database unavailable")).toBeVisible();
    expect(screen.getByText("Status: pending")).toBeVisible();
  });

  it("supports keyboard navigation through rule groups and actions", async () => {
    const user = userEvent.setup();
    render(<App />);

    const ruleButton = await screen.findByRole("button", { name: "Use 12px card radii" });
    ruleButton.focus();
    expect(ruleButton).toHaveFocus();
    await user.keyboard("{Enter}");

    const editor = screen.getByLabelText("Selected rule editor");
    expect(within(editor).getByText("rule-radius")).toBeVisible();

    const acceptButton = screen.getByRole("button", { name: "Accept rule" });
    acceptButton.focus();
    expect(acceptButton).toHaveFocus();
    await user.keyboard("{Enter}");

    expect(await screen.findByText("Status: accepted")).toBeVisible();
  });
});

function specFixture(): DesignSpec {
  return {
    metadata: {
      schema_version: "1.0.0",
      project_id: "project-1",
      platform: "web",
      provider_id: "provider-1",
      model: "vision-model",
      source_screenshot_ids: ["shot-1", "shot-2"],
      excluded_terms: [],
      created_at: "2026-07-01T00:00:00Z",
    },
    intent: [],
    tokens: [
      rule({
        id: "rule-radius",
        category: "tokens",
        statement: "Use 12px card radii",
        evidence_ids: ["ev-radius"],
        confidence: 0.42,
      }),
    ],
    layout: [
      rule({
        id: "rule-nav",
        category: "layout",
        statement: "Use compact top navigation",
        evidence_ids: ["missing-evidence"],
      }),
    ],
    components: [],
    assets: [],
    motion: [],
    constraints: [],
    evidence: [
      {
        id: "ev-radius",
        screenshot_id: "shot-1",
        region: { x: 10, y: 20, width: 120, height: 80 },
        description: "Card corner evidence",
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
    evidence_ids: [],
    confidence: 0.9,
    status: "pending",
    source: "model",
    ...overrides,
  };
}

function replaceRule(
  spec: DesignSpec,
  ruleId: string,
  update: (rule: Rule) => Rule,
): DesignSpec {
  return {
    ...spec,
    intent: spec.intent.map((ruleItem) => (ruleItem.id === ruleId ? update(ruleItem) : ruleItem)),
    tokens: spec.tokens.map((ruleItem) => (ruleItem.id === ruleId ? update(ruleItem) : ruleItem)),
    layout: spec.layout.map((ruleItem) => (ruleItem.id === ruleId ? update(ruleItem) : ruleItem)),
    components: spec.components.map((ruleItem) => (ruleItem.id === ruleId ? update(ruleItem) : ruleItem)),
    assets: spec.assets.map((ruleItem) => (ruleItem.id === ruleId ? update(ruleItem) : ruleItem)),
    motion: spec.motion.map((ruleItem) => (ruleItem.id === ruleId ? update(ruleItem) : ruleItem)),
    constraints: spec.constraints.map((ruleItem) => (ruleItem.id === ruleId ? update(ruleItem) : ruleItem)),
  };
}
