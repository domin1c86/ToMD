import { FormEvent, useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import { desktop } from "../../lib/desktop";
import type { Provider, ProviderCapabilities, ProviderKind } from "../../lib/desktop";

const defaultBaseUrls: Record<ProviderKind, string> = {
  open_ai: "https://api.openai.com/v1",
  anthropic: "https://api.anthropic.com",
  gemini: "https://generativelanguage.googleapis.com/v1beta",
  open_ai_compatible: "",
};

export function ProviderSettingsPage() {
  const { projectId = "" } = useParams();
  const navigate = useNavigate();
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
    <section>
      <h2>Provider setup</h2>
      <p>API keys are stored by the desktop backend and are never refilled into this form.</p>

      {loading ? <p>Loading providers…</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      <form onSubmit={saveProvider}>
        <label>
          Provider type
          <select
            value={kind}
            onChange={(event) => handleKindChange(event.target.value as ProviderKind)}
          >
            <option value="open_ai">OpenAI</option>
            <option value="anthropic">Anthropic</option>
            <option value="gemini">Gemini</option>
            <option value="open_ai_compatible">OpenAI-compatible</option>
          </select>
        </label>
        <label>
          Provider name
          <input
            value={name}
            onChange={(event) => {
              setName(event.target.value);
              invalidateConnectionTest();
            }}
          />
        </label>
        <label>
          Base URL
          <input
            value={baseUrl}
            onChange={(event) => {
              setBaseUrl(event.target.value);
              invalidateConnectionTest();
            }}
          />
        </label>
        <label>
          Model name
          <input
            value={model}
            onChange={(event) => {
              setModel(event.target.value);
              invalidateConnectionTest();
            }}
          />
        </label>
        <label>
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
        <button type="submit">Save provider</button>
      </form>

      {providers.length > 0 ? (
        <ul aria-label="Saved providers">
          {providers.map((provider) => (
            <li key={provider.id}>
              <p>Provider: {provider.name}</p>
              <p>Model: {provider.model}</p>
              <p>{provider.has_credential ? "Stored securely" : "No API key stored"}</p>
              <button type="button" onClick={() => void testProvider(provider)}>
                Test connection for {provider.name}
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
        type="button"
        disabled={!canContinue}
        onClick={() => navigate(`/projects/${projectId}/analyze`)}
      >
        Continue to disclosure
      </button>
    </section>
  );
}
