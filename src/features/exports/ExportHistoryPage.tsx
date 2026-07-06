import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";

import { useI18n } from "../../app/i18n";
import type { DesignSpec, Rule } from "../../generated/bindings";
import { desktop, type ExportVersion } from "../../lib/desktop";
import { getRuleGroups } from "../workbench/ruleGroups";

export function ExportHistoryPage() {
  const { projectId = "" } = useParams();
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const [spec, setSpec] = useState<DesignSpec | null>(null);
  const [exports, setExports] = useState<ExportVersion[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [revealedPath, setRevealedPath] = useState<string | null>(null);
  const [copiedExportId, setCopiedExportId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [exporting, setExporting] = useState(false);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const [loadedSpec, loadedExports] = await Promise.all([
        desktop.getDesignSpec({ projectId }),
        desktop.listExports({ projectId }),
      ]);
      setSpec(loadedSpec);
      setExports(loadedExports);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, [projectId]);

  const exportCurrentVersion = async () => {
    setExporting(true);
    setError(null);
    try {
      await desktop.exportDesignMarkdown({ projectId });
      setExports(await desktop.listExports({ projectId }));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setExporting(false);
    }
  };

  const copyContent = async (exportVersion: ExportVersion) => {
    setError(null);
    setCopiedExportId(null);
    try {
      const content = await desktop.readExportMarkdown({
        projectId,
        exportId: exportVersion.id,
      });
      await navigator.clipboard.writeText(content);
      setCopiedExportId(exportVersion.id);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };

  const revealExport = async (exportVersion: ExportVersion) => {
    setError(null);
    setRevealedPath(null);
    try {
      await desktop.revealExport({ projectId, exportId: exportVersion.id });
      setRevealedPath(exportVersion.relative_path);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  };

  return (
    <section className="page-grid">
      <div className="page-panel">
      <div className="page-header">
        <div>
          <h2>{isEnglish ? "Export DESIGN.md" : "导出 DESIGN.md"}</h2>
          <p>
            {isEnglish
              ? "The exported Markdown includes only accepted or edited rules and can be used by another AI as a design specification."
              : "导出的 Markdown 只包含已接受或已编辑的规则，可直接交给其他 AI 作为设计规范。"}
          </p>
        </div>
        <button
          className="button-primary"
          type="button"
          aria-label="Export current version"
          disabled={exporting}
          onClick={() => void exportCurrentVersion()}
        >
          {isEnglish ? "Export current version" : "导出当前版本"}
        </button>
      </div>
      {loading ? <p>{isEnglish ? "Loading exports…" : "正在加载导出记录…"}</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      {revealedPath ? <p>Reveal in folder: {revealedPath}</p> : null}
      {copiedExportId ? <p>Copied DESIGN.md content for {copiedExportId}</p> : null}

      <div className="exports-grid">
        <ul className="export-list" aria-label="Export history">
          {exports.map((exportVersion) => (
            <li className="card exp-item" key={exportVersion.id}>
              <div className="exp-item__info">
                <p className="exp-item__when mono">{exportVersion.created_at}</p>
                <p className="exp-item__path">Source spec version: {exportVersion.spec_version_id}</p>
                <p className="exp-item__path">{exportVersion.relative_path}</p>
              </div>
              <div className="exp-item__actions">
                <button
                  className="button-secondary"
                  type="button"
                  aria-label={`Copy content for ${exportVersion.id}`}
                  onClick={() => void copyContent(exportVersion)}
                >
                  {isEnglish ? "Copy content" : "复制内容"}
                </button>
                <button
                  className="button-quiet"
                  type="button"
                  aria-label={`Reveal in folder for ${exportVersion.id}`}
                  onClick={() => void revealExport(exportVersion)}
                >
                  {isEnglish ? "Reveal in folder" : "在文件夹中显示"}
                </button>
              </div>
            </li>
          ))}
        </ul>

        <section className="paperwrap" aria-label="Export preview">
          <div className="paper-progress">
            <span>{isEnglish ? "Preview" : "预览"}</span>
          </div>
          <div className="paper">
            <pre data-testid="export-preview">{spec ? compilePreview(spec) : ""}</pre>
          </div>
        </section>
      </div>
      </div>
      <aside className="help-panel">
        <h3>{isEnglish ? "How to use the export" : "如何使用导出文件"}</h3>
        <p>
          {isEnglish
            ? "Place DESIGN.md in the new product repository or prompt so another AI follows its colors, layout, components, and constraints."
            : "把 DESIGN.md 放到新产品仓库或提示词中，让其他 AI 按其中的颜色、布局、组件和约束实现新界面。"}
        </p>
        <hr />
        <h3>{isEnglish ? "Export rules" : "导出规则"}</h3>
        <p>
          {isEnglish
            ? "Rejected rules are excluded. Each export creates a new history item for traceability."
            : "Rejected 规则不会导出。每次导出都会生成新的历史记录，便于回溯。"}
        </p>
      </aside>
    </section>
  );
}

function compilePreview(spec: DesignSpec): string {
  const sections = getRuleGroups(spec)
    .map((group) => {
      const rules = group.rules.filter(isExportable);
      if (rules.length === 0) {
        return "";
      }

      return [`## ${group.key}`, ...rules.map((rule) => `- ${rule.statement}`)].join("\n");
    })
    .filter(Boolean);

  return ["# DESIGN.md Preview", ...sections].join("\n\n");
}

function isExportable(rule: Rule): boolean {
  return rule.status === "accepted" || rule.status === "edited";
}
