# Mawaku CLI

Mawaku is a command-line interface that will grow into a toolkit for generating video call backgrounds from natural language prompts.

## Getting Started

From the `mawaku-rs/` workspace root, run:

```bash
cargo run -p mawaku -- --help
```

By default the CLI prints the active background prompt. Pass `--prompt "Describe your scene"` to override the configured value for a single run.

## Configuration

Mawaku persists its defaults in a user-level config file (created on first run) via the `mawaku-config` crate. The file lives at `~/.mawaku/config.toml`, ensuring the CLI keeps its settings directly under your home directory across operating systems. Whenever the CLI has no `--prompt` flag, it falls back to the `default_prompt` stored in that file and prints it to stdout.

To reset to the built-in default, delete the config file and rerun the CLI; a fresh file will be created with the stock prompt.

Future versions will connect to image generation providers (Google, OpenAI, etc.) based on the description you provide.

## Development

Rust 1.76+ is recommended. Install the toolchain with [rustup](https://rustup.rs/) and use `cargo check` while building new features.

The repository is organized as a Rust workspace:

- `mawaku-rs/mawaku-cli`: the Clap-based binary crate.
- `mawaku-rs/mawaku-config`: shared configuration utilities for locating and seeding the config file.

### Devcontainer usage

The repository ships with a VS Code / Dev Containers setup under `.devcontainer/`. To work inside it:

1. Install Docker and the VS Code Dev Containers extension (or `devcontainer` CLI).
2. Open the repo in VS Code and run **Dev Containers: Reopen in Container**, or execute `devcontainer up --workspace-folder .` from your terminal.
3. The container automatically provisions the Rust toolchain, so you can run `cargo check`, `cargo test`, and other project commands immediately.
4. When you exit the container, your project files remain in the host workspace; rerun step 2 to reattach later.
