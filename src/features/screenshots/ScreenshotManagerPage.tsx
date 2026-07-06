import { FormEvent, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Link, useNavigate, useParams } from "react-router-dom";

import { useI18n } from "../../app/i18n";
import { desktop } from "../../lib/desktop";
import type { Project, Screenshot } from "../../lib/desktop";

type EditableScreenshot = Screenshot & {
  draftPageName: string;
  draftScene: string;
};

export function ScreenshotManagerPage() {
  const { projectId = "" } = useParams();
  const navigate = useNavigate();
  const { locale, t } = useI18n();
  const isEnglish = locale === "en-US";
  const [project, setProject] = useState<Project | null>(null);
  const [screenshots, setScreenshots] = useState<EditableScreenshot[]>([]);
  const [paths, setPaths] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectingFiles, setSelectingFiles] = useState(false);

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

  const chooseScreenshotFiles = async () => {
    setSelectingFiles(true);
    setError(null);
    try {
      const selected = await open({
        multiple: true,
        filters: [
          {
            name: "Images",
            extensions: ["png", "jpg", "jpeg", "webp"],
          },
        ],
      });
      const selectedPaths = Array.isArray(selected) ? selected : selected ? [selected] : [];
      if (selectedPaths.length === 0) {
        return;
      }
      await desktop.importScreenshots({ projectId, paths: selectedPaths });
      setPaths("");
      await load();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setSelectingFiles(false);
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
    <section className="page-grid">
      <div className="page-panel">
      <Link className="muted" to="/">Projects</Link>
      <div className="page-header">
        <div>
          <h2 aria-label="Reference screenshots">
            {isEnglish ? "Import reference screenshots" : "导入参考截图"}
          </h2>
          <p>
            {isEnglish
              ? "Import app or website screenshots, then organize page names, scenes, and dimensions."
              : "导入您的应用或网站截图，支持批量整理页面名称、场景和尺寸信息。"}
          </p>
        </div>
        <button
          className="button-primary"
          type="button"
          aria-label="Configure analysis"
          disabled={sortedScreenshots.length === 0}
          onClick={() => navigate(`/projects/${projectId}/providers`)}
        >
          {isEnglish ? "Next: configure provider" : "下一步：配置模型"}
        </button>
      </div>
      {project ? (
        <p>
          {project.name} · {project.platform}
        </p>
      ) : null}
      {loading ? <p>Loading screenshots…</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      <form className="form-grid" onSubmit={importScreenshots}>
        <div className="upload-zone">
          <div>
            <h3>{isEnglish ? "Drop images here, or paste local paths" : "拖拽图片到此处，或粘贴本地路径"}</h3>
            <p className="muted">
              {isEnglish
                ? "PNG, JPG, and WebP are supported. Include core pages and typical states."
                : "支持 PNG、JPG、WebP。建议包含完整核心页面与典型状态。"}
            </p>
            <p>{isEnglish ? "No images are sent before analysis." : "分析前不会发送任何图片。"}</p>
            <button
              className="button-primary"
              type="button"
              aria-label="Choose screenshot files"
              disabled={selectingFiles}
              onClick={() => void chooseScreenshotFiles()}
            >
              {selectingFiles
                ? isEnglish
                  ? "Opening picker…"
                  : "正在打开选择器…"
                : isEnglish
                  ? "Choose image files"
                  : "选择图片文件"}
            </button>
          </div>
        </div>
        <label className="field">
          {isEnglish ? "Local screenshot paths" : "本地截图路径"}
          <span aria-hidden="true">
            {isEnglish ? "One path per line, or comma separated" : "一行一个路径，或用逗号分隔"}
          </span>
          <textarea
            aria-label="Local screenshot paths"
            value={paths}
            onChange={(event) => setPaths(event.target.value)}
          />
        </label>
        <button className="button-secondary" type="submit" aria-label="Import screenshots">
          {isEnglish ? "Import screenshots" : "导入截图"}
        </button>
      </form>

      {sortedScreenshots.length === 0 && !loading ? (
        <div className="empty-state">
          <h3>{isEnglish ? "No screenshots imported" : "还没有导入截图"}</h3>
          <p>
            {isEnglish
              ? "Import at least one screenshot to continue. Three or more references produce stronger cross-screen patterns."
              : "至少导入 1 张后可以继续配置分析；3 张以上能更稳定地提取跨页面模式。"}
          </p>
          <p>No screenshots imported yet.</p>
        </div>
      ) : null}
      {sortedScreenshots.length > 0 && sortedScreenshots.length < 3 ? (
        <p className="alert">Recommendation: import at least 3 screenshots for stronger patterns.</p>
      ) : null}

      {sortedScreenshots.length > 0 ? (
        <ul className="shot-grid" aria-label="Screenshots">
          {sortedScreenshots.map((screenshot) => (
            <li className="shot" key={screenshot.id}>
              <div className="shot__thumb">
                <img
                  src={desktop.screenshotUrl(screenshot)}
                  alt={`Screenshot preview for ${screenshot.page_name}`}
                />
                <span className="shot__dim">
                  {screenshot.width}×{screenshot.height} · {screenshot.media_type.replace("image/", "")}
                </span>
              </div>
              <div className="shot__fields">
                <label className="shot__field">
                  {isEnglish ? "Page" : "页面"}
                  <input
                    aria-label={`Page name for ${screenshot.page_name}`}
                    value={screenshot.draftPageName}
                    onChange={(event) =>
                      updateDraft(screenshot.id, { draftPageName: event.target.value })
                    }
                  />
                </label>
                <label className="shot__field">
                  {isEnglish ? "Scene" : "场景"}
                  <input
                    aria-label={`Scene for ${screenshot.page_name}`}
                    value={screenshot.draftScene}
                    onChange={(event) =>
                      updateDraft(screenshot.id, { draftScene: event.target.value })
                    }
                  />
                </label>
                <div className="shot__actions">
                  <button
                    className="button-secondary"
                    type="button"
                    aria-label={`Save metadata for ${screenshot.page_name}`}
                    onClick={() => void saveMetadata(screenshot)}
                  >
                    {isEnglish ? "Save" : "保存标注"}
                  </button>
                  <button
                    className="button-quiet button-danger-text"
                    type="button"
                    aria-label={`Remove ${screenshot.page_name}`}
                    onClick={() => void removeScreenshot(screenshot)}
                  >
                    {isEnglish ? "Remove" : "移除"}
                  </button>
                </div>
              </div>
            </li>
          ))}
        </ul>
      ) : null}
      </div>
      <aside className="help-panel">
        <h3>{isEnglish ? "What happens next" : "接下来会发生什么"}</h3>
        <p>{isEnglish ? "1. Local preprocessing reads image dimensions, format, and metadata." : "1. 本地预处理：只读取图片尺寸、格式和摘要信息。"}</p>
        <p>{isEnglish ? "2. AI analysis sends selected screenshots only after disclosure confirmation." : "2. 发送 AI 分析：只有在披露页确认后才发送选中的截图。"}</p>
        <p>{isEnglish ? "3. The model extracts colors, layout, components, and constraints." : "3. 提取设计语言：生成颜色、布局、组件和约束规则。"}</p>
        <hr />
        <h3>{t("privacyTitle")}</h3>
        <p>{t("privacyBody")}</p>
      </aside>
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
