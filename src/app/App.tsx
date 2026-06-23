import { BrowserRouter, Link, Route, Routes } from "react-router-dom";

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
          <Route path="/" element={<p>No projects yet.</p>} />
        </Routes>
      </main>
    </BrowserRouter>
  );
}
