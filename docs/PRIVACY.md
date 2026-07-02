# Privacy

## Local data locations

Project files are stored in the app-data directory for `com.tomd.designlanguageextractor`.

- Windows: `%APPDATA%\com.tomd.designlanguageextractor`
- macOS: `~/Library/Application Support/com.tomd.designlanguageextractor`
- Linux: `~/.local/share/com.tomd.designlanguageextractor`

The exact path can vary by OS configuration.

## What stays local

The following stay local unless you explicitly send analysis to a configured provider:

- Project metadata.
- Imported screenshot copies.
- Draft and versioned design specs.
- Rule edits and review status.
- Exported `DESIGN.md` files.
- Export history.

The project list, screenshot import, rule editing, Markdown preview, and export history do not require provider network calls.

## What is sent when analysis starts

After you confirm the transmission disclosure, the app sends:

- The selected provider and model request.
- The selected screenshots.
- A design-analysis prompt.
- Screenshot IDs and local metadata needed for provenance.

The disclosure screen lists provider, model, selected image IDs, image count, and estimated encoded payload before sending.

## Credentials

API keys are stored through the desktop backend credential store. Provider records exposed to the frontend contain only whether a credential exists. Existing API keys are never displayed or refilled into forms.

## Project deletion

Deleting a project removes its database rows and managed project directory, including copied screenshots and exports. If filesystem cleanup fails, the app reports the cleanup path instead of silently ignoring it.

## Provider retention policies

Configured AI providers have their own logging and retention policies. Review the provider’s policy before sending sensitive screenshots.
