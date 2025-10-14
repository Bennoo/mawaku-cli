# Mawaku CLI

Mawaku is a command-line interface that will grow into a toolkit for generating video call backgrounds from natural language prompts.

## Getting Started

```
cargo run -- --help
```

The current scaffold only provides usage information. Future versions will connect to image generation providers (Google, OpenAI, etc.) based on the description you provide.

## Development

Rust 1.76+ is recommended. Install the toolchain with [rustup](https://rustup.rs/) and use `cargo check` while building new features.

### Devcontainer usage

The repository ships with a VS Code / Dev Containers setup under `.devcontainer/`. To work inside it:

1. Install Docker and the VS Code Dev Containers extension (or `devcontainer` CLI).
2. Open the repo in VS Code and run **Dev Containers: Reopen in Container**, or execute `devcontainer up --workspace-folder .` from your terminal.
3. The container automatically provisions the Rust toolchain, so you can run `cargo check`, `cargo test`, and other project commands immediately.
4. When you exit the container, your project files remain in the host workspace; rerun step 2 to reattach later.
