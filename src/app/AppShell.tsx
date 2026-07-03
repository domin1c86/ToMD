import { ReactNode, useMemo } from "react";
import { Link, useLocation } from "react-router-dom";

import { WorkflowStep, WorkflowStepper } from "../components/workflow/WorkflowStepper";
import { useI18n } from "./i18n";
import { useTheme } from "./theme";

type AppShellProps = {
  children: ReactNode;
};

export function AppShell({ children }: AppShellProps) {
  const location = useLocation();
  const { locale, setLocale, t } = useI18n();
  const { theme, setTheme } = useTheme();
  const projectId = useMemo(() => extractProjectId(location.pathname), [location.pathname]);
  const activeStep = useMemo(() => stepFromPath(location.pathname), [location.pathname]);
  const nextAction = useMemo(() => getNextAction(activeStep, projectId, t), [activeStep, projectId, t]);

  return (
    <div className="app-shell">
      <aside className="app-sidebar">
        <Link className="app-brand" to="/">
          <span className="app-brand__mark">MD</span>
          <span>
            <strong>{t("brand")}</strong>
            <small>{t("localFirst")}</small>
          </span>
        </Link>

        <Link className="app-sidebar__primary-action" to="/">
          <span aria-hidden="true">＋</span>
          {t("newProject")}
        </Link>

        <nav className="app-sidebar__nav" aria-label="Primary navigation">
          <Link className="app-sidebar__link" to="/">
            <span aria-hidden="true">▦</span>
            <span>{t("projects")}</span>
            <span className="app-sidebar__link-en">Projects</span>
          </Link>
          <Link className="app-sidebar__link" to={projectId ? `/projects/${projectId}/providers` : "/"}>
            <span aria-hidden="true">◈</span>
            <span>{t("providers")}</span>
            <span className="app-sidebar__link-en">Providers</span>
          </Link>
          <Link className="app-sidebar__link" to={projectId ? `/projects/${projectId}/exports` : "/"}>
            <span aria-hidden="true">□</span>
            <span>{t("exports")}</span>
            <span className="app-sidebar__link-en">Exports</span>
          </Link>
        </nav>

        <div className="app-sidebar__footer">
          <span>{t("settings")}</span>
          <span>{t("help")}</span>
        </div>
      </aside>

      <div className="app-workspace">
        <header className="app-topbar">
          <div>
            <p className="eyebrow">{t("localFirst")}</p>
            <h1>
              <span className="app-topbar__title-main">{t("appName")}</span>
              <span>{t("appNameEn")}</span>
            </h1>
          </div>
          <div className="app-topbar__actions">
            <Link className="button-primary" to={nextAction.to}>
              {nextAction.label}
            </Link>
            <button
              className="segmented-button"
              type="button"
              aria-label={locale === "zh-CN" ? "Switch to English" : "Switch to Chinese"}
              onClick={() => setLocale(locale === "zh-CN" ? "en-US" : "zh-CN")}
            >
              {locale === "zh-CN" ? t("switchToEnglish") : t("switchToChinese")}
            </button>
            <button
              className="segmented-button"
              type="button"
              aria-label={theme === "light" ? "Switch to dark theme" : "Switch to light theme"}
              onClick={() => setTheme(theme === "light" ? "dark" : "light")}
            >
              {theme === "light" ? t("darkTheme") : t("lightTheme")}
            </button>
          </div>
        </header>

        <WorkflowStepper activeStep={activeStep} projectId={projectId} />

        <main className="app-main">{children}</main>
      </div>
    </div>
  );
}

function getNextAction(
  activeStep: WorkflowStep,
  projectId: string | null,
  t: (key: "newProject" | "screenshotsStep" | "providerStep" | "analysisStep" | "reviewExportStep") => string,
) {
  if (!projectId || activeStep === "project") {
    return { label: t("newProject"), to: "/" };
  }
  if (activeStep === "screenshots") {
    return { label: t("providerStep"), to: `/projects/${projectId}/providers` };
  }
  if (activeStep === "provider") {
    return { label: t("analysisStep"), to: `/projects/${projectId}/analyze` };
  }
  if (activeStep === "analysis") {
    return { label: t("reviewExportStep"), to: `/projects/${projectId}/workbench` };
  }
  return { label: t("reviewExportStep"), to: `/projects/${projectId}/exports` };
}

function extractProjectId(pathname: string): string | null {
  const match = pathname.match(/^\/projects\/([^/]+)/);
  return match?.[1] ?? null;
}

function stepFromPath(pathname: string): WorkflowStep {
  if (pathname.includes("/providers")) {
    return "provider";
  }
  if (pathname.includes("/analyze")) {
    return "analysis";
  }
  if (pathname.includes("/workbench") || pathname.includes("/exports")) {
    return "review_export";
  }
  if (pathname.startsWith("/projects/")) {
    return "screenshots";
  }
  return "project";
}
