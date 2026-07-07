import { FormEvent, useEffect, useState } from "react";

import { useI18n } from "../../app/i18n";
import { desktop } from "../../lib/desktop";
import type { Provider, ProviderKind } from "../../lib/desktop";
import {
  clearProviderVerification,
  isProviderVerified,
  markProviderVerified,
} from "../../lib/providerVerification";

const defaultBaseUrls: Record<ProviderKind, string> = {
  open_ai: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com",
  gemini: "https://generativelanguage.googleapis.com/v1beta",
  open_ai_compatible: "",
  anthropic_compatible: "",
};

type SettingsModalProps = {
  onClose: () => void;
};

export function SettingsModal({ onClose }: SettingsModalProps) {
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const [providers, setProviders] = useState<Provider[]>([]);
  const [view, setView] = useState<"list" | "add">("list");
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [testingId, setTestingId] = useState<string | null>(null);
  // bumped after verification changes so badges re-render
  const [, setVerificationVersion] = useState(0);

  const [kind, setKind] = useState<ProviderKind>("open_ai");
  const [name, setName] = useState("");
  const [baseUrl, setBaseUrl] = useState(defaultBaseUrls.open_ai);
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const loaded = await desktop.listProviders();
        if (!cancelled) {
          setProviders(loaded);
        }
      } catch (caught) {
        if (!cancelled) {
          setError(caught instanceof Error ? caught.message : String(caught));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const resetForm = () => {
    setKind("open_ai");
    setName("");
    setBaseUrl(defaultBaseUrls.open_ai);
    setModel("");
    setApiKey("");
  };

  const saveProvider = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    try {
      const saved = await desktop.saveProvider({
        name: name.trim(),
        kind,
        baseUrl: baseUrl.trim(),
        model: model.trim(),
        apiKey: apiKey || undefined,
      });
      clearProviderVerification(saved.id);
      setProviders((current) => [saved, ...current.filter((provider) => provider.id !== saved.id)]);
      resetForm();
      setView("list");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };

  const testProvider = async (provider: Provider) => {
    setError(null);
    setNotice(null);
    setTestingId(provider.id);
    try {
      const capabilities = await desktop.testProvider({ providerId: provider.id });
      if (capabilities.image_input) {
        markProviderVerified(provider.id);
        setNotice(
          isEnglish
            ? `${provider.name}: connection test passed with image input support.`
            : `${provider.name}：连接测试通过，支持图片输入。`,
        );
      } else {
        clearProviderVerification(provider.id);
        setNotice(
          isEnglish
            ? `${provider.name}: this model does not report image input support.`
            : `${provider.name}：该模型未报告图片输入支持。`,
        );
      }
    } catch (caught) {
      clearProviderVerification(provider.id);
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setTestingId(null);
      setVerificationVersion((version) => version + 1);
    }
  };

  const deleteProvider = async (provider: Provider) => {
    setError(null);
    try {
      await desktop.deleteProvider({ providerId: provider.id });
      clearProviderVerification(provider.id);
      setProviders((current) => current.filter((candidate) => candidate.id !== provider.id));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };

  return (
    <div
      className="modal-overlay"
      role="dialog"
      aria-modal="true"
      aria-label="Settings"
      onClick={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <div className="modal-win">
        <header className="modal-win__header">
          <span className="modal-win__title">{isEnglish ? "Settings" : "设置"}</span>
          <span className="modal-win__spacer" />
          <button className="button-quiet" type="button" aria-label="Close settings" onClick={onClose}>
            {isEnglish ? "Close" : "关闭"}
          </button>
        </header>

        <div className="modal-win__body">
          {error ? <p role="alert" className="error">{error}</p> : null}
          {notice ? <p role="status">{notice}</p> : null}

          {view === "list" ? (
            <section aria-label="AI models">
              <div className="settings-section__head">
                <h3>{isEnglish ? "AI models" : "AI 模型"}</h3>
                <button
                  className="button-secondary"
                  type="button"
                  aria-label="Add AI model"
                  onClick={() => {
                    setError(null);
                    setNotice(null);
                    setView("add");
                  }}
                >
                  ＋
                </button>
              </div>
              <p className="muted settings-note">
                {isEnglish
                  ? "Connection testing is optional. If you skip it, you are responsible for the model's availability and output quality."
                  : "测试连接为可选项；跳过测试时，模型的可用性与结果质量由你自行负责。"}
              </p>

              {loading ? <p>{isEnglish ? "Loading models…" : "正在加载模型…"}</p> : null}
              {!loading && providers.length === 0 ? (
                <p className="muted">
                  {isEnglish ? "No models yet. Add one with the plus button." : "还没有模型，点击加号添加。"}
                </p>
              ) : null}

              <ul className="provider-list" aria-label="Saved providers">
                {providers.map((provider) => (
                  <li className="card provider-item" key={provider.id}>
                    <p>
                      <strong>{provider.name}</strong> · <span className="mono">{provider.model}</span>
                    </p>
                    <p className="muted">
                      {provider.has_credential
                        ? isEnglish
                          ? "Stored securely"
                          : "已安全保存凭证"
                        : isEnglish
                          ? "No API key stored"
                          : "未保存 API Key"}
                      {" · "}
                      {isProviderVerified(provider.id)
                        ? isEnglish
                          ? "connection tested"
                          : "已通过连接测试"
                        : isEnglish
                          ? "not tested"
                          : "未测试"}
                    </p>
                    <div className="provider-item__actions">
                      <button
                        className="button-secondary"
                        type="button"
                        aria-label={`Test connection for ${provider.name}`}
                        disabled={testingId === provider.id}
                        onClick={() => void testProvider(provider)}
                      >
                        {testingId === provider.id
                          ? isEnglish
                            ? "Testing…"
                            : "测试中…"
                          : isEnglish
                            ? "Test connection"
                            : "测试连接"}
                      </button>
                      <button
                        className="button-quiet button-danger-text"
                        type="button"
                        aria-label={`Delete ${provider.name}`}
                        onClick={() => void deleteProvider(provider)}
                      >
                        {isEnglish ? "Delete" : "删除"}
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            </section>
          ) : (
            <section aria-label="Add AI model">
              <div className="settings-section__head">
                <h3>{isEnglish ? "Add AI model" : "添加 AI 模型"}</h3>
                <button
                  className="button-quiet"
                  type="button"
                  aria-label="Back to model list"
                  onClick={() => setView("list")}
                >
                  {isEnglish ? "Back" : "返回"}
                </button>
              </div>
              <form className="form-grid" onSubmit={saveProvider}>
                <label className="field">
                  {isEnglish ? "Provider type" : "Provider 类型"}
                  <select
                    aria-label="Provider type"
                    value={kind}
                    onChange={(event) => {
                      const nextKind = event.target.value as ProviderKind;
                      setKind(nextKind);
                      setBaseUrl(defaultBaseUrls[nextKind]);
                    }}
                  >
                    <option value="open_ai">OpenAI</option>
                    <option value="anthropic">Anthropic</option>
                    <option value="gemini">Gemini</option>
                    <option value="open_ai_compatible">OpenAI-compatible</option>
                    <option value="anthropic_compatible">Anthropic-compatible</option>
                  </select>
                </label>
                <label className="field">
                  {isEnglish ? "Provider name" : "Provider 名称"}
                  <input
                    aria-label="Provider name"
                    value={name}
                    onChange={(event) => setName(event.target.value)}
                  />
                </label>
                <label className="field">
                  Base URL
                  <input value={baseUrl} onChange={(event) => setBaseUrl(event.target.value)} />
                </label>
                <label className="field">
                  {isEnglish ? "Model name" : "模型名称"}
                  <input
                    aria-label="Model name"
                    value={model}
                    onChange={(event) => setModel(event.target.value)}
                  />
                </label>
                <label className="field">
                  API key
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(event) => setApiKey(event.target.value)}
                  />
                </label>
                <p className="muted settings-note">
                  {isEnglish
                    ? "The key is stored by the desktop backend and never refilled into this form."
                    : "密钥只交给桌面后端保存，前端不会回填或读取明文。"}
                </p>
                <button className="button-primary" type="submit" aria-label="Save provider">
                  {isEnglish ? "Save provider" : "保存 Provider"}
                </button>
              </form>
            </section>
          )}
        </div>
      </div>
    </div>
  );
}
