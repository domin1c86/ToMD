import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { App } from "../../app/App";
import { desktop, type Provider } from "../../lib/desktop";

vi.mock("../../lib/desktop", () => ({
  desktop: {
    listProjects: vi.fn(),
    listProviders: vi.fn(),
    saveProvider: vi.fn(),
    deleteProvider: vi.fn(),
    testProvider: vi.fn(),
    fetchProviderModels: vi.fn(),
  },
}));

const mockedDesktop = vi.mocked(desktop);

describe("SettingsModal", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    localStorage.clear();
    window.history.pushState({}, "", "/");
    mockedDesktop.listProjects.mockResolvedValue([]);
    mockedDesktop.listProviders.mockResolvedValue([]);
    mockedDesktop.saveProvider.mockResolvedValue(provider());
    mockedDesktop.deleteProvider.mockResolvedValue(undefined);
    mockedDesktop.testProvider.mockResolvedValue({
      image_input: true,
      structured_output: true,
    });
  });

  async function openSettings() {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Open settings" }));
    return user;
  }

  it("adds an AI model from the settings modal without exposing the key", async () => {
    const user = await openSettings();

    expect(await screen.findByText("AI 模型")).toBeVisible();
    expect(
      screen.getByText("测试连接为可选项；跳过测试时，模型的可用性与结果质量由你自行负责。"),
    ).toBeVisible();

    await user.click(screen.getByRole("button", { name: "Add AI model" }));
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
    expect(await screen.findByText("已安全保存凭证", { exact: false })).toBeVisible();
    expect(screen.queryByDisplayValue("sk-secret")).not.toBeInTheDocument();
    expect(screen.getByText("未测试", { exact: false })).toBeVisible();
  });

  it("records verification on a passing optional connection test", async () => {
    mockedDesktop.listProviders.mockResolvedValue([provider()]);
    const user = await openSettings();

    await user.click(
      await screen.findByRole("button", { name: "Test connection for My endpoint" }),
    );

    expect(mockedDesktop.testProvider).toHaveBeenCalledWith({ providerId: "provider-1" });
    expect(await screen.findByText("My endpoint：连接测试通过，支持图片输入。")).toBeVisible();
    expect(localStorage.getItem("dle.verifiedProviderIds")).toContain("provider-1");
  });

  it("clears verification when the model lacks image input", async () => {
    mockedDesktop.listProviders.mockResolvedValue([provider()]);
    mockedDesktop.testProvider.mockResolvedValue({
      image_input: false,
      structured_output: true,
    });
    const user = await openSettings();

    await user.click(
      await screen.findByRole("button", { name: "Test connection for My endpoint" }),
    );

    expect(await screen.findByText("My endpoint：该模型未报告图片输入支持。")).toBeVisible();
    expect(localStorage.getItem("dle.verifiedProviderIds") ?? "[]").not.toContain("provider-1");
  });

  it("deletes a model", async () => {
    mockedDesktop.listProviders.mockResolvedValue([provider()]);
    const user = await openSettings();

    await user.click(await screen.findByRole("button", { name: "Delete My endpoint" }));

    expect(mockedDesktop.deleteProvider).toHaveBeenCalledWith({ providerId: "provider-1" });
    expect(screen.queryByText("My endpoint", { exact: false })).not.toBeInTheDocument();
  });

  it("edits an existing model, keeping the stored key when left empty", async () => {
    mockedDesktop.listProviders.mockResolvedValue([provider()]);
    mockedDesktop.saveProvider.mockResolvedValue(provider({ model: "vision-large" }));
    const user = await openSettings();

    await user.click(await screen.findByRole("button", { name: "Edit My endpoint" }));

    expect(await screen.findByLabelText("Provider name")).toHaveValue("My endpoint");
    expect(screen.getByLabelText("Model name")).toHaveValue("vision-model");
    expect(screen.getByText("留空则沿用已保存的 Key。密钥只交给桌面后端保存，前端不会回填。")).toBeVisible();

    await user.clear(screen.getByLabelText("Model name"));
    await user.type(screen.getByLabelText("Model name"), "vision-large");
    await user.click(screen.getByRole("button", { name: "Save provider" }));

    expect(mockedDesktop.saveProvider).toHaveBeenCalledWith({
      providerId: "provider-1",
      name: "My endpoint",
      kind: "open_ai_compatible",
      baseUrl: "https://ai.example.com/v1",
      model: "vision-large",
      apiKey: undefined,
    });
    expect(await screen.findByText("vision-large")).toBeVisible();
  });

  it("fetches available models and turns the model field into a dropdown", async () => {
    mockedDesktop.fetchProviderModels.mockResolvedValue(["vision-large", "vision-mini"]);
    mockedDesktop.saveProvider.mockResolvedValue(provider({ model: "vision-mini" }));
    const user = await openSettings();

    await user.click(screen.getByRole("button", { name: "Add AI model" }));
    await user.selectOptions(await screen.findByLabelText("Provider type"), "open_ai_compatible");
    await user.type(screen.getByLabelText("Provider name"), "My endpoint");
    await user.type(screen.getByLabelText("Base URL"), "https://ai.example.com/v1");
    await user.type(screen.getByLabelText("API key"), "sk-secret");
    await user.click(screen.getByRole("button", { name: "Fetch models" }));

    expect(mockedDesktop.fetchProviderModels).toHaveBeenCalledWith({
      kind: "open_ai_compatible",
      baseUrl: "https://ai.example.com/v1",
      apiKey: "sk-secret",
      providerId: undefined,
    });
    const modelSelect = await screen.findByLabelText("Model name");
    expect(modelSelect.tagName).toBe("SELECT");
    await user.selectOptions(modelSelect, "vision-mini");
    await user.click(screen.getByRole("button", { name: "Save provider" }));

    expect(mockedDesktop.saveProvider).toHaveBeenCalledWith(
      expect.objectContaining({ model: "vision-mini" }),
    );
  });

  it("requires a key before fetching models for a new provider", async () => {
    const user = await openSettings();

    await user.click(screen.getByRole("button", { name: "Add AI model" }));
    await user.type(await screen.findByLabelText("Base URL"), "https://ai.example.com/v1");
    await user.click(screen.getByRole("button", { name: "Fetch models" }));

    expect(await screen.findByText("请先填写 API Key，再获取模型列表。")).toBeVisible();
    expect(mockedDesktop.fetchProviderModels).not.toHaveBeenCalled();
  });

  it("shows save errors without leaking the secret and closes on demand", async () => {
    mockedDesktop.saveProvider.mockRejectedValue(new Error("invalid key"));
    const user = await openSettings();

    await user.click(screen.getByRole("button", { name: "Add AI model" }));
    await user.type(await screen.findByLabelText("Provider name"), "Bad endpoint");
    await user.type(screen.getByLabelText("Model name"), "vision-model");
    await user.type(screen.getByLabelText("API key"), "sk-invalid");
    await user.click(screen.getByRole("button", { name: "Save provider" }));

    expect(await screen.findByText("invalid key")).toBeVisible();
    expect(screen.queryByText("sk-invalid")).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Close settings" }));
    expect(screen.queryByRole("dialog", { name: "Settings" })).not.toBeInTheDocument();
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
