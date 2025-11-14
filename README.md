# Mawaku CLI ✨

Craft richly lit video-call backdrops from a single prompt. **Mawaku** (間 *ma* “space, pause” + 枠 *waku* “frame”) is a Rust-powered command-line companion that turns location, season, and time-of-day hints into AI-generated interiors.

---

## Table of Contents
- [Quickstart](#quickstart)
- [Spotlight Prompts](#spotlight-prompts)
- [Configuration](#configuration)
- [Docker Workflow](#docker-workflow)
- [Development](#development)

---

## Quickstart

> 🧰 **Prerequisite:** Rust 1.76+ with [`rustup`](https://rustup.rs/) installed.

1. **Install dependencies and view the CLI help**

   ```bash
   cd mawaku-rs
   cargo run -p mawaku -- --help
   ```

2. **Generate a prompt (location is required)**

   ```bash
   cargo run -p mawaku -- \
     --location "Lisbon, Portugal" \
     --season spring \
     --time-of-day dusk
   ```

3. **Export your Gemini API key once**

   ```bash
   export GEMINI_API_KEY="your-secret"
   ```

   Mawaku reads this variable each time it runs (and warns loudly if it is absent), so you never have to edit the config with raw secrets.

---

## Precompiled Linux binary

Prefer not to install Rust? Every GitHub Release now includes a `mawaku-linux-x86_64.tar.gz` archive built by CI. Download the asset from the project’s **Releases** page, extract it, and run the binary directly:

```bash
# Replace <TAG> with the published release tag and OWNER/REPO with this project path.
curl -L -o mawaku-linux-x86_64.tar.gz \
  https://github.com/OWNER/REPO/releases/download/<TAG>/mawaku-linux-x86_64.tar.gz
tar -xzf mawaku-linux-x86_64.tar.gz
chmod +x mawaku
./mawaku --help
```

Move the extracted `mawaku` binary anywhere on your `PATH` (e.g., `/usr/local/bin`) to call it from any directory.

---

## Spotlight Prompts

Get inspired by a few curated scenes. Each command runs from the `mawaku-rs/` workspace root.

<table>
  <tr>
    <th>Scene</th>
    <th>Command</th>
    <th>Mood</th>
  </tr>
  <tr>
    <td><strong>Italian Coastal Morning</strong></td>
    <td><code>cargo run -p mawaku -- --location "Italian coastal village" --season summer --time-of-day morning</code></td>
    <td>Sunlit cliffside homes and warm café interiors for breezy AM energy.</td>
  </tr>
  <tr>
    <td colspan="3" align="center">
      <img src="docs/examples/mawaku-italian-co-summer-morning-p1-TH6RX.png" alt="Italian coastal morning render 1" width="240" />
      <img src="docs/examples/mawaku-italian-co-summer-morning-p2-XOKB0.png" alt="Italian coastal morning render 2" width="240" />
    </td>
  </tr>
  <tr>
    <td><strong>Santorini Nightscape</strong></td>
    <td><code>cargo run -p mawaku -- --location "Santorini, Greece" --season summer --time-of-day night</code></td>
    <td>Lantern-lit terraces with caldera views for dramatic twilight calls.</td>
  </tr>
  <tr>
    <td colspan="3" align="center">
      <img src="docs/examples/mawaku-santorini-summer-night-p1-V3DAZ.png" alt="Santorini night render 1" width="240" />
      <img src="docs/examples/mawaku-santorini-summer-night-p2-0S7LH.png" alt="Santorini night render 2" width="240" />
    </td>
  </tr>
  <tr>
    <td><strong>Zermatt Midnight Chalet</strong></td>
    <td><code>cargo run -p mawaku -- --location "Zermatt alpine village, Switzerland" --season winter --time-of-day midnight</code></td>
    <td>Snow-dusted timber chalets with Matterhorn silhouettes and ember glow.</td>
  </tr>
  <tr>
    <td colspan="3" align="center">
      <img src="docs/examples/mawaku-zermatt-al-winter-midnight-p1-1H3PX.png" alt="Zermatt winter midnight render 1" width="240" />
      <img src="docs/examples/mawaku-zermatt-al-winter-midnight-p2-CHDG7.png" alt="Zermatt winter midnight render 2" width="240" />
    </td>
  </tr>
  <tr>
    <td><strong>El Nido Monsoon Sunrise</strong></td>
    <td><code>cargo run -p mawaku -- --location "El Nido lagoon, Palawan, Philippines" --season monsoon --time-of-day sunrise</code></td>
    <td>Tropical loft wrapped in sunrise haze and karst cliffs after summer rain.</td>
  </tr>
  <tr>
    <td colspan="3" align="center">
      <img src="docs/examples/mawaku-el-nido-la-monsoon-sunrise-p1-GF4DI.png" alt="El Nido monsoon sunrise render 1" width="240" />
      <img src="docs/examples/mawaku-el-nido-la-monsoon-sunrise-p2-GRYDJ.png" alt="El Nido monsoon sunrise render 2" width="240" />
    </td>
  </tr>
</table>

---

## Configuration

Mawaku writes persistent settings to `~/.mawaku/config.toml` the first time you run the CLI. Key entries include:

| Key / Section       | Purpose                                                                                      |
| ------------------- | -------------------------------------------------------------------------------------------- |
| `prompt`            | Baseline template the CLI enriches with your inputs.                                         |
| `[gemini_api]`      | Tracks the environment variable that stores the Gemini API key.                               |
| `image_output_dir`  | Directory (inside or outside Docker) for rendered assets.                                    |

> **Gemini credentials**
>
> Mawaku never writes the Gemini API key to disk. Instead, `[gemini_api]` keeps a single entry: `api_key_env_var`. It defaults to `GEMINI_API_KEY`, but you can edit the config file to point to any environment variable name you prefer (for example, `GEMINI_KEY`). Make sure that variable is exported before invoking the CLI.

> **Image output directory**
>
> `image_output_dir` remains at the root of the file for backward compatibility: older Mawaku releases only understood this top-level key, so keeping it there avoids breaking existing configs while still letting you edit the path manually.

To revert to defaults, delete the file and re-run any Mawaku command; a fresh template is generated automatically.

---

## Docker Workflow

Run Mawaku inside an isolated container while keeping prompts, credentials, and images on the host:

1. **Build the image**

   ```bash
   docker build -f mawaku-rs/Dockerfile -t mawaku-cli mawaku-rs
   ```

2. **Prime configuration and credentials**

   ```bash
   mkdir -p .mawaku-config
   docker run --rm \
     -e GEMINI_API_KEY="YOUR_GEMINI_KEY" \
     -v "$(pwd)/.mawaku-config:/root/.mawaku" \
     mawaku-cli \
     --location "Lisbon, Portugal"
   ```

3. **Choose an output directory**

   ```bash
   mkdir -p outputs
   # Ensure .mawaku-config/config.toml contains:
   # [gemini_api]
   # api_key_env_var = "GEMINI_API_KEY"
   # image_output_dir = "/workspace/outputs"
   ```

4. **Render a scene**

   ```bash
   docker run --rm \
     -v "$(pwd)/.mawaku-config:/root/.mawaku" \
     -v "$(pwd)/outputs:/workspace/outputs" \
     mawaku-cli \
     --location "Hakone, Japan" \
     --season spring \
     --time-of-day dusk
   ```

Resulting PNGs appear in `./outputs`, while Mawaku keeps credentials under `./.mawaku-config` for future runs.

---

## Development

### Local toolchain

- Install Rust 1.76+ via `rustup`.
- Use `cargo check` to iterate quickly and catch type errors early.

### Testing

```bash
cargo test           # full workspace
cargo test -p mawaku # CLI crate only
```

Append `-- --nocapture` to either command to see CLI stdout during tests.

### Dev Container

The repo includes `.devcontainer/` for a ready-to-code environment:

1. Install Docker and the VS Code **Dev Containers** extension (or the `devcontainer` CLI).
2. Reopen the project in the container (`Dev Containers: Reopen in Container` or `devcontainer up --workspace-folder .`).
3. The container ships with the Rust toolchain, so you can immediately run `cargo check`, `cargo test`, or `cargo run` without extra setup.
4. Exit the container any time—your project files stay on the host machine.
