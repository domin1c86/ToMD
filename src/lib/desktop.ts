import { invoke } from "@tauri-apps/api/core";

import type { DesignSpec, Platform, RuleStatus } from "../generated/bindings";

export type Project = {
  id: string;
  name: string;
  platform: Platform;
  archived_at: string | null;
  created_at: string;
  updated_at: string;
};

export type Screenshot = {
  id: string;
  project_id: string;
  relative_path: string;
  sha256: string;
  media_type: string;
  width: number;
  height: number;
  page_name: string;
  scene: string;
  sort_order: number;
  created_at: string;
};

export type ProviderKind =
  | "open_ai"
  | "anthropic"
  | "gemini"
  | "openai_compatible";

export type Provider = {
  id: string;
  name: string;
  kind: ProviderKind;
  base_url: string;
  model: string;
  credential_ref: string;
  has_credential: boolean;
};

export type ProviderCapabilities = {
  image_input: boolean;
  structured_output: boolean;
};

export type AnalysisPreview = {
  provider_name: string;
  model: string;
  image_ids: string[];
  image_count: number;
  estimated_encoded_bytes: number;
};

export type AnalysisOutcome = {
  version_id: string;
  repair_attempted: boolean;
  spec: DesignSpec;
};

export type ExportVersion = {
  id: string;
  project_id: string;
  spec_version_id: string;
  relative_path: string;
  created_at: string;
};

export type CreateProjectInput = {
  name: string;
  platform: Platform;
};

export type RenameProjectInput = {
  projectId: string;
  name: string;
};

export type ProjectIdInput = {
  projectId: string;
};

export type ListProjectsInput = {
  includeArchived?: boolean;
};

export type ListScreenshotsInput = ProjectIdInput;

export type ImportScreenshotsInput = ProjectIdInput & {
  paths: string[];
};

export type UpdateScreenshotMetadataInput = ProjectIdInput & {
  screenshotId: string;
  pageName: string;
  scene: string;
  sortOrder: number;
};

export type RemoveScreenshotInput = ProjectIdInput & {
  screenshotId: string;
};

export type SaveProviderInput = {
  providerId?: string;
  name: string;
  kind: ProviderKind;
  baseUrl: string;
  model: string;
  apiKey?: string;
};

export type ProviderIdInput = {
  providerId: string;
};

export type AnalysisSelectionInput = ProjectIdInput & {
  providerId: string;
  screenshotIds: string[];
};

export type UpdateRuleInput = ProjectIdInput & {
  ruleId: string;
  statement?: string;
  status?: RuleStatus;
};

export type ExportDesignMarkdownInput = ProjectIdInput & {
  destinationPath?: string;
};

const command = <Output, Input = undefined>(
  name: string,
  input?: Input,
): Promise<Output> => {
  if (input === undefined) {
    return invoke<Output>(name);
  }

  return invoke<Output>(name, { input });
};

export const desktop = {
  listProjects: (input: ListProjectsInput = {}) =>
    command<Project[], ListProjectsInput>("list_projects", input),
  createProject: (input: CreateProjectInput) =>
    command<Project, CreateProjectInput>("create_project", input),
  renameProject: (input: RenameProjectInput) =>
    command<Project, RenameProjectInput>("rename_project", input),
  archiveProject: (input: ProjectIdInput) =>
    command<void, ProjectIdInput>("archive_project", input),
  deleteProject: (input: ProjectIdInput) =>
    command<void, ProjectIdInput>("delete_project", input),

  listScreenshots: (input: ListScreenshotsInput) =>
    command<Screenshot[], ListScreenshotsInput>("list_screenshots", input),
  importScreenshots: (input: ImportScreenshotsInput) =>
    command<Screenshot[], ImportScreenshotsInput>("import_screenshots", input),
  updateScreenshotMetadata: (input: UpdateScreenshotMetadataInput) =>
    command<Screenshot, UpdateScreenshotMetadataInput>(
      "update_screenshot_metadata",
      input,
    ),
  removeScreenshot: (input: RemoveScreenshotInput) =>
    command<void, RemoveScreenshotInput>("remove_screenshot", input),

  listProviders: () => command<Provider[]>("list_providers"),
  saveProvider: (input: SaveProviderInput) =>
    command<Provider, SaveProviderInput>("save_provider", input),
  deleteProvider: (input: ProviderIdInput) =>
    command<void, ProviderIdInput>("delete_provider", input),
  testProvider: (input: ProviderIdInput) =>
    command<ProviderCapabilities, ProviderIdInput>("test_provider", input),

  previewAnalysisRequest: (input: AnalysisSelectionInput) =>
    command<AnalysisPreview, AnalysisSelectionInput>(
      "preview_analysis_request",
      input,
    ),
  analyzeProject: (input: AnalysisSelectionInput) =>
    command<AnalysisOutcome, AnalysisSelectionInput>("analyze_project", input),
  getDesignSpec: (input: ProjectIdInput) =>
    command<DesignSpec, ProjectIdInput>("get_design_spec", input),
  updateRule: (input: UpdateRuleInput) =>
    command<DesignSpec, UpdateRuleInput>("update_rule", input),

  listExports: (input: ProjectIdInput) =>
    command<ExportVersion[], ProjectIdInput>("list_exports", input),
  exportDesignMarkdown: (input: ExportDesignMarkdownInput) =>
    command<ExportVersion, ExportDesignMarkdownInput>(
      "export_design_markdown",
      input,
    ),
};
