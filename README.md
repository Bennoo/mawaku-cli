Example Morning Prompt
======================

To generate a morning scene for an Italian coastal village, run:

```bash
cargo run -p mawaku -- --location "Italian coastal village" --season summer --time-of-day morning
```

When executed with those parameters, the CLI produced the following background candidates:

![Italian coastal village morning render 1](docs/examples/mawaku-italian-co-summer-morning-p1-TH6RX.png)
![Italian coastal village morning render 2](docs/examples/mawaku-italian-co-summer-morning-p2-XOKB0.png)

Example Night Prompt
=====================

For a cozy night scene overlooking Santorini, run:

```bash
cargo run -p mawaku -- --location "Santorini, Greece" --season summer --time-of-day night
```

The CLI prints the composed prompt and saved these nighttime background candidates:

![Santorini night render 1](docs/examples/mawaku-santorini-summer-night-p1-V3DAZ.png)
![Santorini night render 2](docs/examples/mawaku-santorini-summer-night-p2-0S7LH.png)

# Mawaku CLI

Mawaku is a command-line interface that will grow into a toolkit for generating video call backgrounds from natural language prompts.

## Getting Started

From the `mawaku-rs/` workspace root, run:

```bash
cargo run -p mawaku -- --help
```

By default the CLI prints the background prompt composed from your inputs. Provide the required location plus optional season and time of day, for example:

```bash
cargo run -p mawaku -- --location "Lisbon, Portugal" --season spring --time-of-day dusk
```

To persist your Gemini API credential, run:

```bash
cargo run -p mawaku -- --set-gemini-api-key "your-secret"
```

The CLI will warn on startup if the `GEMINI_API_KEY` remains empty.

## Configuration

Mawaku persists its defaults in a user-level config file (created on first run) via the `mawaku-config` crate. The file lives at `~/.mawaku/config.toml`, ensuring the CLI keeps its settings directly under your home directory across operating systems. The same file now stores an optional `gemini_api_key` entry so that the CLI can connect to Gemini without prompting for the credential every run.

To reset to the built-in default, delete the config file and rerun the CLI; a fresh file will be created with the stock prompt.

## Development

Rust 1.76+ is recommended. Install the toolchain with [rustup](https://rustup.rs/) and use `cargo check` while building new features.

### Testing

Run the full workspace test suite from the `mawaku-rs/` directory:

```bash
cargo test
```

To target the CLI crate specifically (including its unit tests), run:

```bash
cargo test -p mawaku
```

Append `-- --nocapture` if you want to see the CLI output during test runs.

### Devcontainer usage

The repository ships with a VS Code / Dev Containers setup under `.devcontainer/`. To work inside it:

1. Install Docker and the VS Code Dev Containers extension (or `devcontainer` CLI).
2. Open the repo in VS Code and run **Dev Containers: Reopen in Container**, or execute `devcontainer up --workspace-folder .` from your terminal.
3. The container automatically provisions the Rust toolchain, so you can run `cargo check`, `cargo test`, and other project commands immediately.
4. When you exit the container, your project files remain in the host workspace; rerun step 2 to reattach later.
