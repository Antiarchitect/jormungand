#!/bin/sh

cd "$(dirname "${0}")" || exit 1

rustup self update 2>/dev/null || true \
    && rustup update stable \
    && rustup component add clippy \
    && rustup component add rustfmt \
    && cargo update \
    && cargo fmt \
    && cargo clippy \
    && cargo build \
    && cargo install --locked cargo-audit \
    && cargo audit || true \
    && cargo install --locked cargo-pants \
    && cargo pants || true \
    && cargo install --locked cargo-deny \
    && cargo deny check --hide-inclusion-graph || true \
    && cargo install --locked cargo-outdated \
    && cargo outdated || true \
    && cargo install --locked cargo-udeps \
    && cargo +nightly udeps --all-targets || true
