import { BrowserRouter, Link, Route, Routes } from "react-router-dom";

import { ProjectListPage } from "../features/projects/ProjectListPage";
import { ScreenshotManagerPage } from "../features/screenshots/ScreenshotManagerPage";

export function App() {
  return (
    <BrowserRouter>
      <header>
        <h1>Design Language Extractor</h1>
        <nav aria-label="Primary navigation">
          <Link to="/">Projects</Link>
        </nav>
      </header>
      <main>
        <Routes>
          <Route path="/" element={<ProjectListPage />} />
          <Route path="/projects/:projectId" element={<ScreenshotManagerPage />} />
          <Route path="/projects/:projectId/providers" element={<p>Provider setup</p>} />
          <Route path="/projects/:projectId/analyze" element={<p>Analysis setup</p>} />
          <Route path="/projects/:projectId/workbench" element={<p>Rule workbench</p>} />
          <Route path="/projects/:projectId/exports" element={<p>Exports</p>} />
        </Routes>
      </main>
    </BrowserRouter>
  );
}
