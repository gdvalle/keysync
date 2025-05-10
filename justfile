export TARGETS := "aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu"
export VERSION := "v0.0.1"

lint:
    cargo fmt
    cargo clippy

build:
    #!/usr/bin/env bash
    set -euo pipefail

    for target in $TARGETS; do
        rustup target add "$target"
        cross build --target "$target" --release
    done

release: build
    #!/usr/bin/env bash
    set -euo pipefail
    temp_dir=$(mktemp -d)
    for target in $TARGETS; do
        cp "target/$target/release/keysync" "$temp_dir/keysync-${target}"
    done
    gh release create "${VERSION}-$(date +%s)" "$temp_dir/"*

