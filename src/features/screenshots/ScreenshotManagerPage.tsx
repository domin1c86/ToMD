import { FormEvent, useEffect, useMemo, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { desktop } from "../../lib/desktop";
import type { Project, Screenshot } from "../../lib/desktop";

type EditableScreenshot = Screenshot & {
  draftPageName: string;
  draftScene: string;
};

export function ScreenshotManagerPage() {
  const { projectId = "" } = useParams();
  const navigate = useNavigate();
  const [project, setProject] = useState<Project | null>(null);
  const [screenshots, setScreenshots] = useState<EditableScreenshot[]>([]);
  const [paths, setPaths] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const sortedScreenshots = useMemo(
    () => [...screenshots].sort((a, b) => a.sort_order - b.sort_order),
    [screenshots],
  );

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const [projects, loadedScreenshots] = await Promise.all([
        desktop.listProjects({ includeArchived: false }),
        desktop.listScreenshots({ projectId }),
      ]);
      setProject(projects.find((item) => item.id === projectId) ?? null);
      setScreenshots(loadedScreenshots.map(toEditableScreenshot));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, [projectId]);

  const importScreenshots = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const parsedPaths = paths
      .split(/\r?\n|,/)
      .map((path) => path.trim())
      .filter(Boolean);
    if (parsedPaths.length === 0) {
      setError("Add at least one local screenshot path.");
      return;
    }

    try {
      await desktop.importScreenshots({ projectId, paths: parsedPaths });
      setPaths("");
      await load();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };

  const updateDraft = (
    screenshotId: string,
    patch: Partial<Pick<EditableScreenshot, "draftPageName" | "draftScene">>,
  ) => {
    setScreenshots((current) =>
      current.map((screenshot) =>
        screenshot.id === screenshotId ? { ...screenshot, ...patch } : screenshot,
      ),
    );
  };

  const saveMetadata = async (screenshot: EditableScreenshot) => {
    try {
      await desktop.updateScreenshotMetadata({
        projectId,
        screenshotId: screenshot.id,
        pageName: screenshot.draftPageName,
        scene: screenshot.draftScene,
        sortOrder: screenshot.sort_order,
      });
      await load();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };

  const removeScreenshot = async (screenshot: Screenshot) => {
    try {
      await desktop.removeScreenshot({ projectId, screenshotId: screenshot.id });
      await load();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };

  return (
    <section>
      <Link to="/">Projects</Link>
      <h2>Reference screenshots</h2>
      {project ? (
        <p>
          {project.name} · {project.platform}
        </p>
      ) : null}
      {loading ? <p>Loading screenshots…</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      <form onSubmit={importScreenshots}>
        <label>
          Local screenshot paths
          <textarea value={paths} onChange={(event) => setPaths(event.target.value)} />
        </label>
        <button type="submit">Import screenshots</button>
      </form>

      {sortedScreenshots.length === 0 && !loading ? <p>No screenshots imported yet.</p> : null}
      {sortedScreenshots.length > 0 && sortedScreenshots.length < 3 ? (
        <p>Recommendation: import at least 3 screenshots for stronger patterns.</p>
      ) : null}

      <button
        type="button"
        disabled={sortedScreenshots.length === 0}
        onClick={() => navigate(`/projects/${projectId}/providers`)}
      >
        Configure analysis
      </button>

      {sortedScreenshots.length > 0 ? (
        <table>
          <thead>
            <tr>
              <th>Page</th>
              <th>Scene</th>
              <th>Dimensions</th>
              <th>Media</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {sortedScreenshots.map((screenshot) => (
              <tr key={screenshot.id}>
                <td>
                  <label>
                    Page name for {screenshot.page_name}
                    <input
                      value={screenshot.draftPageName}
                      onChange={(event) =>
                        updateDraft(screenshot.id, { draftPageName: event.target.value })
                      }
                    />
                  </label>
                </td>
                <td>
                  <label>
                    Scene for {screenshot.page_name}
                    <input
                      value={screenshot.draftScene}
                      onChange={(event) =>
                        updateDraft(screenshot.id, { draftScene: event.target.value })
                      }
                    />
                  </label>
                </td>
                <td>
                  {screenshot.width} × {screenshot.height}
                </td>
                <td>{screenshot.media_type}</td>
                <td>
                  <button type="button" onClick={() => void saveMetadata(screenshot)}>
                    Save metadata for {screenshot.page_name}
                  </button>
                  <button type="button" onClick={() => void removeScreenshot(screenshot)}>
                    Remove {screenshot.page_name}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </section>
  );
}

function toEditableScreenshot(screenshot: Screenshot): EditableScreenshot {
  return {
    ...screenshot,
    draftPageName: screenshot.page_name,
    draftScene: screenshot.scene,
  };
}
