import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";

import type { DesignSpec, Rule } from "../../generated/bindings";
import { desktop, type ExportVersion } from "../../lib/desktop";
import { getRuleGroups } from "../workbench/ruleGroups";

export function ExportHistoryPage() {
  const { projectId = "" } = useParams();
  const [spec, setSpec] = useState<DesignSpec | null>(null);
  const [exports, setExports] = useState<ExportVersion[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [revealedPath, setRevealedPath] = useState<string | null>(null);
  const [copiedPath, setCopiedPath] = useState<string | null>(null);
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

  const copyPath = async (relativePath: string) => {
    await navigator.clipboard.writeText(relativePath);
    setCopiedPath(relativePath);
  };

  return (
    <section>
      <h2>Exports</h2>
      {loading ? <p>Loading exports…</p> : null}
      {error ? <p role="alert">{error}</p> : null}

      <button type="button" disabled={exporting} onClick={() => void exportCurrentVersion()}>
        Export current version
      </button>

      <section aria-label="Export preview">
        <h3>Preview</h3>
        <pre data-testid="export-preview">{spec ? compilePreview(spec) : ""}</pre>
      </section>

      {revealedPath ? <p>Reveal in folder: {revealedPath}</p> : null}
      {copiedPath ? <p>Copied path: {copiedPath}</p> : null}

      <ul aria-label="Export history">
        {exports.map((exportVersion) => (
          <li key={exportVersion.id}>
            <p>{exportVersion.created_at}</p>
            <p>Source spec version: {exportVersion.spec_version_id}</p>
            <p>{exportVersion.relative_path}</p>
            <button type="button" onClick={() => void copyPath(exportVersion.relative_path)}>
              Copy path for {exportVersion.id}
            </button>
            <button type="button" onClick={() => setRevealedPath(exportVersion.relative_path)}>
              Reveal in folder for {exportVersion.id}
            </button>
          </li>
        ))}
      </ul>
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
