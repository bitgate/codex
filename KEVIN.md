# KEVIN.md

## What this is
Fork of openai/codex. Default branch `main` tracks upstream. Our work lives on feature branches + PRs.

## Active work
- **Profile model selector** (PR #1, branch `feature/profile-model-selector`): config profiles (`[profiles.<name>]` in config.toml or `<name>.config.toml` in CODEX_HOME) surface as `model/list` entries → desktop GUI picker shows them; `thread/start` expands profile → real model/provider (+effort); settings updates expand model only and reject cross-provider profiles (providers are fixed per thread). Key files: `codex-rs/app-server/src/profiles.rs`, `models.rs`, `request_processors/{catalog,thread,turn}_processor.rs`.
- **codex-binaries workflow**: builds `codex` CLI per platform → rolling `fork-binaries` release. Consumed by bitgate/CodexDesktop-Rebuild via `CODEX_VENDOR_BIN`.

## Gotchas
- `model/list` catalog is backend-driven only (OpenAI presets / `model_catalog_json`); custom `model_providers` from config.toml never reach clients unless profiles (our patch) or a catalog file is used.
- Full `cargo check -p codex-app-server` from cold ≈ 12 min; tests ≈ 20+ min. Plan long e2b polls.
- Workspace is huge — sparse checkout breaks cargo (workspace members missing); need full checkout for builds.
