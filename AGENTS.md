# Repository Guidelines

## Project Structure & Module Organization
- Repository root hosts contributor docs, devcontainer assets, and the Rust workspace under `mawaku-rs/`.
- The CLI crate lives in `mawaku-rs/mawaku-cli`; `src/main.rs` exposes the Clap-based entrypoint and handles prompt selection.
- Shared configuration code (including config file discovery and defaults) is in `mawaku-rs/mawaku-config/src/lib.rs`.
- Create integration tests under `mawaku-rs/tests/` (auto-discovered by Cargo) and sample prompts or fixtures inside `mawaku-rs/fixtures/` if needed.
- Keep generated assets out of version control; prefer referencing reproducible commands instead.

## Build, Test, and Development Commands
- From `mawaku-rs/`, run `cargo run -p mawaku -- --help` to exercise the CLI and verify flags/output. The first run seeds a config file with the default prompt.
- `cargo check`: type-check and validate dependencies without producing a binary—use this before committing.
- `cargo test`: execute unit and integration tests; add `-- --nocapture` for verbose CLI output when debugging.
- `cargo fmt`: format the codebase using Rustfmt’s default project settings.

## Coding Style & Naming Conventions
- Follow Rust edition 2021 defaults with Rustfmt; 4-space indentation and snake_case for modules, functions, and variables.
- Use CamelCase for type names and struct variants, mirroring standard Rust guidelines.
- Keep CLI arguments descriptive (e.g., `--prompt`) and document them with Clap attributes. The CLI should gracefully fall back to config defaults when flags are omitted.

## Testing Guidelines
- Co-locate lightweight unit tests below the function they cover using Rust’s `#[cfg(test)]` pattern.
- Place scenario-level tests in `mawaku-rs/tests/` to exercise the CLI end-to-end via `assert_cmd`.
- Aim to cover argument parsing, configuration fallbacks, prompt rewriting, and provider selection logic as those features mature.

## Commit & Pull Request Guidelines
- Write imperative, present-tense commit subjects (e.g., “Add prompt rewrite scaffold”).
- Group related changes in a single commit; avoid mixing refactors with feature work.
- Pull requests should summarize the problem, the solution, and testing performed; link to tracking issues or product specs when available.
- Provide CLI screenshots or sample outputs when behavior changes, especially for help text or new subcommands.
