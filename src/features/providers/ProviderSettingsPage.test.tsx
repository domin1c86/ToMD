import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../../app/App";
import { desktop, type Provider } from "../../lib/desktop";

vi.mock("../../lib/desktop", () => ({
  desktop: {
    listProviders: vi.fn(),
    saveProvider: vi.fn(),
    testProvider: vi.fn(),
  },
}));

const mockedDesktop = vi.mocked(desktop);

describe("ProviderSettingsPage", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    localStorage.clear();
    window.history.pushState({}, "", "/projects/project-1/providers");
    mockedDesktop.listProviders.mockResolvedValue([]);
    mockedDesktop.saveProvider.mockResolvedValue(provider());
    mockedDesktop.testProvider.mockResolvedValue({
      image_input: true,
      structured_output: true,
    });
  });

  it("saves an OpenAI-compatible provider and shows only a secure key placeholder", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect((await screen.findAllByText("配置 AI 模型")).length).toBeGreaterThan(0);
    expect(screen.getByText("OpenAI-compatible 适合兼容 Chat Completions 的第三方端点。")).toBeVisible();
    await user.selectOptions(await screen.findByLabelText("Provider type"), "open_ai_compatible");
    await user.type(screen.getByLabelText("Provider name"), "My endpoint");
    await user.type(screen.getByLabelText("Base URL"), "https://ai.example.com/v1");
    await user.type(screen.getByLabelText("Model name"), "vision-model");
    await user.type(screen.getByLabelText("API key"), "sk-secret");
    await user.click(screen.getByRole("button", { name: "Save provider" }));

    expect(mockedDesktop.saveProvider).toHaveBeenCalledWith({
      name: "My endpoint",
      kind: "open_ai_compatible",
      baseUrl: "https://ai.example.com/v1",
      model: "vision-model",
      apiKey: "sk-secret",
    });
    expect(await screen.findByText("Stored securely")).toBeVisible();
    expect(screen.queryByDisplayValue("sk-secret")).not.toBeInTheDocument();
  });

  it("saves an Anthropic-compatible provider endpoint", async () => {
    const user = userEvent.setup();
    mockedDesktop.saveProvider.mockResolvedValue(
      provider({
        kind: "anthropic_compatible",
        base_url: "https://claude-compatible.example.com",
      }),
    );
    render(<App />);

    await user.selectOptions(await screen.findByLabelText("Provider type"), "anthropic_compatible");
    await user.type(screen.getByLabelText("Provider name"), "Claude-compatible endpoint");
    await user.type(screen.getByLabelText("Base URL"), "https://claude-compatible.example.com");
    await user.type(screen.getByLabelText("Model name"), "third-party-vision");
    await user.type(screen.getByLabelText("API key"), "sk-secret");
    await user.click(screen.getByRole("button", { name: "Save provider" }));

    expect(mockedDesktop.saveProvider).toHaveBeenCalledWith({
      name: "Claude-compatible endpoint",
      kind: "anthropic_compatible",
      baseUrl: "https://claude-compatible.example.com",
      model: "third-party-vision",
      apiKey: "sk-secret",
    });
    expect(await screen.findByText("Stored securely")).toBeVisible();
  });

  it("tests provider connectivity and blocks analysis when image input is unsupported", async () => {
    const user = userEvent.setup();
    mockedDesktop.listProviders.mockResolvedValue([provider()]);
    mockedDesktop.testProvider.mockResolvedValue({
      image_input: false,
      structured_output: true,
    });

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Test connection for My endpoint" }));

    expect(mockedDesktop.testProvider).toHaveBeenCalledWith({ providerId: "provider-1" });
    expect(await screen.findByText("This model does not report image input support.")).toBeVisible();
    expect(screen.getByRole("button", { name: "Continue to disclosure" })).toBeDisabled();
    expect(localStorage.getItem("dle.verifiedProviderIds") ?? "[]").not.toContain("provider-1");
  });

  it("records the verified provider after a passing connection test", async () => {
    const user = userEvent.setup();
    mockedDesktop.listProviders.mockResolvedValue([provider()]);

    render(<App />);

    await user.click(await screen.findByRole("button", { name: "Test connection for My endpoint" }));

    expect(await screen.findByText("Connection test passed with image input support.")).toBeVisible();
    expect(localStorage.getItem("dle.verifiedProviderIds")).toContain("provider-1");
    expect(localStorage.getItem("dle.lastVerifiedProviderId")).toBe("provider-1");
    expect(screen.getByRole("button", { name: "Continue to disclosure" })).toBeEnabled();
  });

  it("shows invalid key errors without exposing the secret", async () => {
    const user = userEvent.setup();
    mockedDesktop.saveProvider.mockRejectedValue(new Error("invalid key"));

    render(<App />);

    await user.type(await screen.findByLabelText("Provider name"), "Bad endpoint");
    await user.type(screen.getByLabelText("Model name"), "vision-model");
    await user.type(screen.getByLabelText("API key"), "sk-invalid");
    await user.click(screen.getByRole("button", { name: "Save provider" }));

    expect(await screen.findByText("invalid key")).toBeVisible();
    expect(screen.queryByText("sk-invalid")).not.toBeInTheDocument();
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
