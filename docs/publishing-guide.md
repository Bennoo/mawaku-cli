# Publishing the Mawaku CLI to crates.io

This guide walks through every step required to turn the Mawaku CLI workspace into a versioned package on [crates.io](https://crates.io) so it can be consumed by richer orchestration systems. It covers technical prerequisites, build hygiene, release automation, and long-term maintenance considerations so future services can depend on a stable crate.

> **Scope:** All commands below are issued from the repository root (`/workspace/mawaku-cli`) unless noted otherwise.

---

## 1. Understand the Workspace Layout

Mawaku ships as a Cargo workspace rooted at `mawaku-rs/`. The CLI crate we publish lives at `mawaku-rs/mawaku-cli` and is named `mawaku` in its `Cargo.toml`.

```
mawaku-cli/
├── README.md        # User-facing documentation (referenced by the crate)
└── mawaku-rs/
    ├── Cargo.toml   # Workspace members and shared dependencies
    ├── mawaku-cli/  # CLI entrypoint we publish
    ├── mawaku-utils # Shared helpers imported by the CLI
    ├── mawaku-config
    ├── mawaku-image
    └── mawaku-gemini
```

Publishing the CLI therefore requires coordinating versions across the CLI crate and any internal crates whose code is bundled into the final package (`mawaku-config`, `mawaku-gemini`, `mawaku-image`, and `mawaku-utils`).

---

## 2. Prerequisites

1. **Rust toolchain**: Install Rust 1.76+ with `rustup` to ensure the workspace compiles locally.
2. **crates.io account**: Create an account on [crates.io](https://crates.io/), link it to a GitHub identity, and confirm your email.
3. **API token**: Generate a new crates.io API token via **Account → API Tokens**. Store it in a password manager; you will use it with `cargo login`.
4. **Access to Google Gemini** (optional but recommended for end-to-end tests): The CLI integrates with Gemini for rendering imagery, so having an API key ensures integration tests stay relevant.
5. **Documentation assets**: Screenshot PNGs referenced in `README.md` live under `docs/examples/`. Ensure they exist in the repository before publishing because crates.io packages embed the README but **not** binary assets; use absolute URLs if images must render on crates.io.

---

## 3. Versioning Strategy

1. **Update crate versions**: The CLI package (`mawaku-rs/mawaku-cli/Cargo.toml`) currently declares `version = "0.0.0"`. Bump it following [Semantic Versioning](https://semver.org/). For example, change to `0.1.0` for the first public release.
2. **Align internal crates**: Any crate published alongside the CLI (e.g., `mawaku-config`) must also carry compatible versions. If they remain workspace-only, ensure they are marked with `publish = false` to prevent accidental uploads.
3. **Workspace lockfile**: Run `cargo update` from `mawaku-rs/` to refresh `Cargo.lock` so the published crate references exact dependency versions.
4. **Changelog entry**: Create or update a `CHANGELOG.md` with a section describing the release. Include CLI flags, configuration behaviors, and integration notes for downstream systems.

---

## 4. Prepare the Codebase

1. **Install dependencies**
   ```bash
   cd mawaku-rs
   cargo fetch
   ```
2. **Format & lint**
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --all-features -- -D warnings
   ```
   `cargo fmt` prevents style drift, while `cargo clippy` surfaces logic issues that could break automated orchestration.
3. **Run tests**
   ```bash
   cargo test --all --all-features
   ```
   Include integration tests under `mawaku-rs/tests/` to exercise Gemini prompts, configuration fallbacks, and image generation.
4. **Manual CLI sanity check**
   ```bash
   cargo run -p mawaku -- --location "Lisbon, Portugal" --season spring --time-of-day dusk
   ```
   Verify the command succeeds, writes `~/.mawaku/config.toml`, and saves images to the configured output directory.
5. **Docker validation (optional but recommended)**
   ```bash
   docker build -f mawaku-rs/Dockerfile -t mawaku-cli mawaku-rs
   docker run --rm mawaku-cli --help
   ```
   Future infrastructure can reuse this container image, so confirm it behaves identically to the local binary.

---

## 5. Author High-Signal Documentation

1. **README alignment**: The CLI crate references `../../README.md` as its crates.io landing page. Confirm it explains:
   - Install instructions (`cargo install mawaku`).
   - Configuration steps (`~/.mawaku/config.toml` defaults).
   - Spotlight prompt examples.
2. **Publish-specific section**: Add a "Publishing" or "Release process" section describing how to reproduce the release. Downstream maintainers rely on this to trace provenance.
3. **API references**: Document CLI flags in `README.md` using a table or bullet list. Complex systems may invoke the binary via `std::process::Command`, so they need deterministic flag names (`--location`, `--season`, `--time-of-day`, `--set-gemini-api-key`).
4. **Docs directory**: Store extended guides (like this file) under `docs/` to keep the README concise yet linkable.

---

## 6. Validate the Package Before Publishing

1. **Dry-run package assembly**
   ```bash
   cd mawaku-rs
   cargo package -p mawaku
   ```
   This produces a `.crate` file under `target/package/` without uploading it. Inspect the manifest contents to ensure only necessary files are bundled.
2. **Inspect contents**
   ```bash
   tar -tf target/package/mawaku-<VERSION>.crate
   ```
   Confirm `README.md`, `LICENSE`, and `src/` files are present. Remove large binaries or credentials via `.gitignore` / `.cargo_vcs_info.json` if needed.
3. **Check size limits**: crates.io enforces a 10 MB upload limit. If screenshot assets would exceed that, replace them with hosted URLs inside the README.
4. **Re-run tests**: After packaging, run `cargo test` again to ensure no files were accidentally removed.

---

## 7. Publish to crates.io

1. **Authenticate Cargo**
   ```bash
   cargo login <CRATES_IO_TOKEN>
   ```
   This writes the token to `~/.cargo/credentials`. Only run once per machine.
2. **Publish internal dependencies first** (if applicable). If `mawaku-config`, `mawaku-utils`, etc. must also be available on crates.io, publish them in dependency order so the CLI can resolve them. Otherwise, mark them `publish = false` to keep them workspace-only.
3. **Upload the CLI**
   ```bash
   cd mawaku-rs
   cargo publish -p mawaku
   ```
   Cargo rebuilds the package, verifies the signature, and uploads it. Publishing is irreversible, so double-check versions beforehand.
4. **Monitor crates.io**: Within a few minutes, `https://crates.io/crates/mawaku` should display the new version. Verify the README renders correctly.

---

## 8. Tag and Announce the Release

1. **Create a Git tag**
   ```bash
   git tag -a v0.1.0 -m "Mawaku CLI v0.1.0"
   git push origin v0.1.0
   ```
   Tags help downstream systems pin to known-good states.
2. **Release notes**: Draft GitHub Release notes summarizing:
   - New CLI arguments or config keys.
   - Required environment variables (Gemini API key).
   - Docker workflow changes.
3. **Binary artifacts (optional)**: Attach pre-built binaries for Linux/macOS/Windows using `cargo build --release`. Complex systems without a Rust toolchain can download these instead of compiling from crates.io.

---

## 9. Integrate With Larger Systems

To make the CLI useful inside orchestrators or CI/CD pipelines:

1. **Create automation scripts**: Provide a `scripts/install-mawaku.sh` that runs `cargo install mawaku --version <X>`. Pinning versions prevents breakage.
2. **Expose structured output**: Ensure the CLI writes metadata (e.g., JSON summary of generated images) to stdout or a file so higher-level systems can parse results.
3. **Document environment variables**: Clarify how to set `GEMINI_API_KEY`, `MAWAKU_OUTPUT_DIR`, or any future knobs. Complex systems rely on deterministic configuration.
4. **Add smoke tests**: In CI, call the installed binary with `--help` and a sample render to ensure packaging hasn’t regressed.
5. **Version compatibility table**: Maintain a matrix mapping CLI versions to workspace schemas/config keys so services know when to upgrade.

---

## 10. Maintenance Checklist

- [ ] Re-run `cargo fmt`, `cargo clippy`, and `cargo test` before every release.
- [ ] Update README screenshots if prompts or rendering quality changes.
- [ ] Rotate crates.io tokens periodically and remove unused tokens.
- [ ] Audit dependencies for security advisories via `cargo audit`.
- [ ] Keep Dockerfile in sync with CLI releases so container users receive the latest features.
- [ ] Document breaking changes prominently and bump the major version when necessary.

Following this checklist ensures the Mawaku CLI remains easy to install, dependable for complex automation, and transparent for future contributors.
