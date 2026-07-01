import { FormEvent, useState } from "react";

import type { Platform } from "../../generated/bindings";
import { desktop } from "../../lib/desktop";
import type { Project } from "../../lib/desktop";

type NewProjectDialogProps = {
  onCancel: () => void;
  onCreated: (project: Project) => void;
};

export function NewProjectDialog({ onCancel, onCreated }: NewProjectDialogProps) {
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
    <section aria-label="New project">
      <h2>New project</h2>
      <form onSubmit={handleSubmit}>
        <label>
          Project name
          <input value={name} onChange={(event) => setName(event.target.value)} />
        </label>
        <label>
          Target platform
          <select
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
        <button type="submit" disabled={submitting}>
          Create project
        </button>
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
      </form>
    </section>
  );
}
