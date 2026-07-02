# Design Language Extractor User Guide

## Prerequisites

- Windows desktop environment.
- A multimodal AI provider API key.
- Local reference screenshots in PNG, JPEG, or WebP format.

## Basic workflow

1. Create a project and choose the target platform.
2. Import local screenshots.
3. Configure a multimodal provider.
4. Review the transmission disclosure before analysis.
5. Run analysis.
6. Accept, edit, or reject extracted design rules.
7. Export `DESIGN.md`.

## Provider setup

Open the provider setup page from a project after importing screenshots. Choose OpenAI, Anthropic, Gemini, or an OpenAI-compatible endpoint. API keys are submitted once and are not refilled into the form. Replacing a key requires entering a fresh value.

Run the connection test before continuing. Analysis remains blocked until the selected provider reports image input support.

## Local project location

The app stores project data under the operating system app-data directory for `com.tomd.designlanguageextractor`. Inside each project directory, screenshots are copied into a managed `screenshots/` folder and exports are written under `exports/`.

## Screenshot workflow

Use non-sensitive screenshots where possible. The app copies imported files into its local project directory, records dimensions and media type, and rejects unsupported, corrupt, oversized, duplicate, or unsafe paths.

Import at least one screenshot to enable analysis. Three or more screenshots are recommended for stronger pattern extraction.

## Review semantics

AI-generated rules begin as pending when they are low-confidence or need user judgment. You can:

- Accept a rule so it appears in exported Markdown.
- Edit a rule, which marks it as user-authored.
- Reject a rule, which keeps it in the project record but removes it from exported Markdown.

The Markdown preview is for review only. The actual export is compiled from the validated backend design spec snapshot.

## Export

Export creates an immutable design spec snapshot and a timestamped file:

```text
exports/{timestamp}-DESIGN.md
```

Export history shows when each file was created and which spec version it came from.

## Recovering from provider errors

If provider testing or analysis fails:

- Recheck the API key and model name.
- Verify the endpoint supports image input.
- Try fewer or smaller screenshots.
- Keep local project data; failed provider calls do not delete projects, screenshots, or existing reviewed rules.
