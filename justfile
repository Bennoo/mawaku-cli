set positional-arguments := true

CLI_TOML := "mawaku-rs/mawaku-cli/Cargo.toml"

default:
    @just --list

# Run the CLI from source with local changes, e.g. `just run --location "Lisbon, Portugal"` or `just run setup`.
run *args:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{justfile_directory()}}/mawaku-rs"
    cargo run -p mawaku -- "$@"

# Bump mawaku's version, commit Cargo.toml, and tag. Does not push. Version must start with 'v', e.g. `just bump v1.2.0`
bump version:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{justfile_directory()}}"

    tag="{{version}}"

    if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      echo "error: version must start with 'v', e.g. v0.6.0 (got '{{version}}')" >&2
      exit 1
    fi

    ver="${tag#v}"

    current=$(grep -m1 -E '^version = "[0-9]+\.[0-9]+\.[0-9]+"' {{CLI_TOML}} | sed -E 's/^version = "(.*)"/\1/')
    IFS='.' read -r cur_major cur_minor cur_patch <<< "$current"
    IFS='.' read -r new_major new_minor new_patch <<< "$ver"

    if (( new_major < cur_major )) || \
       (( new_major == cur_major && new_minor < cur_minor )) || \
       (( new_major == cur_major && new_minor == cur_minor && new_patch <= cur_patch )); then
      echo "error: $ver is not greater than current version $current" >&2
      exit 1
    fi

    if [[ -n "$(git status --porcelain)" ]]; then
      echo "error: working tree is not clean, commit or stash first" >&2
      exit 1
    fi

    if git rev-parse "$tag" >/dev/null 2>&1; then
      echo "error: tag $tag already exists" >&2
      exit 1
    fi

    sed -i.bak -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$ver\"/" {{CLI_TOML}}
    rm {{CLI_TOML}}.bak

    (cd mawaku-rs && cargo check -p mawaku)

    git add {{CLI_TOML}} mawaku-rs/Cargo.lock
    git commit -m "Bump mawaku version to $ver"
    git tag -a "$tag" -m "mawaku $tag"

    echo "Bumped to $ver, committed, and tagged $tag locally."
    echo "Run 'just push $tag' when ready to release."

# Push main and the tag to origin, triggering the release workflow. Asks for confirmation first.
push tag:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{justfile_directory()}}"

    if [[ ! "{{tag}}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      echo "error: tag must start with 'v', e.g. v0.6.0 (got '{{tag}}')" >&2
      exit 1
    fi

    if ! git rev-parse "{{tag}}" >/dev/null 2>&1; then
      echo "error: tag {{tag}} does not exist locally, run 'just bump {{tag}}' first" >&2
      exit 1
    fi

    echo "About to push origin/main and tag {{tag}}."
    echo "This triggers .github/workflows/release.yml: cross-platform builds,"
    echo "a GitHub Release, and a Formula update pushed to Bennoo/homebrew-tap."
    read -r -p "Continue? [y/N] " reply
    if [[ "$reply" =~ ^[Yy]$ ]]; then
      git push origin main
      git push origin "{{tag}}"
    else
      echo "Aborted. Nothing pushed."
    fi
