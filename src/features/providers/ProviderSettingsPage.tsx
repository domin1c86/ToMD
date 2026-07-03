import { FormEvent, useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import { useI18n } from "../../app/i18n";
import { desktop } from "../../lib/desktop";
import type { Provider, ProviderCapabilities, ProviderKind } from "../../lib/desktop";

const defaultBaseUrls: Record<ProviderKind, string> = {
  open_ai: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com",
  gemini: "https://generativelanguage.googleapis.com/v1beta",
  open_ai_compatible: "",
  anthropic_compatible: "",
};

export function ProviderSettingsPage() {
  const { projectId = "" } = useParams();
  const navigate = useNavigate();
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const [providers, setProviders] = useState<Provider[]>([]);
  const [name, setName] = useState("");
  const [kind, setKind] = useState<ProviderKind>("open_ai");
  const [baseUrl, setBaseUrl] = useState(defaultBaseUrls.open_ai);
  const [model, setModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [testedProviderId, setTestedProviderId] = useState<string | null>(null);
  const [capabilities, setCapabilities] = useState<ProviderCapabilities | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const loadProviders = async () => {
    setLoading(true);
    setError(null);
    try {
      setProviders(await desktop.listProviders());
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadProviders();
  }, []);

  const handleKindChange = (nextKind: ProviderKind) => {
    setKind(nextKind);
    setBaseUrl(defaultBaseUrls[nextKind]);
    invalidateConnectionTest();
  };

  const invalidateConnectionTest = () => {
    setTestedProviderId(null);
    setCapabilities(null);
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
      setApiKey("");
      setProviders((current) => [saved, ...current.filter((provider) => provider.id !== saved.id)]);
      invalidateConnectionTest();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };

  const testProvider = async (provider: Provider) => {
    setError(null);
    try {
      const nextCapabilities = await desktop.testProvider({ providerId: provider.id });
      setTestedProviderId(provider.id);
      setCapabilities(nextCapabilities);
    } catch (caught) {
      setTestedProviderId(null);
      setCapabilities(null);
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };

  const canContinue = testedProviderId !== null && capabilities?.image_input === true;

  return (
    <section className="page-grid">
      <div className="page-panel">
      <div className="page-header">
        <div>
          <h2 aria-label="Provider setup">{isEnglish ? "Configure AI model" : "配置 AI 模型"}</h2>
          <p>Provider setup</p>
          <p>
            {isEnglish
              ? "Choose a real multimodal API or compatible endpoint. Save it, then test the connection before analysis."
              : "选择真实多模态 API 或兼容端点。保存后必须测试连接，确认模型支持图片输入。"}
          </p>
          <p>API keys are stored by the desktop backend and are never refilled into this form.</p>
        </div>
      </div>

      {loading ? <p>Loading providers…</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      <div className="card" style={{ marginBottom: "1rem" }}>
        <h3>{isEnglish ? "How to choose compatible endpoints" : "兼容端点怎么选？"}</h3>
        <p className="muted">
          {isEnglish
            ? "OpenAI-compatible is for third-party endpoints compatible with Chat Completions."
            : "OpenAI-compatible 适合兼容 Chat Completions 的第三方端点。"}
        </p>
        <p className="muted">
          {isEnglish
            ? "Anthropic-compatible is for third-party endpoints compatible with the Messages API."
            : "Anthropic-compatible 适合兼容 Messages API 的第三方端点。"}
        </p>
      </div>

      <form className="form-grid" onSubmit={saveProvider}>
        <label className="field">
          {isEnglish ? "Provider type" : "Provider 类型"}
          <span aria-hidden="true">Provider type</span>
          <select
            aria-label="Provider type"
            value={kind}
            onChange={(event) => handleKindChange(event.target.value as ProviderKind)}
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
          <span aria-hidden="true">Provider name</span>
          <input
            aria-label="Provider name"
            value={name}
            onChange={(event) => {
              setName(event.target.value);
              invalidateConnectionTest();
            }}
          />
        </label>
        <label className="field">
          Base URL
          <input
            value={baseUrl}
            onChange={(event) => {
              setBaseUrl(event.target.value);
              invalidateConnectionTest();
            }}
          />
        </label>
        <label className="field">
          {isEnglish ? "Model name" : "模型名称"}
          <span aria-hidden="true">Model name</span>
          <input
            aria-label="Model name"
            value={model}
            onChange={(event) => {
              setModel(event.target.value);
              invalidateConnectionTest();
            }}
          />
        </label>
        <label className="field">
          API key
          <input
            type="password"
            value={apiKey}
            onChange={(event) => {
              setApiKey(event.target.value);
              invalidateConnectionTest();
            }}
          />
        </label>
        <button className="button-primary" type="submit" aria-label="Save provider">
          {isEnglish ? "Save provider" : "保存 Provider"}
        </button>
      </form>

      {providers.length > 0 ? (
        <ul aria-label="Saved providers">
          {providers.map((provider) => (
            <li className="card" key={provider.id}>
              <p>Provider: {provider.name}</p>
              <p>Model: {provider.model}</p>
              <p>{formatCredentialStatus(provider.has_credential, isEnglish)}</p>
              {provider.has_credential && !isEnglish ? <p>Stored securely</p> : null}
              <button
                className="button-secondary"
                type="button"
                aria-label={`Test connection for ${provider.name}`}
                onClick={() => void testProvider(provider)}
              >
                {isEnglish ? `Test connection for ${provider.name}` : `测试 ${provider.name} 连接`}
              </button>
            </li>
          ))}
        </ul>
      ) : null}

      {testedProviderId && capabilities?.image_input === false ? (
        <p role="alert">This model does not report image input support.</p>
      ) : null}
      {testedProviderId && capabilities?.image_input === true ? (
        <p>Connection test passed with image input support.</p>
      ) : null}

      <button
        className="button-primary"
        type="button"
        disabled={!canContinue}
        aria-label="Continue to disclosure"
        onClick={() => navigate(`/projects/${projectId}/analyze`)}
      >
        {isEnglish ? "Next: review disclosure" : "下一步：查看发送披露"}
      </button>
      </div>
      <aside className="help-panel">
        <h3>{isEnglish ? "Where is your API key stored?" : "你的 API Key 存在哪里？"}</h3>
        <p>
          {isEnglish
            ? "The key is stored only by the desktop backend. The frontend never refills or reads plaintext credentials."
            : "密钥只交给桌面后端保存，前端不会回填或读取明文。"}
        </p>
        <hr />
        <h3>{isEnglish ? "What does the test check?" : "测试连接检查什么？"}</h3>
        <p>
          {isEnglish
            ? "It verifies credentials and confirms the selected model supports image input. You cannot continue until the test passes."
            : "确认凭证可用，并确认当前模型支持图片输入。未通过测试时不能继续分析。"}
        </p>
      </aside>
    </section>
  );
}

function formatCredentialStatus(hasCredential: boolean, isEnglish: boolean): string {
  if (hasCredential) {
    return isEnglish ? "Stored securely" : "已安全保存凭证";
  }
  return isEnglish ? "No API key stored" : "未保存 API Key";
}
