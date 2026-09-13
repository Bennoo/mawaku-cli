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

# Bump mawaku's version, commit Cargo.toml, and tag. Does not push. e.g. `just bump v1.2.0`
bump version:
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{justfile_directory()}}"

    tag="{{version}}"
    ver="${tag#v}"

    if [[ ! "$ver" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      echo "error: version must be vX.Y.Z or X.Y.Z (got '{{version}}')" >&2
      exit 1
    fi

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
    echo "Push manually when ready:"
    echo "  git push origin main"
    echo "  git push origin $tag"
