# Repository Guidelines

## Project Structure & Module Organization
- Root directory hosts `Cargo.toml`, `README.md`, and contributor docs such as this guide.
- Core source lives in `src/`; `main.rs` currently exposes the CLI entrypoint via `clap`.
- Create integration tests under `tests/` (auto-discovered by Cargo) and sample prompts or fixtures inside `fixtures/` if needed.
- Keep generated assets out of version control; prefer referencing reproducible commands instead.

## Build, Test, and Development Commands
- `cargo run -- --help`: run the CLI locally to verify flags and output.
- `cargo check`: type-check and validate dependencies without producing a binary—use this before committing.
- `cargo test`: execute unit and integration tests; add `-- --nocapture` for verbose CLI output when debugging.
- `cargo fmt`: format the codebase using Rustfmt’s default project settings.

## Coding Style & Naming Conventions
- Follow Rust edition 2021 defaults with Rustfmt; 4-space indentation and snake_case for modules, functions, and variables.
- Use CamelCase for type names and struct variants, mirroring standard Rust guidelines.
- Keep CLI arguments descriptive (`--prompt`, `--provider`) and document them with Clap attributes.

## Testing Guidelines
- Co-locate lightweight unit tests below the function they cover using Rust’s `#[cfg(test)]` pattern.
- Place scenario-level tests in `tests/` to exercise the CLI end-to-end via `assert_cmd`.
- Aim to cover argument parsing, prompt rewriting, and provider selection logic as those features mature.

## Commit & Pull Request Guidelines
- Write imperative, present-tense commit subjects (e.g., “Add prompt rewrite scaffold”).
- Group related changes in a single commit; avoid mixing refactors with feature work.
- Pull requests should summarize the problem, the solution, and testing performed; link to tracking issues or product specs when available.
- Provide CLI screenshots or sample outputs when behavior changes, especially for help text or new subcommands.
