# Design Language Extractor

A local-first Tauri desktop app that turns website or app screenshots into a reviewed, provider-neutral `DESIGN.md`.

## What it does

- Creates local projects.
- Imports local reference screenshots.
- Configures a multimodal AI provider without exposing stored keys to the frontend.
- Shows exactly what will leave the device before analysis.
- Lets users accept, edit, or reject extracted design rules.
- Exports timestamped Markdown history from backend-validated design spec snapshots.

## Development

```powershell
npm install
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --workspace
npm run tauri build
```

## Documentation

- [User guide](docs/USER_GUIDE.md)
- [Privacy](docs/PRIVACY.md)

## Privacy model

Project CRUD, screenshot import, rule editing, preview, and export are local. Provider network calls are limited to connection tests and explicit analysis runs after the transmission disclosure.
