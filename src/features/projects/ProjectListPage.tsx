import { useEffect, useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { useI18n } from "../../app/i18n";
import { desktop } from "../../lib/desktop";
import type { Project } from "../../lib/desktop";
import { NewProjectDialog } from "./NewProjectDialog";

export function ProjectListPage() {
  const navigate = useNavigate();
  const { locale, t } = useI18n();
  const isEnglish = locale === "en-US";
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
      setError(formatProjectListError(caught));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadProjects();
  }, []);

  const archiveProject = async (project: Project) => {
    setError(null);
    try {
      await desktop.archiveProject({ projectId: project.id });
    } catch (caught) {
      setError(formatProjectListError(caught));
      return;
    }
    await loadProjects();
  };

  const deleteProject = async () => {
    if (!deleteCandidate) {
      return;
    }
    setError(null);
    try {
      await desktop.deleteProject({ projectId: deleteCandidate.id });
    } catch (caught) {
      setError(formatProjectListError(caught));
      return;
    }
    setDeleteCandidate(null);
    await loadProjects();
  };

  return (
    <section className="page-grid">
      <div className="page-panel">
      <div className="page-header">
        <div>
          <h2>{isEnglish ? "Project workspace" : "项目工作台"}</h2>
          <p>
            {isEnglish
              ? "Extract a design language from reference screenshots, review it, and export DESIGN.md for another AI."
              : "从参考截图提取设计语言，审核后导出给其他 AI 使用的 DESIGN.md。"}
          </p>
        </div>
        <button
          className="button-primary"
          type="button"
          aria-label="New project"
          onClick={() => setShowNewProject(true)}
        >
          {t("newProject")}
        </button>
      </div>

      <div className="card" style={{ marginBottom: "1rem" }}>
        <h3>{isEnglish ? "Three steps" : "三步完成"}</h3>
        <p className="muted">
          {isEnglish
            ? "1. Import 3–8 references → 2. Connect a multimodal model → 3. Review rules and export DESIGN.md."
            : "1. 导入 3–8 张参考截图 → 2. 连接多模态模型 → 3. 审核规则并导出 DESIGN.md。"}
        </p>
      </div>

      {showNewProject ? (
        <NewProjectDialog
          onCancel={() => setShowNewProject(false)}
          onCreated={(project) => navigate(`/projects/${project.id}`)}
        />
      ) : null}

      {loading ? <p>{isEnglish ? "Loading projects…" : "正在加载项目…"}</p> : null}
      {error ? <p role="alert">{error}</p> : null}
      {!loading && projects.length === 0 ? (
        <div className="empty-state">
          <h3>{isEnglish ? "No projects yet" : "还没有项目"}</h3>
          <p>
            {isEnglish
              ? "Create a project to start. Projects, screenshots, rule drafts, and exports stay on this device."
              : "点击“新建项目”开始。项目、截图、规则草稿和导出历史都会保存在本机。"}
          </p>
          <p>No projects yet.</p>
        </div>
      ) : null}

      {projects.length > 0 ? (
        <ul className="project-list" aria-label="Projects">
          {projects.map((project) => (
            <li className="card pcard" key={project.id}>
              <Link className="pcard__name" to={`/projects/${project.id}`}>
                {project.name}
              </Link>
              <span className="pcard__meta">
                <span className="tag">{project.platform.toUpperCase()}</span>
              </span>
              <div className="pcard__actions">
                <button
                  className="button-secondary"
                  type="button"
                  aria-label={`Archive ${project.name}`}
                  onClick={() => void archiveProject(project)}
                >
                  {isEnglish ? "Archive" : "归档"}
                </button>
                <button
                  className="button-quiet button-danger-text"
                  type="button"
                  aria-label={`Delete ${project.name}`}
                  onClick={() => setDeleteCandidate(project)}
                >
                  {isEnglish ? "Delete" : "删除"}
                </button>
              </div>
            </li>
          ))}
        </ul>
      ) : null}

      {deleteCandidate ? (
        <section className="alert" aria-label="Delete project confirmation">
          <p>Delete {deleteCandidate.name}?</p>
          <button type="button" aria-label="Confirm delete" onClick={() => void deleteProject()}>
            {isEnglish ? "Confirm delete" : "确认删除"}
          </button>
          <button
            type="button"
            aria-label="Cancel delete"
            onClick={() => setDeleteCandidate(null)}
          >
            {isEnglish ? "Cancel delete" : "取消删除"}
          </button>
        </section>
      ) : null}
      </div>
      <aside className="help-panel">
        <h3>{isEnglish ? "What does this tool do?" : "这个工具会做什么？"}</h3>
        <p>
          {isEnglish
            ? "It does not copy the original product directly. It turns visible design evidence into reviewable rules such as color, layout, components, spacing, and interaction constraints."
            : "它不会直接复制原产品，而是把截图中的可见设计证据提取成可审核的规范，例如颜色、布局、组件、间距和交互约束。"}
        </p>
        <hr />
        <h3>{t("privacyTitle")}</h3>
        <p>{t("privacyBody")}</p>
      </aside>
    </section>
  );
}

function formatProjectListError(caught: unknown): string {
  const message = caught instanceof Error ? caught.message : String(caught);
  if (message.includes("invoke") || message.includes("__TAURI__")) {
    return "桌面后端暂不可用，请在桌面应用中运行。";
  }
  return message;
}
