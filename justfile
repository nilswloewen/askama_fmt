clean:
    cargo clippy --all-targets
    cargo fmt
    cargo doc --no-deps
