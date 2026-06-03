set dotenv-load := true

clean:
    cargo clippy --all-targets
    cargo fmt
    cargo doc --no-deps

publish-crates:
    cargo publish

publish-jetbrains:
    cd plugin && \
        PRIVATE_KEY="$(cat ../creds/private.pem)" \
        CERTIFICATE_CHAIN="$(cat ../creds/chain.crt)" \
        ./gradlew publishPlugin
