# End-to-End Tutorial: Publishing the Mawaku CLI to crates.io

This walkthrough is written as a narrative tutorial that explains **what** to do and **why each step matters**. Follow it when preparing an official release of the Mawaku CLI (`mawaku-rs/mawaku-cli`) so the resulting crate can be installed with `cargo install mawaku`.

> **Scope:** Commands assume you are inside the repository root (`/workspace/mawaku-cli`). Paths are shown relative to this folder.

---

## 1. Know the Moving Pieces

Before touching Cargo, make sure you understand how the workspace is assembled:

- `mawaku-rs/Cargo.toml` declares a Cargo workspace that owns every internal crate (CLI, config helpers, utils, render backends, etc.).
- The **published crate** lives at `mawaku-rs/mawaku-cli` and exposes the binary target named `mawaku`.
- Supporting crates such as `mawaku-config`, `mawaku-utils`, `mawaku-image`, and `mawaku-gemini` either ship inside the workspace package (if `publish = false`) or must be published separately so crates.io can resolve them.

Having this mental map clarifies why you must coordinate versions across multiple `Cargo.toml` files instead of treating the CLI in isolation.

---

## 2. Gather Prerequisites

1. **Rust toolchain** – Install the latest stable toolchain (1.76+). Use `rustup update stable` to stay current. Cargo must match or exceed the minimum supported Rust version declared in the workspace.
2. **crates.io account** – Sign in with GitHub at <https://crates.io/> and verify your email. Publishing is tied to this identity.
3. **API token** – Visit *Account → API Tokens*, generate a token, and copy it to a secure password manager. This token authenticates `cargo publish`.
4. **GitHub/Git tagging permission** – Releases should be tagged (e.g., `v0.2.0`). Ensure you have push access to the canonical repository so tags can be created after publishing.
5. **Gemini API key (optional)** – The CLI orchestrates Gemini for imagery. Having a valid key lets you run end-to-end tests before release.

> ⚠️ **Never** store the crates.io or Gemini credentials inside the repo. Use environment variables or the `~/.cargo/credentials` file managed by Cargo.

---

## 3. Authenticate Cargo Once

Run `cargo login <TOKEN>` (using the token from step 2) on the machine that will publish. Cargo writes encrypted credentials to `~/.cargo/credentials`, so you do **not** need to export environment variables every release. You can confirm authentication with `cargo publish --dry-run -p mawaku`; if Cargo needs credentials it will prompt you.

To revoke access later, delete the token from crates.io and remove the entry from `~/.cargo/credentials`.

---

## 4. Align Versions and Metadata

1. **Bump the CLI version** – Edit `mawaku-rs/mawaku-cli/Cargo.toml` and change the `version` field to the new SemVer (e.g., `0.1.0`).
2. **Update dependent crates** – Any local dependencies that will be published must receive matching version bumps so `mawaku-cli` can depend on them with exact versions.
3. **Review metadata** – Confirm that `description`, `license`, `repository`, `readme`, and `keywords` fields are set. These appear on crates.io and power discoverability.
4. **Lock dependency graph** – From `mawaku-rs/`, run `cargo update` to refresh `Cargo.lock`. Commit the lockfile so the published crate references tested dependency versions.
5. **Changelog + README** – Document the release in `CHANGELOG.md` (if present) and ensure `README.md` reflects new features, flags, or config keys. crates.io will render this README.

Why it matters: crates.io treats each version as immutable. Metadata mistakes (typos, wrong license, incorrect README links) require publishing a brand-new version, so catching them now saves churn.

---

## 5. Run Quality Gates

Execute the full suite from `mawaku-rs/` to prove the workspace builds:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all --all-features
```

- `cargo fmt` keeps diffs clean and prevents reviewer noise.
- `cargo clippy` enforces lints that frequently catch missing error handling or unused fields before they leak into a release.
- `cargo test` guarantees functional correctness and ensures integration tests in `mawaku-rs/tests/` still pass.

Optionally, run a manual smoke test to validate configuration file creation and Gemini integration:

```bash
cargo run -p mawaku -- --location "Lisbon, Portugal" --season spring --time-of-day dusk
```

If the command renders images and writes `~/.mawaku/config.toml`, you know the published binary will behave the same for end users.

---

## 6. Stage the Package Locally

Before touching crates.io, build the exact `.crate` archive that will be uploaded:

```bash
cd mawaku-rs
cargo package -p mawaku
```

Cargo creates `target/package/mawaku-<VERSION>.crate`. Inspect its contents with `tar -tf` to confirm that only the expected source files, README, and license are present. Large binaries, temporary artifacts, or secrets should **not** appear. Use `.gitignore`, `.cargo_vcs_info.json`, or the `exclude` directive in `Cargo.toml` to keep the archive lean (<10 MB limit).

Running `cargo package` also verifies that build scripts succeed and that there are no missing files referenced by `build.rs` or the manifest.

---

## 7. Perform a Dry Run Publish

A dry run exercises the entire publishing pipeline (version checks, dependency resolution, manifest validation) without uploading anything:

```bash
cd mawaku-rs
cargo publish -p mawaku --dry-run
```

Common issues surfaced here include:

- **Out-of-date dependencies** – Cargo refuses to publish if referenced versions do not exist on crates.io.
- **Uncommitted changes** – The command warns if the working tree is dirty so you can commit before release.
- **Missing README or license** – crates.io requires both; the dry run ensures they are packaged.

Fix any warnings/errors and rerun the dry run until it succeeds.

---

## 8. Publish for Real

When the dry run passes and reviewers sign off:

```bash
cd mawaku-rs
cargo publish -p mawaku
```

Cargo rebuilds the package, signs it using the credentials stored earlier, and uploads it to crates.io. Publishing is permanent, so double-check the version and changelog before executing this command.

The upload typically appears at <https://crates.io/crates/mawaku> within a couple of minutes. Refresh the page to confirm the README renders correctly and the dependency tree looks sane.

---

## 9. Verify Installation

To ensure end users can actually install the new version, run:

```bash
cargo install mawaku --version <VERSION>
mawaku --help
```

Executing these commands in a clean environment (e.g., a Docker container or CI job) provides confidence that there are no hidden build-time assumptions tied to your development machine.

---

## 10. Tag and Communicate the Release

1. **Git tag**
   ```bash
   git tag -a v<VERSION> -m "Mawaku CLI v<VERSION>"
   git push origin v<VERSION>
   ```
   Tags let downstream automation pin to specific releases.
2. **GitHub Release notes** – Summarize key features, configuration changes, and any migration steps.
3. **Update docs** – If `README.md`, `docs/`, or external docs changed, ensure links point to the new version.
4. **Notify teams** – Post in Slack/Teams/email so integrators know a new CLI is available.

---

## 11. Ongoing Maintenance Checklist

- [ ] Rotate crates.io API tokens periodically and remove unused ones.
- [ ] Schedule security scans with `cargo audit` to catch vulnerable dependencies before the next release.
- [ ] Keep Docker images (`mawaku-rs/Dockerfile`) aligned with the latest crate so container users match crates.io behavior.
- [ ] Monitor crates.io download stats and user feedback to prioritize follow-up fixes.

Following this tutorial ensures every Mawaku CLI release is reproducible, well-documented, and trustworthy for the automation stacks that depend on it.
