import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import { useI18n } from "../../app/i18n";
import { desktop } from "../../lib/desktop";
import type { AnalysisPreview, Provider, Screenshot } from "../../lib/desktop";
import {
  isProviderVerified,
  lastVerifiedProviderId,
  selectedProviderId,
  setSelectedProviderId,
} from "../../lib/providerVerification";

function initialProviderId(providers: Provider[]): string | null {
  const stored = selectedProviderId();
  if (stored && providers.some((provider) => provider.id === stored)) {
    return stored;
  }
  const last = lastVerifiedProviderId();
  if (last && providers.some((provider) => provider.id === last)) {
    return last;
  }
  const verified = providers.find((provider) => isProviderVerified(provider.id));
  return verified?.id ?? providers[0]?.id ?? null;
}

export function AnalysisStartPage() {
  const { projectId = "" } = useParams();
  const navigate = useNavigate();
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const [providers, setProviders] = useState<Provider[]>([]);
  const [screenshots, setScreenshots] = useState<Screenshot[]>([]);
  const [currentProviderId, setCurrentProviderId] = useState<string | null>(null);
  const [preview, setPreview] = useState<AnalysisPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [analyzing, setAnalyzing] = useState(false);

  const selectedProvider = providers.find((provider) => provider.id === currentProviderId) ?? null;
  const providerVerified = selectedProvider !== null && isProviderVerified(selectedProvider.id);
  const selectedScreenshotIds = useMemo(
    () => screenshots.map((screenshot) => screenshot.id),
    [screenshots],
  );

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [loadedProviders, loadedScreenshots] = await Promise.all([
          desktop.listProviders(),
          desktop.listScreenshots({ projectId }),
        ]);
        if (cancelled) {
          return;
        }

        setProviders(loadedProviders);
        setScreenshots(loadedScreenshots);
        setCurrentProviderId(initialProviderId(loadedProviders));
      } catch (caught) {
        if (!cancelled) {
          setError(caught instanceof Error ? caught.message : String(caught));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void load();

    return () => {
      cancelled = true;
    };
  }, [projectId]);

  useEffect(() => {
    if (!currentProviderId || selectedScreenshotIds.length === 0) {
      setPreview(null);
      return;
    }

    let cancelled = false;

    async function loadPreview() {
      try {
        const nextPreview = await desktop.previewAnalysisRequest({
          projectId,
          providerId: currentProviderId as string,
          screenshotIds: selectedScreenshotIds,
        });
        if (!cancelled) {
          setPreview(nextPreview);
        }
      } catch (caught) {
        if (!cancelled) {
          setPreview(null);
          setError(caught instanceof Error ? caught.message : String(caught));
        }
      }
    }

    void loadPreview();

    return () => {
      cancelled = true;
    };
  }, [projectId, currentProviderId, selectedScreenshotIds]);

  const sendAnalysis = async () => {
    if (!selectedProvider || selectedScreenshotIds.length === 0) {
      return;
    }

    setAnalyzing(true);
    setError(null);
    try {
      await desktop.analyzeProject({
        projectId,
        providerId: selectedProvider.id,
        screenshotIds: selectedScreenshotIds,
      });
      navigate(`/projects/${projectId}/workbench`);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setAnalyzing(false);
    }
  };

  return (
    <section className="page-grid">
      <div className="page-panel">
      <div className="page-header">
        <div>
          <h2>{isEnglish ? "Confirm before sending" : "发送前确认"}</h2>
          <p>
            {isEnglish
              ? "Before this step, screenshots stay on this device. Confirm the provider, model, and image count before sending."
              : "在这一步之前，截图不会离开本机。请确认 Provider、模型和图片数量后再发送。"}
          </p>
        </div>
      </div>
      {loading ? <p>{isEnglish ? "Preparing disclosure…" : "正在准备发送披露…"}</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      {!loading && !selectedProvider ? (
        <p>
          {isEnglish
            ? "No provider configured. Add an AI model in the settings (top right)."
            : "尚未配置模型。请点击右上角「设置」添加 AI 模型。"}
        </p>
      ) : null}
      {!loading && selectedProvider && selectedScreenshotIds.length === 0 ? (
        <p>{isEnglish ? "No screenshots selected for analysis." : "没有可用于分析的截图。"}</p>
      ) : null}

      {providers.length > 0 ? (
        <label className="field">
          {isEnglish ? "Analysis provider" : "分析使用的 Provider"}
          <select
            aria-label="Analysis provider"
            value={currentProviderId ?? ""}
            onChange={(event) => {
              setCurrentProviderId(event.target.value);
              setSelectedProviderId(event.target.value);
            }}
          >
            {providers.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.name} ({provider.model})
              </option>
            ))}
          </select>
        </label>
      ) : null}

      {selectedProvider && !providerVerified ? (
        <p role="alert">
          {isEnglish
            ? "This model has not passed a connection test. You can still send, but you are responsible for its availability and output quality."
            : "该模型未通过连接测试。你仍然可以发送，但模型可用性与结果质量由你自行负责。"}
        </p>
      ) : null}

      {preview ? (
        <section className="card disclosure" aria-label="What leaves this device">
          <h3>{isEnglish ? "What will be sent" : "会发送什么"}</h3>
          <p>Provider: {preview.provider_name}</p>
          <p>Model: {preview.model}</p>
          <p>{preview.image_count} images will be sent</p>
          <p>Selected image IDs: {preview.image_ids.join(", ")}</p>
          <p>Estimated encoded payload: {preview.estimated_encoded_bytes} bytes</p>
          <p>
            {isEnglish
              ? "Only the listed screenshots and the analysis prompt are sent to the configured provider."
              : "只会把上面列出的截图和分析提示词发送给所配置的 Provider。"}
          </p>
        </section>
      ) : null}

      <button
        className="button-primary"
        type="button"
        aria-label="Send and analyze"
        disabled={!preview || analyzing}
        onClick={() => void sendAnalysis()}
      >
        {isEnglish ? "Send and analyze" : "发送并分析"}
      </button>
      </div>
      <aside className="help-panel">
        <h3>{isEnglish ? "What is not sent" : "不会发送什么"}</h3>
        <p>
          {isEnglish
            ? "Local paths, the database, export history, and saved plaintext API keys are not sent."
            : "不会发送本地路径、数据库、导出历史或已保存的 API Key 明文。"}
        </p>
        <hr />
        <h3>{isEnglish ? "What if it fails?" : "失败后怎么办？"}</h3>
        <p>
          {isEnglish
            ? "If the provider fails, project state is preserved. Return to provider setup to switch endpoint or model."
            : "如果 Provider 报错，项目状态会保留，你可以返回模型配置页更换端点或模型。"}
        </p>
      </aside>
    </section>
  );
}
