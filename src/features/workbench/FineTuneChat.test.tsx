import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../../app/App";
import { desktop } from "../../lib/desktop";
import { markProviderVerified } from "../../lib/providerVerification";
import type { DesignSpec, Rule } from "../../generated/bindings";

vi.mock("../../lib/desktop", () => ({
  desktop: {
    getDesignSpec: vi.fn(),
    updateRule: vi.fn(),
    listScreenshots: vi.fn(),
    screenshotUrl: vi.fn(),
    refineRules: vi.fn(),
  },
}));

const mockedDesktop = vi.mocked(desktop);

describe("FineTuneChat", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    localStorage.clear();
    markProviderVerified("provider-9");
    window.history.pushState({}, "", "/projects/project-1/workbench");
    mockedDesktop.getDesignSpec.mockResolvedValue(specFixture());
    mockedDesktop.listScreenshots.mockResolvedValue([]);
    mockedDesktop.screenshotUrl.mockReturnValue("asset://localhost/shot-1.png");
    mockedDesktop.refineRules.mockResolvedValue({
      spec: specFixture({ statement: "Use 12-16px card radii.", status: "edited" }),
      affected_rule_ids: ["rule-radius"],
    });
  });

  it("applies an instruction to all rules and shows the receipt with rule chips", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.type(
      await screen.findByLabelText("Refine instruction"),
      "Rewrite pixel values as ranges",
    );
    await user.click(screen.getByRole("button", { name: "Apply instruction" }));

    expect(mockedDesktop.refineRules).toHaveBeenCalledWith({
      projectId: "project-1",
      providerId: "provider-9",
      instruction: "Rewrite pixel values as ranges",
      ruleId: undefined,
    });
    expect(await screen.findByText("已调整 1 条规则，均标记为已编辑，等你确认：")).toBeVisible();
    expect(screen.getByRole("button", { name: "Use 12-16px card radii." })).toBeVisible();
    expect(screen.getByText("Status: edited")).toBeVisible();
  });

  it("scopes the instruction to the current rule when selected", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.selectOptions(await screen.findByLabelText("Instruction scope"), "current");
    await user.type(screen.getByLabelText("Refine instruction"), "Loosen this rule");
    await user.click(screen.getByRole("button", { name: "Apply instruction" }));

    expect(mockedDesktop.refineRules).toHaveBeenCalledWith({
      projectId: "project-1",
      providerId: "provider-9",
      instruction: "Loosen this rule",
      ruleId: "rule-radius",
    });
  });

  it("expands to the full history dialog and collapses again", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.type(await screen.findByLabelText("Refine instruction"), "Adjust rules");
    await user.click(screen.getByRole("button", { name: "Apply instruction" }));
    await screen.findByText("已调整 1 条规则，均标记为已编辑，等你确认：");

    await user.click(screen.getByRole("button", { name: "Expand chat history" }));
    const dialog = await screen.findByRole("dialog", { name: "Refine chat history" });
    expect(dialog).toBeVisible();
    expect(screen.getByText("Adjust rules")).toBeVisible();

    await user.click(screen.getByRole("button", { name: "Collapse chat" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("surfaces refine failures without mutating rules and supports dismissing bubbles", async () => {
    const user = userEvent.setup();
    mockedDesktop.refineRules.mockRejectedValue(new Error("provider returned HTTP status 400"));
    render(<App />);

    await user.type(await screen.findByLabelText("Refine instruction"), "Break things");
    await user.click(screen.getByRole("button", { name: "Apply instruction" }));

    expect(
      await screen.findByText("未能应用：provider returned HTTP status 400"),
    ).toBeVisible();
    expect(screen.getByRole("button", { name: "Use 12px card radii" })).toBeVisible();

    const dismissButtons = screen.getAllByRole("button", { name: "Dismiss message" });
    await user.click(dismissButtons[dismissButtons.length - 1]);
    expect(
      screen.queryByText("未能应用：provider returned HTTP status 400"),
    ).not.toBeInTheDocument();
  });

  it("disables the composer until a provider passed a connection test", async () => {
    localStorage.clear();
    render(<App />);

    expect(await screen.findByLabelText("Refine instruction")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Apply instruction" })).toBeDisabled();
    expect(screen.getByText("先在模型配置页通过连接测试，才能使用指令微调。")).toBeVisible();
  });
});

function specFixture(tokenOverrides: Partial<Rule> = {}): DesignSpec {
  return {
    metadata: {
      schema_version: "1.0.0",
      project_id: "project-1",
      platform: "web",
      provider_id: "provider-9",
      model: "vision-model",
      source_screenshot_ids: ["shot-1"],
      excluded_terms: [],
      created_at: "2026-07-01T00:00:00Z",
    },
    intent: [],
    tokens: [
      {
        id: "rule-radius",
        category: "tokens",
        statement: "Use 12px card radii",
        kind: "recommendation",
        scope: "global",
        value: null,
        evidence_ids: [],
        confidence: 0.9,
        status: "pending",
        source: "model",
        ...tokenOverrides,
      },
    ],
    layout: [],
    components: [],
    assets: [],
    motion: [],
    constraints: [],
    evidence: [],
    uncertainties: [],
  };
}
