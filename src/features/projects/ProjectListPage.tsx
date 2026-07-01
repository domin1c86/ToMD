import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { desktop } from "../../lib/desktop";
import type { Project } from "../../lib/desktop";
import { NewProjectDialog } from "./NewProjectDialog";

export function ProjectListPage() {
  const navigate = useNavigate();
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showNewProject, setShowNewProject] = useState(false);
  const [deleteCandidate, setDeleteCandidate] = useState<Project | null>(null);

  const loadProjects = async () => {
    setLoading(true);
    setError(null);
    try {
      setProjects(await desktop.listProjects({ includeArchived: false }));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadProjects();
  }, []);

  const archiveProject = async (project: Project) => {
    await desktop.archiveProject({ projectId: project.id });
    await loadProjects();
  };

  const deleteProject = async () => {
    if (!deleteCandidate) {
      return;
    }
    await desktop.deleteProject({ projectId: deleteCandidate.id });
    setDeleteCandidate(null);
    await loadProjects();
  };

  return (
    <section>
      <div>
        <h2>Projects</h2>
        <button type="button" onClick={() => setShowNewProject(true)}>
          New project
        </button>
      </div>

      {showNewProject ? (
        <NewProjectDialog
          onCancel={() => setShowNewProject(false)}
          onCreated={(project) => navigate(`/projects/${project.id}`)}
        />
      ) : null}

      {loading ? <p>Loading projects…</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      {!loading && projects.length === 0 ? <p>No projects yet.</p> : null}

      {projects.length > 0 ? (
        <ul aria-label="Projects">
          {projects.map((project) => (
            <li key={project.id}>
              <Link to={`/projects/${project.id}`}>{project.name}</Link>
              <span> · {project.platform}</span>
              <button
                type="button"
                aria-label={`Archive ${project.name}`}
                onClick={() => void archiveProject(project)}
              >
                Archive
              </button>
              <button
                type="button"
                aria-label={`Delete ${project.name}`}
                onClick={() => setDeleteCandidate(project)}
              >
                Delete
              </button>
            </li>
          ))}
        </ul>
      ) : null}

      {deleteCandidate ? (
        <section aria-label="Delete project confirmation">
          <p>Delete {deleteCandidate.name}?</p>
          <button type="button" onClick={() => void deleteProject()}>
            Confirm delete
          </button>
          <button type="button" onClick={() => setDeleteCandidate(null)}>
            Cancel delete
          </button>
        </section>
      ) : null}
    </section>
  );
}
