import { BrowserRouter, Route, Routes } from "react-router-dom";

import { AnalysisStartPage } from "../features/analysis/AnalysisStartPage";
import { ExportHistoryPage } from "../features/exports/ExportHistoryPage";
import { ProjectListPage } from "../features/projects/ProjectListPage";
import { ScreenshotManagerPage } from "../features/screenshots/ScreenshotManagerPage";
import { WorkbenchPage } from "../features/workbench/WorkbenchPage";
import { AppShell } from "./AppShell";
import { I18nProvider } from "./i18n";
import { ThemeProvider } from "./theme";

export function App() {
  return (
    <BrowserRouter>
      <ThemeProvider>
        <I18nProvider>
          <AppShell>
            <h1 className="sr-only">Design Language Extractor</h1>
        <Routes>
          <Route path="/" element={<ProjectListPage />} />
          <Route path="/projects/:projectId" element={<ScreenshotManagerPage />} />
          <Route path="/projects/:projectId/analyze" element={<AnalysisStartPage />} />
          <Route path="/projects/:projectId/workbench" element={<WorkbenchPage />} />
          <Route path="/projects/:projectId/exports" element={<ExportHistoryPage />} />
        </Routes>
          </AppShell>
        </I18nProvider>
      </ThemeProvider>
    </BrowserRouter>
  );
}
