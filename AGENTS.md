# Repository Guidelines

## Project Structure & Module Organization

This workspace currently contains cookie files `*.cookies.txt` and environment-managed directories (`.agents/`, `.codex/`, `.aws/`, and `.git/`). No application source, tests, assets, or dependency manifests are present. Do not assume a language, framework, or runtime. Leave environment-managed directories untouched unless the task explicitly requires changes.

When adding application code, use a simple structure appropriate to the chosen language and document its entry point and supporting directories here.

## Build, Test, and Development Commands

No build, test, lint, or local development commands are configured. Do not assume commands such as `npm test` or `make build` exist.

Useful inspection commands:

- `ls -la`: inspect the workspace, including hidden entries.
- `rg --files`: list visible project files without reading their contents.

When introducing tooling, document the exact installation, development, and validation commands alongside the configuration that enables them.

## Coding Style & Naming Conventions

No existing source code establishes indentation or naming conventions. Follow the surrounding style when code becomes available. For Markdown, use descriptive headings, short paragraphs, and fenced blocks for multiline commands. Use descriptive filenames and avoid adding dependencies or configuration without a concrete need.

## Testing Guidelines

No testing framework or coverage threshold is configured. Add focused tests with new behavior, including relevant failure cases. Use the selected framework’s standard test naming and directory conventions, and document how to run them. Documentation-only changes require checking accuracy and Markdown formatting.

## Commit & Pull Request Guidelines

Git history is unavailable in this workspace, so no established commit convention can be verified. Use concise, imperative commit subjects. Pull requests should explain the change, its purpose, validation performed, and any remaining limitations. Link relevant issues when available.

## Security & Configuration

Treat cookie files `*.cookies.txt` as sensitive authentication material. Never print its contents, copy cookie values into documentation or tests, or include them in commits or pull requests. Use sanitized examples when explaining authentication configuration.

## Current Application (2026-10-26)

The bootstrap notes above predate the Rust application now present. CLI entry
point: src/main.rs. Shared SQLite bindings: src/db.rs. Downloads/collection
networking live under src/; offline media processing lives in src/processing/.
Monthly Discogs artist/master resources and their tests live in src/resources.rs.
Runtime mapping data lives under data/ and skill source under skills/. Resource
outputs belong under <output>/resources/; never commit extracts or SQLite files.

Validation commands: cargo test --offline; cargo fmt --check;
cargo clippy --offline --all-targets -- -D warnings; cargo build --offline --release.
README documents system dependencies and the sibling VCW build dependency.

## Documentation Preferences

Use US English spelling and do not use em dashes in documentation or replies.

Use `johnsmith` as the collection owner in all documentation, sample commands,
and deployment guides. Do not use Stue's real collection username in examples,
including personalized deployment documentation.
