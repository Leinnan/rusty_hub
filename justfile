# Rusty Hub task runner. `just` lists the recipes.

default:
    @just --list

# Run the app (debug)
run:
    cargo run

# Run an optimized build
run-release:
    cargo run --release

# Format check, tests, clippy, and type checks for the other desktop targets
validate:
    cargo fmt --all -- --check
    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo check --workspace --target x86_64-pc-windows-msvc
    cargo check --workspace --target x86_64-unknown-linux-gnu

# Re-record kittest snapshot images
snapshots:
    UPDATE_SNAPSHOTS=1 cargo test --package rusty_hub
