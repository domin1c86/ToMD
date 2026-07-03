import { FormEvent, useState } from "react";

import { useI18n } from "../../app/i18n";
import type { Platform } from "../../generated/bindings";
import { desktop } from "../../lib/desktop";
import type { Project } from "../../lib/desktop";

type NewProjectDialogProps = {
  onCancel: () => void;
  onCreated: (project: Project) => void;
};

export function NewProjectDialog({ onCancel, onCreated }: NewProjectDialogProps) {
  const { locale } = useI18n();
  const isEnglish = locale === "en-US";
  const [name, setName] = useState("");
  const [platform, setPlatform] = useState<Platform>("web");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!name.trim()) {
      setError("Project name is required.");
      return;
    }

    setSubmitting(true);
    setError(null);
    try {
      const project = await desktop.createProject({
        name: name.trim(),
        platform,
      });
      onCreated(project);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <section className="card" aria-label="New project">
      <h2>{isEnglish ? "New project" : "新建项目"}</h2>
      <form className="form-grid" onSubmit={handleSubmit}>
        <label className="field">
          {isEnglish ? "Project name" : "项目名称"}
          <span aria-hidden="true">Project name</span>
          <input
            aria-label="Project name"
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <label className="field">
          {isEnglish ? "Target platform" : "目标平台"}
          <span aria-hidden="true">Target platform</span>
          <select
            aria-label="Target platform"
            value={platform}
            onChange={(event) => setPlatform(event.target.value as Platform)}
          >
            <option value="web">Web</option>
            <option value="mobile">Mobile</option>
            <option value="desktop">Desktop</option>
            <option value="cross_platform">Cross-platform</option>
          </select>
        </label>
        {error ? <p role="alert">{error}</p> : null}
        <button className="button-primary" type="submit" aria-label="Create project" disabled={submitting}>
          {isEnglish ? "Create project" : "创建项目"}
        </button>
        <button className="button-secondary" type="button" onClick={onCancel}>
          {isEnglish ? "Cancel" : "取消"}
        </button>
      </form>
    </section>
  );
}
