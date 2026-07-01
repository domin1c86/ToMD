import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import { desktop } from "../../lib/desktop";
import type { AnalysisPreview, Provider, Screenshot } from "../../lib/desktop";

export function AnalysisStartPage() {
  const { projectId = "" } = useParams();
  const navigate = useNavigate();
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
    <section>
      <h2>Transmission disclosure</h2>
      {loading ? <p>Preparing disclosure…</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      {!loading && !selectedProvider ? <p>No provider configured.</p> : null}
      {!loading && selectedProvider && selectedScreenshotIds.length === 0 ? (
        <p>No screenshots selected for analysis.</p>
      ) : null}

      {preview ? (
        <section aria-label="What leaves this device">
          <p>Provider: {preview.provider_name}</p>
          <p>Model: {preview.model}</p>
          <p>{preview.image_count} images will be sent</p>
          <p>Selected image IDs: {preview.image_ids.join(", ")}</p>
          <p>Estimated encoded payload: {preview.estimated_encoded_bytes} bytes</p>
          <p>Only the selected screenshots and analysis prompt are sent to the configured provider.</p>
        </section>
      ) : null}

      <button
        type="button"
        disabled={!preview || analyzing}
        onClick={() => void sendAnalysis()}
      >
        Send and analyze
      </button>
    </section>
  );
}
