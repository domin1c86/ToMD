import { ReactNode, useMemo, useState, useSyncExternalStore } from "react";
import { Link, useLocation } from "react-router-dom";

import { SettingsModal } from "../features/settings/SettingsModal";
import {
  getNetworkRequestCount,
  subscribeNetworkRequests,
} from "../lib/networkCounter";
import { useI18n } from "./i18n";
import { useTheme } from "./theme";

type AppShellProps = {
  children: ReactNode;
};

type StepKey = "projects" | "screenshots" | "provider" | "review" | "exports";

const STEP_ORDER: StepKey[] = ["projects", "screenshots", "provider", "review", "exports"];

export function AppShell({ children }: AppShellProps) {
  const location = useLocation();
  const { locale, setLocale, t } = useI18n();
  const { theme, setTheme } = useTheme();
  const isEnglish = locale === "en-US";
  const projectId = useMemo(() => extractProjectId(location.pathname), [location.pathname]);
  const activeStep = useMemo(() => stepFromPath(location.pathname), [location.pathname]);
  const requestCount = useSyncExternalStore(subscribeNetworkRequests, getNetworkRequestCount);
  const [showSettings, setShowSettings] = useState(false);

  const steps: { key: StepKey; label: string; to: string; ariaLabel?: string }[] = [
    { key: "projects", label: t("stepProjects"), to: "/", ariaLabel: "Projects" },
    { key: "screenshots", label: t("stepScreenshots"), to: projectId ? `/projects/${projectId}` : "/" },
    { key: "provider", label: t("stepProvider"), to: projectId ? `/projects/${projectId}/analyze` : "/" },
    { key: "review", label: t("stepReview"), to: projectId ? `/projects/${projectId}/workbench` : "/" },
    { key: "exports", label: t("stepExports"), to: projectId ? `/projects/${projectId}/exports` : "/" },
  ];
  const activeIndex = STEP_ORDER.indexOf(activeStep);

  return (
    <div className="frame">
      <header className="headerbar">
        <span className="app-brand__mark" aria-hidden="true">MD</span>
        <span className="headerbar__title">{t("appName")}</span>
        <span className="headerbar__tagline">{t("localFirst")}</span>
        <span className="headerbar__spacer" />
        <button
          className="headerbar__chip"
          type="button"
          aria-label={locale === "zh-CN" ? "Switch to English" : "Switch to Chinese"}
          onClick={() => setLocale(locale === "zh-CN" ? "en-US" : "zh-CN")}
        >
          {locale === "zh-CN" ? t("switchToEnglish") : t("switchToChinese")}
        </button>
        <button
          className="headerbar__chip"
          type="button"
          aria-label={theme === "light" ? "Switch to dark theme" : "Switch to light theme"}
          onClick={() => setTheme(theme === "light" ? "dark" : "light")}
        >
          {theme === "light" ? t("darkTheme") : t("lightTheme")}
        </button>
        <button
          className="headerbar__chip"
          type="button"
          aria-label="Open settings"
          onClick={() => setShowSettings(true)}
        >
          {t("settings")}
        </button>
      </header>

      {showSettings ? <SettingsModal onClose={() => setShowSettings(false)} /> : null}

      <div className="frame-shell">
        <nav className="sidebar" aria-label="Primary navigation">
          <p className="sidebar__label">{t("flowLabel")}</p>
          <div className="sidebar-steps">
            {steps.map((step, index) => {
              const isCurrent = step.key === activeStep;
              const isDone = projectId !== null && index < activeIndex;
              return (
                <Link
                  key={step.key}
                  className={`sidebar-step${isDone ? " sidebar-step--done" : ""}`}
                  aria-current={isCurrent ? "true" : undefined}
                  aria-label={step.ariaLabel}
                  to={step.to}
                >
                  <span className="sidebar-step__n" aria-hidden="true">
                    {isDone ? "✓" : index + 1}
                  </span>
                  <span>{step.label}</span>
                </Link>
              );
            })}
          </div>

          <div className="sidebar__fill" />

          <div className="sidebar-privacy">
            <div>
              <span className="sidebar-privacy__dot" aria-hidden="true" />
              {t("privacyLocal")}
            </div>
            <div className="sidebar-privacy__sub">
              {isEnglish
                ? `${requestCount} network requests this session`
                : `本次会话 ${requestCount} 次网络请求`}
            </div>
          </div>
        </nav>

        <div className="app-workspace">
          <main className="app-main">{children}</main>
        </div>
      </div>

      <footer className="statusbar">
        <span className="statusbar__dot" aria-hidden="true" />
        <span>{isEnglish ? "Local data · SQLite" : "本地数据 · SQLite"}</span>
        <span>{isEnglish ? "Credentials: OS keyring" : "凭据：系统凭据库"}</span>
        <span className="statusbar__spacer" />
        <span>v0.1.0</span>
      </footer>
    </div>
  );
}

function extractProjectId(pathname: string): string | null {
  const match = pathname.match(/^\/projects\/([^/]+)/);
  return match?.[1] ?? null;
}

function stepFromPath(pathname: string): StepKey {
  if (pathname.includes("/analyze")) {
    return "provider";
  }
  if (pathname.includes("/workbench")) {
    return "review";
  }
  if (pathname.includes("/exports")) {
    return "exports";
  }
  if (pathname.startsWith("/projects/")) {
    return "screenshots";
  }
  return "projects";
}
