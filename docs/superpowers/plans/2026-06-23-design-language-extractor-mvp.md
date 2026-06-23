# Design Language Extractor MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a local-first Tauri desktop app that turns multiple reference screenshots into an editable `DesignSpec` and exports a provider-neutral `DESIGN.md`.

**Architecture:** A React/Vite frontend calls narrow Tauri commands. Rust workspace crates own the versioned design model, Markdown compiler, SQLite/file persistence, credential access, multimodal provider adapters, and analysis orchestration. The UI never handles plaintext credentials and the model response is never rendered before schema validation.

**Tech Stack:** Tauri 2, Rust, React, TypeScript, Vite, React Router, Zod, Vitest, React Testing Library, SQLite/rusqlite, reqwest, keyring, serde, schemars, ts-rs.

---

## File map

Create these responsibility boundaries:

- `src/`: React shell and feature folders; no direct filesystem, credential, database, or HTTP access.
- `src/lib/desktop.ts`: the only frontend wrapper around Tauri `invoke`.
- `src/generated/bindings.ts`: generated Rust-to-TypeScript DTOs; never edit manually.
- `src-tauri/crates/design-core/`: `DesignSpec`, rule/evidence types, merge semantics, validation, and Markdown compilation.
- `src-tauri/crates/design-storage/`: SQLite repositories, migrations, project directories, screenshot import, and export files.
- `src-tauri/crates/design-providers/`: provider-neutral request/response contract and OpenAI-compatible adapter.
- `src-tauri/crates/design-analysis/`: prompt construction, orchestration, response extraction, one-pass repair, and provenance.
- `src-tauri/src/commands/`: thin Tauri command adapters grouped by projects, screenshots, providers, analysis, rules, and exports.
- `src-tauri/src/state.rs`: initialized application services only; no business rules.

## Milestone 1: Runnable local application

### Task 1: Scaffold the Tauri workspace and quality gates

**Files:**
- Create: `package.json`
- Create: `vite.config.ts`
- Create: `vitest.config.ts`
- Create: `src/main.tsx`
- Create: `src/app/App.tsx`
- Create: `src/app/App.test.tsx`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/main.rs`
- Create: `.github/workflows/ci.yml`
- Create: `.gitignore`

- [ ] **Step 1: Initialize Git, React/TypeScript, and Tauri 2**

Run from the repository root:

```powershell
git init
npm init -y
npm install react react-dom react-router-dom zod @tauri-apps/api
npm install -D typescript vite @vitejs/plugin-react eslint @eslint/js typescript-eslint @types/react @types/react-dom
npm install -D @tauri-apps/cli@latest
npx tauri init
```

Use these Tauri answers:

```text
App name: Design Language Extractor
Window title: Design Language Extractor
Web assets: ../dist
Dev server: http://localhost:5173
Frontend dev command: npm run dev
Frontend build command: npm run build
Bundle identifier: com.tomd.designlanguageextractor
```

Create the listed Vite and React files using the standard React TypeScript entry point, with `vite.config.ts` loading `@vitejs/plugin-react`. Expected: `npm run tauri dev` opens the desktop window.

- [ ] **Step 2: Add frontend tests and scripts**

Run:

```powershell
npm install -D vitest jsdom @testing-library/react @testing-library/jest-dom @testing-library/user-event
```

Set scripts in `package.json`:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "test": "vitest run",
    "test:watch": "vitest",
    "lint": "eslint .",
    "tauri": "tauri"
  }
}
```

- [ ] **Step 3: Write the failing application-shell test**

`src/app/App.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { App } from "./App";

test("renders the local workspace entry point", () => {
  render(<App />);
  expect(
    screen.getByRole("heading", { name: "Design Language Extractor" }),
  ).toBeInTheDocument();
  expect(screen.getByRole("link", { name: "Projects" })).toBeInTheDocument();
});
```

Run: `npm test -- src/app/App.test.tsx`

Expected: FAIL because `App` does not yet expose the required shell.

- [ ] **Step 4: Implement the minimal shell**

`src/app/App.tsx`:

```tsx
import { BrowserRouter, Link, Route, Routes } from "react-router-dom";

export function App() {
  return (
    <BrowserRouter>
      <header>
        <h1>Design Language Extractor</h1>
        <nav><Link to="/">Projects</Link></nav>
      </header>
      <main>
        <Routes>
          <Route path="/" element={<p>No projects yet.</p>} />
        </Routes>
      </main>
    </BrowserRouter>
  );
}
```

Run:

```powershell
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all commands PASS.

- [ ] **Step 5: Add CI**

`.github/workflows/ci.yml` must run `npm ci`, `npm test`, `npm run build`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --workspace` on Windows.

- [ ] **Step 6: Commit**

```powershell
git add .
git commit -m "chore: scaffold local Tauri application"
```

## Milestone 2: Stable core contract

### Task 2: Implement versioned `DesignSpec` and generated TypeScript bindings

**Files:**
- Create: `src-tauri/crates/design-core/Cargo.toml`
- Create: `src-tauri/crates/design-core/src/lib.rs`
- Create: `src-tauri/crates/design-core/src/model.rs`
- Create: `src-tauri/crates/design-core/src/validation.rs`
- Create: `src-tauri/crates/design-core/tests/schema.rs`
- Generate: `src/generated/bindings.ts`

- [ ] **Step 1: Add the core crate**

Add workspace member `crates/design-core` and dependencies:

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
schemars = { version = "1", features = ["chrono04", "uuid1"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
ts-rs = { version = "11", features = ["chrono-impl", "uuid-impl"] }
uuid = { version = "1", features = ["serde", "v4"] }
```

- [ ] **Step 2: Write failing schema invariants**

`src-tauri/crates/design-core/tests/schema.rs`:

```rust
use design_core::{DesignSpec, RuleKind, RuleStatus};

#[test]
fn new_spec_uses_schema_version_one() {
    let spec = DesignSpec::empty("project-1");
    assert_eq!(spec.metadata.schema_version, "1.0");
}

#[test]
fn confidence_must_be_between_zero_and_one() {
    let mut spec = DesignSpec::empty("project-1");
    spec.rules.push(design_core::Rule::new(
        "color",
        "Use the accent color only for interactive emphasis.",
        RuleKind::Pattern,
        1.2,
    ));
    assert!(spec.validate().is_err());
}

#[test]
fn rejected_rules_are_valid_but_not_exportable() {
    let mut rule = design_core::Rule::new(
        "layout",
        "Use a compact information density.",
        RuleKind::Recommendation,
        0.8,
    );
    rule.status = RuleStatus::Rejected;
    assert!(!rule.is_exportable());
}
```

Run: `cargo test -p design-core`

Expected: FAIL because the crate and types do not exist.

- [ ] **Step 3: Implement the public model**

`model.rs` must define:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct DesignSpec {
    pub metadata: Metadata,
    pub intent: Vec<Rule>,
    pub tokens: Vec<Rule>,
    pub layout: Vec<Rule>,
    pub components: Vec<Rule>,
    pub assets: Vec<Rule>,
    pub motion: Vec<Rule>,
    pub constraints: Vec<Rule>,
    pub evidence: Vec<Evidence>,
    pub uncertainties: Vec<Uncertainty>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
pub struct Rule {
    pub id: Uuid,
    pub category: String,
    pub statement: String,
    pub kind: RuleKind,
    pub scope: RuleScope,
    pub value: Option<serde_json::Value>,
    pub evidence_ids: Vec<Uuid>,
    pub confidence: f32,
    pub status: RuleStatus,
    pub source: RuleSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind { Observation, Pattern, Recommendation }

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum RuleStatus { Pending, Accepted, Edited, Rejected }
```

Also define `Metadata`, `RuleScope`, `RuleSource`, `Evidence`, `EvidenceRegion`, and `Uncertainty` exactly as specified in the design document. `validate()` must reject blank statements, confidence outside `0.0..=1.0`, duplicate IDs, missing evidence references, and schema versions whose major component is not `1`.

`Metadata` must include `schema_version`, `project_id`, `provider_id`, `model`, `source_screenshot_ids`, `excluded_terms`, and `created_at`. `Platform` is the shared enum `Web | Desktop | Mobile | CrossPlatform`.

- [ ] **Step 4: Export TypeScript bindings**

Add a test named `export_typescript_bindings` that resolves `env!("CARGO_MANIFEST_DIR")`, joins `../../../src/generated/bindings.ts`, and writes all exported DTOs using `ts-rs`. The generated file must begin:

```ts
// Generated by design-core. Do not edit.
export type RuleKind = "observation" | "pattern" | "recommendation";
export type RuleStatus = "pending" | "accepted" | "edited" | "rejected";
```

Run:

```powershell
cargo test -p design-core
npx tsc --noEmit
```

Expected: PASS and `src/generated/bindings.ts` exists.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri src/generated
git commit -m "feat: define versioned design specification"
```

### Task 3: Implement deterministic Markdown compilation

**Files:**
- Create: `src-tauri/crates/design-core/src/markdown.rs`
- Create: `src-tauri/crates/design-core/tests/markdown.rs`
- Create: `src-tauri/crates/design-core/tests/fixtures/accepted-spec.json`
- Create: `src-tauri/crates/design-core/tests/fixtures/accepted-design.md`

- [ ] **Step 1: Write the golden-file test**

```rust
#[test]
fn compiles_only_confirmed_rules_in_fixed_section_order() {
    let spec: DesignSpec =
        serde_json::from_str(include_str!("fixtures/accepted-spec.json")).unwrap();
    let output = compile_markdown(&spec).unwrap();
    assert_eq!(output, include_str!("fixtures/accepted-design.md"));
    assert!(!output.contains("OriginalBrand"));
    assert!(!output.contains("rejected rule"));
}
```

Run: `cargo test -p design-core --test markdown`

Expected: FAIL because `compile_markdown` does not exist.

- [ ] **Step 2: Implement the compiler**

Expose:

```rust
pub fn compile_markdown(spec: &DesignSpec) -> Result<String, ValidationError>;
```

It must:

1. Validate the spec.
2. Emit exactly the ten fixed sections from the design document.
3. Include only `Accepted` and `Edited` rules.
4. Sort rules by category, then descending confidence, then stable ID.
5. Render semantic token values when present.
6. End with a checklist generated from accepted constraints.
7. Reject statements containing a value found in `metadata.excluded_terms`.

Run: `cargo test -p design-core`

Expected: PASS with byte-for-byte golden output.

- [ ] **Step 3: Commit**

```powershell
git add src-tauri/crates/design-core
git commit -m "feat: compile design specs into deterministic markdown"
```

## Milestone 3: Local persistence and privacy

### Task 4: Implement project storage, migrations, and project directories

**Files:**
- Create: `src-tauri/crates/design-storage/Cargo.toml`
- Create: `src-tauri/crates/design-storage/src/lib.rs`
- Create: `src-tauri/crates/design-storage/src/migrations.rs`
- Create: `src-tauri/crates/design-storage/src/projects.rs`
- Create: `src-tauri/crates/design-storage/tests/projects.rs`

- [ ] **Step 1: Write repository tests against a temporary directory**

```rust
#[tokio::test]
async fn create_list_rename_archive_and_delete_project() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::open(temp.path()).await.unwrap();
    let project = storage.projects().create("Finance app", Platform::Mobile).await.unwrap();
    assert!(temp.path().join("projects").join(project.id.to_string()).exists());

    storage.projects().rename(project.id, "Money app").await.unwrap();
    storage.projects().archive(project.id).await.unwrap();
    assert_eq!(storage.projects().list(false).await.unwrap().len(), 0);

    storage.projects().delete(project.id).await.unwrap();
    assert!(!temp.path().join("projects").join(project.id.to_string()).exists());
}
```

Run: `cargo test -p design-storage`

Expected: FAIL because storage is not implemented.

- [ ] **Step 2: Create the migration**

Migration `0001_initial` must create:

```sql
CREATE TABLE projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  platform TEXT NOT NULL,
  archived_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE screenshots (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  relative_path TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  media_type TEXT NOT NULL,
  width INTEGER NOT NULL,
  height INTEGER NOT NULL,
  page_name TEXT NOT NULL,
  scene TEXT NOT NULL,
  sort_order INTEGER NOT NULL,
  created_at TEXT NOT NULL
);
CREATE TABLE design_spec_versions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  spec_json TEXT NOT NULL,
  provider_id TEXT NOT NULL,
  model TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE TABLE design_spec_drafts (
  project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
  base_version_id TEXT NOT NULL REFERENCES design_spec_versions(id),
  spec_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE provider_configs (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  base_url TEXT NOT NULL,
  model TEXT NOT NULL,
  credential_ref TEXT NOT NULL,
  capabilities_json TEXT,
  last_tested_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE export_versions (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  spec_version_id TEXT NOT NULL REFERENCES design_spec_versions(id),
  relative_path TEXT NOT NULL,
  created_at TEXT NOT NULL
);
```

Enable SQLite foreign keys and WAL mode on every connection.

`design_spec_versions` are immutable analysis/export snapshots. `design_spec_drafts` contains the one mutable working copy for each project. A successful analysis inserts an immutable version and replaces the project draft in one transaction. Rule edits modify only the draft. Export first snapshots the current draft into `design_spec_versions`, then writes an `export_versions` row referencing that snapshot.

- [ ] **Step 3: Implement the repository contract**

Expose async methods:

```rust
pub trait ProjectRepository {
    async fn create(&self, name: &str, platform: Platform) -> Result<Project, StorageError>;
    async fn list(&self, include_archived: bool) -> Result<Vec<Project>, StorageError>;
    async fn get(&self, id: Uuid) -> Result<Project, StorageError>;
    async fn rename(&self, id: Uuid, name: &str) -> Result<Project, StorageError>;
    async fn archive(&self, id: Uuid) -> Result<(), StorageError>;
    async fn delete(&self, id: Uuid) -> Result<(), StorageError>;
}
```

Use a transaction for database deletion and remove the project directory only after commit. If filesystem deletion fails, return `StorageError::CleanupRequired` with the remaining path.

Run: `cargo test -p design-storage`

Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/crates/design-storage
git commit -m "feat: persist local projects in SQLite"
```

### Task 5: Implement safe screenshot import and invalidation

**Files:**
- Create: `src-tauri/crates/design-storage/src/screenshots.rs`
- Create: `src-tauri/crates/design-storage/tests/screenshots.rs`
- Create: `src-tauri/crates/design-storage/tests/fixtures/valid.png`
- Create: `src-tauri/crates/design-storage/tests/fixtures/not-an-image.txt`

- [ ] **Step 1: Write import tests**

Tests must prove:

```rust
#[tokio::test]
async fn imports_a_copy_with_detected_dimensions_and_hash() { /* assert PNG metadata */ }

#[tokio::test]
async fn rejects_unsupported_or_corrupt_files_without_copying_them() { /* assert no DB row/file */ }

#[tokio::test]
async fn removing_a_screenshot_marks_dependent_rules_for_review() { /* assert stale evidence */ }
```

Accepted input types: PNG, JPEG, and WebP. Maximum decoded dimension: 16,384 pixels per side. Maximum source size: 25 MiB.

- [ ] **Step 2: Implement atomic import**

Expose:

```rust
pub async fn import_screenshot(
    &self,
    project_id: Uuid,
    source: &Path,
    page_name: &str,
    scene: &str,
) -> Result<Screenshot, StorageError>;
```

Read and validate before writing, calculate SHA-256, copy to a temporary file under the project, insert the row in a transaction, then atomically rename to `{screenshot_id}.{ext}`. Deduplicate identical hashes within a project by returning `StorageError::DuplicateScreenshot(existing_id)`.

Run: `cargo test -p design-storage --test screenshots`

Expected: PASS.

- [ ] **Step 3: Commit**

```powershell
git add src-tauri/crates/design-storage
git commit -m "feat: import and track local screenshots safely"
```

### Task 6: Store provider configuration without storing plaintext API keys

**Files:**
- Create: `src-tauri/crates/design-providers/Cargo.toml`
- Create: `src-tauri/crates/design-providers/src/config.rs`
- Create: `src-tauri/crates/design-providers/src/credentials.rs`
- Create: `src-tauri/crates/design-providers/tests/credentials.rs`

- [ ] **Step 1: Define configuration DTOs**

```rust
pub struct ProviderConfig {
    pub id: Uuid,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: Url,
    pub model: String,
    pub credential_ref: String,
}

pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Gemini,
    OpenAiCompatible,
}
```

Persist provider metadata in `provider_configs` and only `credential_ref`, never the secret. Use service name `com.tomd.designlanguageextractor` and username `provider:{provider_id}` in the operating-system credential store.

- [ ] **Step 2: Write tests with an injected credential store**

```rust
#[test]
fn provider_serialization_never_contains_the_secret() {
    let saved = save_provider_with_store(&MemoryCredentialStore::default(), config(), "sk-secret").unwrap();
    let json = serde_json::to_string(&saved).unwrap();
    assert!(!json.contains("sk-secret"));
    assert!(json.contains("credential_ref"));
}
```

The production `KeyringCredentialStore` implements the same trait; tests use memory only.

- [ ] **Step 3: Implement create, replace, read, and delete secret operations**

Never expose a command that returns a stored API key to the frontend. Provider DTOs expose only `has_credential: bool`.

Run: `cargo test -p design-providers --test credentials`

Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/crates/design-providers
git commit -m "feat: secure local provider credentials"
```

## Milestone 4: Multimodal analysis pipeline

### Task 7: Implement provider-neutral multimodal adapters

**Files:**
- Create: `src-tauri/crates/design-providers/src/lib.rs`
- Create: `src-tauri/crates/design-providers/src/client.rs`
- Create: `src-tauri/crates/design-providers/src/openai.rs`
- Create: `src-tauri/crates/design-providers/src/anthropic.rs`
- Create: `src-tauri/crates/design-providers/src/gemini.rs`
- Create: `src-tauri/crates/design-providers/src/openai_compatible.rs`
- Create: `src-tauri/crates/design-providers/src/error.rs`
- Create: `src-tauri/crates/design-providers/tests/openai.rs`
- Create: `src-tauri/crates/design-providers/tests/anthropic.rs`
- Create: `src-tauri/crates/design-providers/tests/gemini.rs`
- Create: `src-tauri/crates/design-providers/tests/openai_compatible.rs`

- [ ] **Step 1: Write HTTP contract tests with a mock server**

The OpenAI-compatible adapter uses the chat-completions-compatible shape and verifies one request contains:

```json
{
  "model": "vision-model",
  "messages": [
    {
      "role": "user",
      "content": [
        { "type": "text", "text": "analysis prompt" },
        { "type": "image_url", "image_url": { "url": "data:image/png;base64,..." } }
      ]
    }
  ],
  "response_format": {
    "type": "json_schema",
    "json_schema": { "name": "design_spec", "strict": true }
  }
}
```

Also test normalization of 401 to `ProviderError::Authentication`, 429 to `RateLimited`, timeout to `Timeout`, and unsupported structured output to `CapabilityMismatch`.

Add provider-specific contract tests:

- OpenAI preset: Responses API with `input_text`, one `input_image` data URL per screenshot, and strict JSON Schema output.
- Anthropic preset: Messages API with base64 image source blocks followed by a text block. If the selected endpoint reports no native schema capability, omit provider-side schema enforcement and rely on the same local parser/validator/repair path.
- Gemini preset: `generateContent` with `inline_data` image parts and JSON response configuration using the `DesignSpec` schema.

Do not hardcode a default model name. The user must enter or select a model; `test_connection` determines whether the configured model accepts image input.

- [ ] **Step 2: Implement the interface**

```rust
#[async_trait]
pub trait MultimodalProvider: Send + Sync {
    async fn test_connection(&self) -> Result<ProviderCapabilities, ProviderError>;
    async fn analyze(&self, request: AnalysisRequest) -> Result<RawModelResponse, ProviderError>;
}
```

`AnalysisRequest` contains model, prompt, JSON Schema, and image bytes with media types. It must not contain filesystem paths.

Create a factory:

```rust
pub fn build_provider(
    config: &ProviderConfig,
    secret: SecretString,
    client: reqwest::Client,
) -> Result<Box<dyn MultimodalProvider>, ProviderError>;
```

It maps all four `ProviderKind` values to the corresponding adapter. `SecretString` must redact its `Debug` and `Display` output.

- [ ] **Step 3: Implement request logging redaction**

Logs may contain provider ID, model, image count, duration, status code, and request ID. They must never contain authorization headers, base64 image data, prompts, or response bodies.

Run: `cargo test -p design-providers`

Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add src-tauri/crates/design-providers
git commit -m "feat: add multimodal provider adapter"
```

### Task 8: Implement prompt construction, schema validation, repair, and provenance

**Files:**
- Create: `src-tauri/crates/design-analysis/Cargo.toml`
- Create: `src-tauri/crates/design-analysis/src/lib.rs`
- Create: `src-tauri/crates/design-analysis/src/prompt.rs`
- Create: `src-tauri/crates/design-analysis/src/orchestrator.rs`
- Create: `src-tauri/crates/design-analysis/src/repair.rs`
- Create: `src-tauri/crates/design-analysis/tests/orchestrator.rs`
- Create: `src-tauri/crates/design-analysis/tests/fixtures/invalid-response.json`

- [ ] **Step 1: Write orchestration tests with a fake provider**

Tests must cover:

1. Valid first response creates one persisted spec version.
2. Invalid JSON triggers exactly one repair request.
3. Invalid repaired output creates no formal version.
4. Missing evidence IDs fail validation.
5. Low-confidence and conflicting findings remain `pending`.
6. Provider failure creates no version and preserves project state.

- [ ] **Step 2: Implement the fixed analysis prompt**

The prompt must state:

```text
Analyze only visible design evidence.
Separate observations, cross-screenshot patterns, and recommendations.
Do not copy or emit brand names, logos, original product copy, customer data, or proprietary assets.
Every pattern and recommendation must cite evidence IDs.
Use exact values only when visually supported; otherwise describe a range or principle.
Return only JSON matching the supplied schema.
```

Append project platform, target product type, screenshot IDs, page names, and scenes. Do not append local paths.

- [ ] **Step 3: Implement `AnalysisOrchestrator`**

```rust
pub async fn analyze_project(
    &self,
    project_id: Uuid,
    provider_id: Uuid,
    screenshot_ids: Vec<Uuid>,
) -> Result<AnalysisOutcome, AnalysisError>;
```

Flow: load metadata → read selected image bytes → show-ready request summary → call provider → parse fenced or plain JSON → deserialize → validate → one repair call if needed → attach provider/model/screenshot provenance → insert an immutable version and replace the project draft transactionally.

- [ ] **Step 4: Run tests**

```powershell
cargo test -p design-analysis
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/crates/design-analysis
git commit -m "feat: orchestrate validated design analysis"
```

## Milestone 5: Tauri command boundary

### Task 9: Expose narrow, typed desktop commands

**Files:**
- Create: `src-tauri/src/state.rs`
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/projects.rs`
- Create: `src-tauri/src/commands/screenshots.rs`
- Create: `src-tauri/src/commands/providers.rs`
- Create: `src-tauri/src/commands/analysis.rs`
- Create: `src-tauri/src/commands/rules.rs`
- Create: `src-tauri/src/commands/exports.rs`
- Create: `src/lib/desktop.ts`
- Create: `src/lib/desktop.test.ts`

- [ ] **Step 1: Define the command surface**

Commands:

```text
list_projects
create_project
rename_project
archive_project
delete_project
list_screenshots
import_screenshots
update_screenshot_metadata
remove_screenshot
list_providers
save_provider
delete_provider
test_provider
preview_analysis_request
analyze_project
get_design_spec
update_rule
list_exports
export_design_markdown
```

`preview_analysis_request` returns provider name, model, image IDs, image count, and estimated encoded bytes. It returns no image bytes or credentials.

- [ ] **Step 2: Write frontend wrapper tests**

Mock `@tauri-apps/api/core` and assert:

```ts
await desktop.createProject({ name: "Finance", platform: "mobile" });
expect(invoke).toHaveBeenCalledWith("create_project", {
  input: { name: "Finance", platform: "mobile" },
});
```

- [ ] **Step 3: Implement `desktop.ts`**

Every command has a typed wrapper. Components may import `desktop`, but may not import `invoke`.

- [ ] **Step 4: Register Tauri commands and initialize state**

At startup create the app-data directory, open SQLite, run migrations, initialize credential storage and reqwest client, then register commands. Initialization failure must show a fatal startup error rather than silently starting an unusable UI.

Run:

```powershell
cargo test --workspace
npm test
npm run build
```

Expected: PASS.

- [ ] **Step 5: Commit**

```powershell
git add src src-tauri
git commit -m "feat: expose typed desktop command boundary"
```

## Milestone 6: User workflow

### Task 10: Build project list and screenshot management

**Files:**
- Create: `src/features/projects/ProjectListPage.tsx`
- Create: `src/features/projects/ProjectListPage.test.tsx`
- Create: `src/features/projects/NewProjectDialog.tsx`
- Create: `src/features/screenshots/ScreenshotManagerPage.tsx`
- Create: `src/features/screenshots/ScreenshotManagerPage.test.tsx`
- Modify: `src/app/App.tsx`

- [ ] **Step 1: Write user-flow tests**

Test:

```tsx
test("creates a project and opens screenshot management", async () => {
  renderApp();
  await user.click(screen.getByRole("button", { name: "New project" }));
  await user.type(screen.getByLabelText("Project name"), "Finance app");
  await user.selectOptions(screen.getByLabelText("Target platform"), "mobile");
  await user.click(screen.getByRole("button", { name: "Create project" }));
  expect(await screen.findByRole("heading", { name: "Reference screenshots" })).toBeVisible();
});
```

Also test empty state, validation, archive, delete confirmation, import error, duplicate screenshot, metadata editing, sorting, and removing a screenshot.

Add an offline regression test where all desktop project/screenshot commands succeed while provider commands are unavailable; the project list, screenshot manager, rule editor, and existing Markdown preview must remain usable.

- [ ] **Step 2: Implement routes**

```text
/                         project list
/projects/:projectId      screenshot manager
/projects/:projectId/providers
/projects/:projectId/analyze
/projects/:projectId/workbench
/projects/:projectId/exports
```

- [ ] **Step 3: Implement the project and screenshot pages**

Use native file selection through a Tauri dialog command. Display page name, scene, platform, dimensions, and validation errors. Require at least one screenshot before enabling “Configure analysis”; recommend three or more without blocking.

Run: `npm test -- src/features/projects src/features/screenshots`

Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add src
git commit -m "feat: manage local projects and screenshots"
```

### Task 11: Build provider setup and transmission disclosure

**Files:**
- Create: `src/features/providers/ProviderSettingsPage.tsx`
- Create: `src/features/providers/ProviderSettingsPage.test.tsx`
- Create: `src/features/analysis/AnalysisStartPage.tsx`
- Create: `src/features/analysis/AnalysisStartPage.test.tsx`

- [ ] **Step 1: Write provider setup tests**

Cover preset selection, OpenAI-compatible Base URL, model name, API key entry, connection test, invalid key, model without image support, and saved-provider display where the secret is represented only as “Stored securely”.

- [ ] **Step 2: Write disclosure test**

```tsx
test("shows exactly what leaves the device before analysis", async () => {
  renderAnalysisStart();
  expect(await screen.findByText("Provider: My endpoint")).toBeVisible();
  expect(screen.getByText("Model: vision-model")).toBeVisible();
  expect(screen.getByText("3 images will be sent")).toBeVisible();
  expect(screen.getByRole("button", { name: "Send and analyze" })).toBeEnabled();
});
```

- [ ] **Step 3: Implement forms and state**

Do not display or refill existing API keys. Replacing a key requires a fresh value. Disable analysis until provider connection test passes in the current configuration.

Run: `npm test -- src/features/providers src/features/analysis/AnalysisStartPage.test.tsx`

Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add src
git commit -m "feat: configure multimodal analysis safely"
```

### Task 12: Build the three-column review workbench

**Files:**
- Create: `src/features/workbench/WorkbenchPage.tsx`
- Create: `src/features/workbench/WorkbenchPage.test.tsx`
- Create: `src/features/workbench/EvidencePanel.tsx`
- Create: `src/features/workbench/RuleEditor.tsx`
- Create: `src/features/workbench/MarkdownPreview.tsx`
- Create: `src/features/workbench/ruleGroups.ts`

- [ ] **Step 1: Write interaction tests**

Cover:

- Selecting a rule highlights its evidence screenshots.
- Accept changes `pending` to `accepted`.
- Editing changes status to `edited` and source to `user`.
- Reject removes the rule from Markdown preview but keeps it in the spec.
- Low-confidence rules are visibly marked.
- Missing evidence is visibly marked.
- Keyboard navigation works through rule groups and actions.

Example:

```tsx
test("rejecting a rule removes it from the markdown preview", async () => {
  renderWorkbench(specWithPendingRule);
  await user.click(screen.getByRole("button", { name: "Reject rule" }));
  expect(screen.getByTestId("markdown-preview")).not.toHaveTextContent(
    "Use 12px card radii",
  );
});
```

- [ ] **Step 2: Implement layout**

Desktop widths at or above 1100 px use `24% / 46% / 30%`. Narrow windows switch to tabbed Evidence, Rules, and Preview views. Keep the rule editor as structured controls; do not add chat.

- [ ] **Step 3: Implement optimistic rule edits with rollback**

Update local preview immediately, invoke `update_rule`, and restore the prior rule with an error banner if persistence fails. Persist one rule mutation per command by updating the project row in `design_spec_drafts`; never mutate an immutable `design_spec_versions` row.

Run: `npm test -- src/features/workbench`

Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add src
git commit -m "feat: review and correct extracted design rules"
```

### Task 13: Build export history and deterministic file export

**Files:**
- Create: `src/features/exports/ExportHistoryPage.tsx`
- Create: `src/features/exports/ExportHistoryPage.test.tsx`
- Create: `src-tauri/crates/design-storage/src/exports.rs`
- Create: `src-tauri/crates/design-storage/tests/exports.rs`

- [ ] **Step 1: Write backend export tests**

Assert that export:

1. Refuses a spec with zero accepted/edited rules.
2. Produces `exports/{timestamp}-DESIGN.md`.
3. Writes through a temporary file and atomic rename.
4. Inserts history only after successful file creation.
5. Re-exporting the same spec creates a new immutable history row.

- [ ] **Step 2: Write frontend history tests**

Cover preview, copy, reveal in folder, and export-current-version. The list displays export timestamp and source spec version.

- [ ] **Step 3: Implement export**

The command snapshots the current draft as an immutable `design_spec_versions` row, calls only `design_core::compile_markdown` on that snapshot, and records the resulting file in `export_versions`. Frontend Markdown is preview-only and is never used as the exported source.

Run:

```powershell
cargo test -p design-storage --test exports
npm test -- src/features/exports
```

Expected: PASS.

- [ ] **Step 4: Commit**

```powershell
git add src src-tauri
git commit -m "feat: export and retain design markdown history"
```

## Milestone 7: Release confidence

### Task 14: Add end-to-end fixtures, privacy regression tests, and packaging

**Files:**
- Create: `src-tauri/tests/end_to_end.rs`
- Create: `src-tauri/tests/fixtures/provider-success.json`
- Create: `src-tauri/tests/fixtures/provider-invalid.json`
- Create: `tests/privacy.test.ts`
- Create: `docs/USER_GUIDE.md`
- Create: `docs/PRIVACY.md`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `README.md`

- [ ] **Step 1: Add the backend happy-path integration test**

The test uses a temporary app-data directory and fake provider:

```rust
#[tokio::test]
async fn screenshot_to_reviewed_markdown_is_local_and_repeatable() {
    let app = TestApp::new().await;
    let project = app.create_project("Finance", Platform::Mobile).await;
    let screenshot = app.import_fixture(project.id, "dashboard.png").await;
    let version = app.analyze(project.id, vec![screenshot.id]).await;
    app.accept_all_rules(version.id).await;
    let export = app.export(version.id).await;
    assert!(export.contents.contains("# Design intent"));
    assert!(!export.contents.contains("OriginalBrand"));
}
```

- [ ] **Step 2: Add privacy regressions**

Scan the temporary database, project directory, exports, and captured logs for the test API key, authorization header, base64 prefix, prompt text, and provider response body. All assertions must prove absence.

Instrument the HTTP client factory and assert that application startup, project CRUD, screenshot import, rule editing, preview, and export make zero network requests. Only `test_provider` and `analyze_project` may invoke provider endpoints.

- [ ] **Step 3: Add frontend critical-flow coverage**

Using mocked desktop commands, cover:

```text
create project → import screenshots → configure provider → review transmission
→ run analysis → accept/edit/reject rules → export DESIGN.md → view history
```

- [ ] **Step 4: Add user and privacy documentation**

`docs/USER_GUIDE.md` must explain prerequisites, provider setup, local project location, screenshot workflow, review semantics, export, and recovery from provider errors.

`docs/PRIVACY.md` must state:

- Local data locations by operating system.
- What stays local.
- What is sent when analysis starts.
- How credentials are stored.
- How project deletion works.
- That configured AI providers have their own retention policies.

- [ ] **Step 5: Verify release build**

Run:

```powershell
npm ci
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --workspace
npm run tauri build
```

Expected: all tests pass and a Windows installer is created under `src-tauri/target/release/bundle/`.

- [ ] **Step 6: Manual acceptance pass**

Use a real multimodal endpoint and three non-sensitive screenshots. Confirm:

- The preflight names provider, model, and three selected images.
- Analysis produces rules with evidence and confidence.
- Rule edits survive application restart.
- Rejected rules do not appear in exported Markdown.
- API keys do not appear in project files or logs.
- The exported file has all ten fixed sections.
- A second AI can implement a new page from `DESIGN.md` without seeing the screenshots.

- [ ] **Step 7: Commit**

```powershell
git add .
git commit -m "test: verify local-first MVP workflow"
```

## Deferred until after Phase A validation

Do not implement these in this plan:

- CLI or MCP server.
- URL capture, Figma, or video imports.
- Multiple simultaneous analysis jobs.
- Team accounts, cloud sync, sharing, or billing.
- Automatic component-code generation.
- Provider-specific prompt tuning beyond request-shape adapters.
- Public plugin architecture for third-party adapters.

## Implementation references

- Tauri project creation: https://v2.tauri.app/start/create-project/
- Tauri frontend-to-Rust commands: https://v2.tauri.app/develop/calling-rust/
- Tauri security and capabilities: https://v2.tauri.app/security/capabilities/
- Rust keyring crate: https://docs.rs/keyring/latest/keyring/
- OpenAI image input: https://developers.openai.com/api/docs/guides/images-vision
- Anthropic vision: https://platform.claude.com/docs/en/build-with-claude/vision
- Gemini image input: https://ai.google.dev/gemini-api/docs/image-understanding
- Gemini structured output: https://ai.google.dev/gemini-api/docs/structured-output
