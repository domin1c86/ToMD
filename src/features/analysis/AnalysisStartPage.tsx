import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import { useI18n } from "../../app/i18n";
import { desktop } from "../../lib/desktop";
import type { AnalysisPreview, Provider, Screenshot } from "../../lib/desktop";

export function AnalysisStartPage() {
  const { projectId = "" } = useParams();
  const navigate = useNavigate();
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const [providers, setProviders] = useState<Provider[]>([]);
  const [screenshots, setScreenshots] = useState<Screenshot[]>([]);
  const [preview, setPreview] = useState<AnalysisPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [analyzing, setAnalyzing] = useState(false);

  const selectedProvider = providers[0] ?? null;
  const selectedScreenshotIds = useMemo(
    () => screenshots.map((screenshot) => screenshot.id),
    [screenshots],
  );

  useEffect(() => {
    let cancelled = false;

    async function loadPreview() {
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
        const provider = loadedProviders[0];
        const screenshotIds = loadedScreenshots.map((screenshot) => screenshot.id);
        if (!provider || screenshotIds.length === 0) {
          setPreview(null);
          return;
        }

        const nextPreview = await desktop.previewAnalysisRequest({
          projectId,
          providerId: provider.id,
          screenshotIds,
        });
        if (!cancelled) {
          setPreview(nextPreview);
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
    }

    void loadPreview();

    return () => {
      cancelled = true;
    };
  }, [projectId]);

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
      {loading ? <p>Preparing disclosure…</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      {!loading && !selectedProvider ? <p>No provider configured.</p> : null}
      {!loading && selectedProvider && selectedScreenshotIds.length === 0 ? (
        <p>No screenshots selected for analysis.</p>
      ) : null}

      {preview ? (
        <section className="card" aria-label="What leaves this device">
          <h3>{isEnglish ? "What will be sent" : "会发送什么"}</h3>
          <p>Provider: {preview.provider_name}</p>
          <p>Model: {preview.model}</p>
          <p>{preview.image_count} images will be sent</p>
          <p>Selected image IDs: {preview.image_ids.join(", ")}</p>
          <p>Estimated encoded payload: {preview.estimated_encoded_bytes} bytes</p>
          <p>Only the selected screenshots and analysis prompt are sent to the configured provider.</p>
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
